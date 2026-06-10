use std::ops::Range;
use std::path::Path;
use std::sync::OnceLock;

use ignore::WalkBuilder;
use pulldown_cmark::{Event, Options, Parser, Tag};
use regex::Regex;
use serde::{Deserialize, Serialize};
use unicase::UniCase;
use unicode_normalization::UnicodeNormalization;

// Matches ![[...]] and [[...]] with optional #heading and |display.
// Groups: (1) embed "!" or "", (2) target, (3) heading?, (4) display?
static WIKILINK_RE: OnceLock<Regex> = OnceLock::new();
fn wikilink_re() -> &'static Regex {
    WIKILINK_RE.get_or_init(|| {
        Regex::new(r"(!?)\[\[([^\[\]|#\n]+?)(?:#([^\[\]|#\n]+?))?(?:\|([^\[\]\n]+?))?\]\]").unwrap()
    })
}

static HTML_COMMENT_RE: OnceLock<Regex> = OnceLock::new();
fn html_comment_re() -> &'static Regex {
    HTML_COMMENT_RE.get_or_init(|| Regex::new(r"(?s)<!--.*?-->").unwrap())
}

/// A single wikilink occurrence in a document.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WikilinkRef {
    /// Target note name (no extension, no heading, no display).
    pub target: String,
    /// Optional heading anchor (`[[A#heading]]`).
    pub heading: Option<String>,
    /// Optional display text (`[[A|display]]`).
    pub display: Option<String>,
    /// `true` for embed links (`![[A]]`).
    pub embed: bool,
    /// 1-indexed line number in the source document.
    pub line: usize,
    /// 1-indexed byte column in the source line.
    pub column: usize,
}

/// A file in the vault that contains backlinks to a target note.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacklinkSource {
    pub path: String,
    pub wikilink_count: usize,
    pub links: Vec<WikilinkRef>,
}

/// Extract all wikilinks from `content`, excluding those inside:
/// - Fenced/indented code blocks (detected via pulldown-cmark offsets)
/// - HTML comments (`<!-- ... -->`)
/// - YAML frontmatter (`---` block at the start of the document)
pub fn extract_wikilinks(content: &str) -> Vec<WikilinkRef> {
    let excluded = excluded_ranges(content);
    let re = wikilink_re();
    let mut result = Vec::new();

    for cap in re.captures_iter(content) {
        let m = cap.get(0).unwrap();
        let start = m.start();

        if excluded.iter().any(|r| r.contains(&start)) {
            continue;
        }

        let embed = cap.get(1).map_or("", |m| m.as_str()) == "!";
        let target = cap.get(2).map_or("", |m| m.as_str()).trim().to_owned();
        let heading = cap.get(3).map(|m| m.as_str().trim().to_owned());
        let display = cap.get(4).map(|m| m.as_str().to_owned());

        let before = &content[..start];
        let line = before.bytes().filter(|&b| b == b'\n').count() + 1;
        let last_newline = before.rfind('\n').map(|i| i + 1).unwrap_or(0);
        let column = start - last_newline + 1;

        result.push(WikilinkRef {
            target,
            heading,
            display,
            embed,
            line,
            column,
        });
    }

    result
}

/// Return all vault files that contain at least one wikilink whose target
/// resolves to `target_stem` (case-insensitive, NFC-normalized).
pub fn build_backlinks(vault_root: &Path, target_stem: &str) -> crate::Result<Vec<BacklinkSource>> {
    let want = normalize_name(target_stem);
    let mut sources = Vec::new();

    for entry in WalkBuilder::new(vault_root).build().flatten() {
        if !entry.file_type().is_some_and(|t| t.is_file()) {
            continue;
        }
        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext != "md" && ext != "markdown" {
            continue;
        }

        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let matching: Vec<WikilinkRef> = extract_wikilinks(&content)
            .into_iter()
            .filter(|l| normalize_name(&l.target) == want)
            .collect();

        if !matching.is_empty() {
            let rel = path
                .strip_prefix(vault_root)
                .map(|r| r.to_string_lossy().replace('\\', "/"))
                .unwrap_or_default()
                .to_owned();
            sources.push(BacklinkSource {
                path: rel,
                wikilink_count: matching.len(),
                links: matching,
            });
        }
    }

    Ok(sources)
}

/// Normalize a wikilink target for resolution: NFC then wrap in UniCase.
pub fn normalize_name(name: &str) -> UniCase<String> {
    let nfc: String = name.trim().nfc().collect();
    UniCase::new(nfc)
}

/// Resolve a wikilink target to a vault-relative file path.
/// Returns the first `.md` file whose stem matches `target`
/// (NFC-normalized, case-insensitive).
pub fn resolve_link_to_path(vault_root: &Path, target: &str) -> Option<String> {
    let want = normalize_name(target);
    for entry in WalkBuilder::new(vault_root).build().flatten() {
        if !entry.file_type().is_some_and(|t| t.is_file()) {
            continue;
        }
        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext != "md" && ext != "markdown" {
            continue;
        }
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str())
            && normalize_name(stem) == want
        {
            return path
                .strip_prefix(vault_root)
                .ok()
                .map(|r| r.to_string_lossy().replace('\\', "/"));
        }
    }
    None
}

/// Replace all wikilinks targeting `old_stem` with `new_stem` in `content`,
/// respecting the same exclusion rules as `extract_wikilinks`.
/// Heading anchors and display text are preserved verbatim.
pub fn replace_wikilinks(content: &str, old_stem: &str, new_stem: &str) -> String {
    let excluded = excluded_ranges(content);
    let want = normalize_name(old_stem);
    let re = wikilink_re();

    let mut replacements: Vec<(Range<usize>, String)> = Vec::new();

    for cap in re.captures_iter(content) {
        let m = cap.get(0).unwrap();
        let start = m.start();

        if excluded.iter().any(|r| r.contains(&start)) {
            continue;
        }

        let target = cap.get(2).map_or("", |m| m.as_str()).trim();
        if normalize_name(target) != want {
            continue;
        }

        let embed = cap.get(1).map_or("", |m| m.as_str());
        let heading = cap
            .get(3)
            .map(|m| format!("#{}", m.as_str()))
            .unwrap_or_default();
        let display = cap
            .get(4)
            .map(|m| format!("|{}", m.as_str()))
            .unwrap_or_default();
        let new_link = format!("{embed}[[{new_stem}{heading}{display}]]");

        replacements.push((m.range(), new_link));
    }

    if replacements.is_empty() {
        return content.to_owned();
    }

    // Apply in reverse order to preserve earlier byte offsets.
    let mut result = content.to_owned();
    for (range, replacement) in replacements.into_iter().rev() {
        result.replace_range(range, &replacement);
    }
    result
}

// ── Internals ────────────────────────────────────────────────────────────────

/// Compute byte ranges in `content` that should be excluded from wikilink
/// extraction: frontmatter, code blocks, HTML comments.
pub(crate) fn excluded_ranges(content: &str) -> Vec<Range<usize>> {
    let mut ranges = Vec::new();

    // 1. YAML frontmatter
    if let Some(r) = frontmatter_range(content) {
        ranges.push(r);
    }

    // 2. Code spans via pulldown-cmark — a single source of truth shared by
    //    extract / replace / backlinks:
    //    - `Start(CodeBlock)` carries the ENTIRE fenced/indented block range.
    //    - `Code` is an inline code span (`` `[[A]]` ``); its range covers the
    //      span including the backticks.
    let opts = Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_GFM;
    for (event, range) in Parser::new_ext(content, opts).into_offset_iter() {
        if matches!(event, Event::Start(Tag::CodeBlock(_)) | Event::Code(_)) {
            ranges.push(range);
        }
    }

    // 3. HTML comments (`<!-- ... -->`, possibly multi-line)
    for m in html_comment_re().find_iter(content) {
        ranges.push(m.range());
    }

    ranges
}

/// Return the byte range of the YAML frontmatter block if present.
/// Frontmatter starts with `---` on the first line and ends at the next
/// `---` or `...` line.
fn frontmatter_range(content: &str) -> Option<Range<usize>> {
    let first_newline = content.find('\n')?;
    let first_line = content[..first_newline].trim_end_matches('\r');
    if first_line != "---" {
        return None;
    }

    let mut pos = first_newline + 1;
    loop {
        if pos >= content.len() {
            return None; // Unclosed frontmatter — don't exclude
        }
        let next_newline = content[pos..]
            .find('\n')
            .map(|i| pos + i)
            .unwrap_or(content.len());
        let line = content[pos..next_newline].trim_end_matches('\r');
        let line_end = if next_newline < content.len() {
            next_newline + 1
        } else {
            next_newline
        };
        if line == "---" || line == "..." {
            return Some(0..line_end);
        }
        pos = line_end;
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    // ── Exclusion rules ───────────────────────────────────────────────────────

    #[test]
    fn codefence_content_not_extracted() {
        let md = "before\n```\n[[X]] in fence\n```\nafter [[Y]]\n";
        let links = extract_wikilinks(md);
        assert!(
            links.iter().all(|l| l.target != "X"),
            "X must be excluded: {links:?}"
        );
        assert!(
            links.iter().any(|l| l.target == "Y"),
            "Y must be present: {links:?}"
        );
    }

    #[test]
    fn indented_code_block_not_extracted() {
        // 4-space indented code block
        let md = "text\n\n    [[X]] indented\n\n[[Y]] normal\n";
        let links = extract_wikilinks(md);
        assert!(
            links.iter().all(|l| l.target != "X"),
            "X must be excluded: {links:?}"
        );
        assert!(
            links.iter().any(|l| l.target == "Y"),
            "Y must be present: {links:?}"
        );
    }

    #[test]
    fn inline_code_span_not_extracted() {
        // `[[X]]` inside an inline code span must be excluded, while a plain
        // `[[Y]]` on the same line is still extracted.
        let md = "use `[[X]]` literally but link [[Y]]\n";
        let links = extract_wikilinks(md);
        assert!(
            links.iter().all(|l| l.target != "X"),
            "X must be excluded: {links:?}"
        );
        assert!(
            links.iter().any(|l| l.target == "Y"),
            "Y must be present: {links:?}"
        );
    }

    #[test]
    fn inline_code_does_not_break_fence_or_text() {
        // Existing behaviour stays unchanged: fenced [[X]] excluded, plain
        // [[Y]] extracted, even alongside an inline code span.
        let md = "intro `code`\n```\n[[X]] in fence\n```\nafter [[Y]]\n";
        let links = extract_wikilinks(md);
        assert!(
            links.iter().all(|l| l.target != "X"),
            "X must be excluded: {links:?}"
        );
        assert!(
            links.iter().any(|l| l.target == "Y"),
            "Y must be present: {links:?}"
        );
    }

    #[test]
    fn html_comment_not_extracted() {
        let md = "before <!-- [[X]] --> after [[Y]]\n";
        let links = extract_wikilinks(md);
        assert!(
            links.iter().all(|l| l.target != "X"),
            "X must be excluded: {links:?}"
        );
        assert!(
            links.iter().any(|l| l.target == "Y"),
            "Y must be present: {links:?}"
        );
    }

    #[test]
    fn multiline_html_comment_not_extracted() {
        let md = "<!--\n[[X]] in multiline comment\n-->\n[[Y]] outside\n";
        let links = extract_wikilinks(md);
        assert!(
            links.iter().all(|l| l.target != "X"),
            "X must be excluded: {links:?}"
        );
        assert!(
            links.iter().any(|l| l.target == "Y"),
            "Y must be present: {links:?}"
        );
    }

    #[test]
    fn frontmatter_not_extracted() {
        let md = "---\ntarget: [[X]]\n---\n\n[[Y]] in body\n";
        let links = extract_wikilinks(md);
        assert!(
            links.iter().all(|l| l.target != "X"),
            "X must be excluded: {links:?}"
        );
        assert!(
            links.iter().any(|l| l.target == "Y"),
            "Y must be present: {links:?}"
        );
    }

    #[test]
    fn frontmatter_closed_by_dots() {
        let md = "---\nkey: value\n...\n\n[[Y]] in body\n";
        let links = extract_wikilinks(md);
        assert!(links.iter().any(|l| l.target == "Y"));
    }

    // ── Wikilink syntax variants ──────────────────────────────────────────────

    #[test]
    fn plain_link_parsed() {
        let links = extract_wikilinks("[[A]]\n");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target, "A");
        assert_eq!(links[0].heading, None);
        assert_eq!(links[0].display, None);
        assert!(!links[0].embed);
    }

    #[test]
    fn aliased_link_parsed() {
        let links = extract_wikilinks("[[A|display text]]\n");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target, "A");
        assert_eq!(links[0].display, Some("display text".into()));
        assert_eq!(links[0].heading, None);
    }

    #[test]
    fn heading_link_parsed() {
        let links = extract_wikilinks("[[A#section]]\n");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target, "A");
        assert_eq!(links[0].heading, Some("section".into()));
        assert_eq!(links[0].display, None);
    }

    #[test]
    fn heading_and_display_link_parsed() {
        let links = extract_wikilinks("[[A#section|label]]\n");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target, "A");
        assert_eq!(links[0].heading, Some("section".into()));
        assert_eq!(links[0].display, Some("label".into()));
    }

    #[test]
    fn embed_link_parsed() {
        let links = extract_wikilinks("![[A]]\n");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target, "A");
        assert!(links[0].embed);
    }

    #[test]
    fn forward_match_is_distinct() {
        let links = extract_wikilinks("[[A]] [[AB]]\n");
        assert_eq!(links.len(), 2);
        let targets: Vec<&str> = links.iter().map(|l| l.target.as_str()).collect();
        assert!(targets.contains(&"A"));
        assert!(targets.contains(&"AB"));
        // [[AB]] must NOT be treated as a match for [[A]]
        assert_ne!(links[0].target, links[1].target);
    }

    #[test]
    fn line_and_column_are_correct() {
        let md = "line1\nline2 [[A]]\nline3\n";
        let links = extract_wikilinks(md);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].line, 2);
        assert_eq!(links[0].column, 7); // 'l','i','n','e','2',' ' = 6 bytes, so col 7
    }

    // ── Normalisation for link resolution ─────────────────────────────────────

    #[test]
    fn case_insensitive_match() {
        assert_eq!(normalize_name("AUTH"), normalize_name("auth"));
        assert_eq!(normalize_name("Auth"), normalize_name("auth"));
    }

    #[test]
    fn nfc_nfd_match() {
        // "é" can be NFC (U+00E9) or NFD (e + U+0301)
        let nfc = "\u{00e9}"; // é precomposed
        let nfd = "e\u{0301}"; // e + combining accent
        assert_eq!(normalize_name(nfc), normalize_name(nfd));
    }

    // ── replace_wikilinks ─────────────────────────────────────────────────────

    #[test]
    fn replace_skips_inline_code_span() {
        // `[[A]]` inside an inline code span must NOT be rewritten, while a
        // plain [[A]] is renamed to [[B]].
        let md = "rename `[[A]]` but link [[A]] here\n";
        let out = replace_wikilinks(md, "A", "B");
        assert_eq!(out, "rename `[[A]]` but link [[B]] here\n");
    }

    // ── build_backlinks ───────────────────────────────────────────────────────

    fn make_vault() -> TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn backlinks_finds_referring_files() {
        let vault = make_vault();
        let root = vault.path();
        fs::write(root.join("auth.md"), "# Auth\n").unwrap();
        fs::write(root.join("index.md"), "See [[auth]] for details.\n").unwrap();
        fs::write(root.join("daily.md"), "also [[auth]] again\n").unwrap();
        fs::write(root.join("other.md"), "no link here\n").unwrap();

        let mut sources = build_backlinks(root, "auth").unwrap();
        sources.sort_by(|a, b| a.path.cmp(&b.path));

        let paths: Vec<&str> = sources.iter().map(|s| s.path.as_str()).collect();
        assert!(paths.contains(&"daily.md"), "{paths:?}");
        assert!(paths.contains(&"index.md"), "{paths:?}");
        assert!(!paths.contains(&"other.md"), "{paths:?}");
    }

    #[test]
    fn backlinks_case_insensitive() {
        let vault = make_vault();
        let root = vault.path();
        fs::write(root.join("ref.md"), "[[AUTH]] and [[Auth]]\n").unwrap();

        let sources = build_backlinks(root, "auth").unwrap();
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].wikilink_count, 2);
    }

    #[test]
    fn backlinks_excludes_code_blocks() {
        let vault = make_vault();
        let root = vault.path();
        fs::write(
            root.join("ref.md"),
            "```\n[[auth]] in code\n```\nnormal [[auth]]\n",
        )
        .unwrap();

        let sources = build_backlinks(root, "auth").unwrap();
        // Only the one outside the code block should be counted
        assert_eq!(sources[0].wikilink_count, 1);
    }

    #[test]
    fn backlinks_excludes_inline_code_spans() {
        let vault = make_vault();
        let root = vault.path();
        fs::write(
            root.join("ref.md"),
            "literal `[[auth]]` then real [[auth]]\n",
        )
        .unwrap();

        let sources = build_backlinks(root, "auth").unwrap();
        // Only the link outside the inline code span should be counted.
        assert_eq!(sources[0].wikilink_count, 1);
    }

    #[test]
    fn backlinks_no_forward_match() {
        let vault = make_vault();
        let root = vault.path();
        // [[authz]] should NOT match backlinks for "auth"
        fs::write(root.join("ref.md"), "[[authz]] and [[auth]]\n").unwrap();

        let sources = build_backlinks(root, "auth").unwrap();
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].wikilink_count, 1); // only [[auth]], not [[authz]]
    }
}
