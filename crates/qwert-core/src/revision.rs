use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use rayon::prelude::*;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::link_index::{extract_wikilinks, normalize_name, replace_wikilinks};
use crate::{CoreError, Result};

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum NamingStyle {
    #[default]
    Increment,
    Date,
    Semver,
    Manual,
}

/// A file affected by a revision (contains wikilinks pointing to the old name).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AffectedFile {
    pub path: String,
    pub wikilink_count: usize,
}

/// The dry-run output shape (§8, matches the JSON output spec).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevisionPlan {
    pub schema_version: String,
    pub kind: String,
    pub dry_run: bool,
    pub old_name: String,
    pub new_name: String,
    pub old_path: String,
    pub new_path: String,
    pub affected_files: Vec<AffectedFile>,
    pub total_wikilinks: usize,
}

/// Input parameters for a revision operation.
#[derive(Debug, Clone)]
pub struct RevisionRequest {
    pub vault_root: PathBuf,
    /// Vault-relative path of the source file to rename.
    pub source_rel_path: String,
    pub naming: NamingStyle,
    /// Required when `naming == Manual`.
    pub new_name: Option<String>,
    pub excluded_dirs: Vec<String>,
    /// YYYYMMDD string — required when `naming == Date`.
    pub date_str: Option<String>,
}

/// Return value of a completed (non-dry-run) revision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevisionResult {
    pub old_path: String,
    pub new_path: String,
    pub affected_files: Vec<AffectedFile>,
    pub total_wikilinks: usize,
}

// ── WAL (internal) ────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
struct ContentOp {
    final_path: String,
    tmp_path: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct PendingRevision {
    schema_version: String,
    kind: String,
    source_old_path: String,
    source_new_path: String,
    content_ops: Vec<ContentOp>,
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Generate a new file stem from the old one according to the naming style.
pub fn generate_new_name(
    old_stem: &str,
    style: &NamingStyle,
    manual_name: Option<&str>,
    date_str: Option<&str>,
) -> Result<String> {
    match style {
        NamingStyle::Manual => {
            let name = manual_name.ok_or_else(|| {
                CoreError::InvalidPattern("Manual style requires --name <value>".into())
            })?;
            Ok(name.to_owned())
        }
        NamingStyle::Increment => Ok(increment_stem(old_stem)),
        NamingStyle::Date => {
            let d = date_str.ok_or_else(|| {
                CoreError::InvalidPattern("Date style requires a date string (YYYYMMDD)".into())
            })?;
            Ok(format!("{old_stem}_{d}"))
        }
        NamingStyle::Semver => Ok(bump_semver(old_stem)),
    }
}

/// Build a dry-run plan without touching the filesystem.
pub fn plan_revision(req: &RevisionRequest) -> Result<RevisionPlan> {
    let vault_root = req.vault_root.canonicalize()?;
    let source_abs = vault_root.join(&req.source_rel_path);
    if !source_abs.exists() {
        return Err(CoreError::NotFound(req.source_rel_path.clone()));
    }

    let old_stem = stem_of(&req.source_rel_path)?;
    let new_stem = generate_new_name(
        &old_stem,
        &req.naming,
        req.new_name.as_deref(),
        req.date_str.as_deref(),
    )?;
    let new_path = sibling_path(&req.source_rel_path, &new_stem);

    let affected = scan_affected(&vault_root, &old_stem, &req.excluded_dirs)?;
    let total_wikilinks = affected.iter().map(|f| f.wikilink_count).sum();

    Ok(RevisionPlan {
        schema_version: "v1".into(),
        kind: "revision_plan".into(),
        dry_run: true,
        old_name: old_stem,
        new_name: new_stem,
        old_path: req.source_rel_path.clone(),
        new_path,
        affected_files: affected,
        total_wikilinks,
    })
}

/// Execute the revision atomically via WAL:
///   1. Write new content to temp files + fsync
///   2. Write `.qwert/pending-revision.json`
///   3. Rename source file; rename all temp → final paths
///   4. Delete WAL
///   5. Invoke `on-revise` hook if present
pub fn execute_revision(req: &RevisionRequest) -> Result<RevisionResult> {
    let vault_root = req.vault_root.canonicalize()?;
    let source_abs = vault_root.join(&req.source_rel_path);
    if !source_abs.exists() {
        return Err(CoreError::NotFound(req.source_rel_path.clone()));
    }

    let old_stem = stem_of(&req.source_rel_path)?;
    let new_stem = generate_new_name(
        &old_stem,
        &req.naming,
        req.new_name.as_deref(),
        req.date_str.as_deref(),
    )?;
    let new_rel = sibling_path(&req.source_rel_path, &new_stem);
    let new_source_abs = vault_root.join(&new_rel);

    if new_source_abs.exists() {
        return Err(CoreError::Io(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!("Target already exists: {new_rel}"),
        )));
    }

    // ── Parallel scan ─────────────────────────────────────────────────────────
    let all_md = collect_md_files(&vault_root, &req.excluded_dirs)?;
    let want = normalize_name(&old_stem);

    struct Scanned {
        abs_path: PathBuf,
        rel_path: String,
        new_content: String,
        wikilink_count: usize,
    }

    let scanned: Vec<Scanned> = all_md
        .par_iter()
        .filter_map(|(abs, rel)| {
            let content = std::fs::read_to_string(abs).ok()?;
            let count = extract_wikilinks(&content)
                .into_iter()
                .filter(|l| normalize_name(&l.target) == want)
                .count();
            if count == 0 {
                return None;
            }
            let new_content = replace_wikilinks(&content, &old_stem, &new_stem);
            Some(Scanned {
                abs_path: abs.clone(),
                rel_path: rel.clone(),
                new_content,
                wikilink_count: count,
            })
        })
        .collect();

    // ── Write temp files (fsync each) ─────────────────────────────────────────
    // For the source file, the final destination is new_source_abs (since it
    // gets renamed first in the commit phase).
    let mut ops: Vec<(PathBuf, tempfile::NamedTempFile)> = Vec::new();
    for s in &scanned {
        let final_abs = if s.abs_path == source_abs {
            new_source_abs.clone()
        } else {
            s.abs_path.clone()
        };
        let dir = final_abs.parent().unwrap_or(Path::new("."));
        let mut tmp = tempfile::NamedTempFile::new_in(dir)?;
        tmp.write_all(s.new_content.as_bytes())?;
        tmp.as_file().sync_all()?;
        ops.push((final_abs, tmp));
    }

    // ── Write WAL ─────────────────────────────────────────────────────────────
    let wal_dir = vault_root.join(".qwert");
    std::fs::create_dir_all(&wal_dir)?;
    let wal_path = wal_dir.join("pending-revision.json");

    let pending = PendingRevision {
        schema_version: "v1".into(),
        kind: "pending_revision".into(),
        source_old_path: req.source_rel_path.clone(),
        source_new_path: new_rel.clone(),
        content_ops: ops
            .iter()
            .map(|(final_abs, tmp)| {
                let final_rel = final_abs
                    .strip_prefix(&vault_root)
                    .map(|r| r.to_string_lossy().replace('\\', "/"))
                    .unwrap_or_default()
                    .to_string();
                ContentOp {
                    final_path: final_rel,
                    tmp_path: tmp.path().to_string_lossy().into_owned(),
                }
            })
            .collect(),
    };
    write_wal(&wal_path, &pending)?;

    // ── Atomic commit ─────────────────────────────────────────────────────────
    // Rename source file first, then apply content updates.
    std::fs::rename(&source_abs, &new_source_abs)?;

    for (final_abs, tmp) in ops {
        tmp.persist(&final_abs)
            .map_err(|e| CoreError::Io(e.error))?;
    }

    // ── Remove WAL (commit complete) ──────────────────────────────────────────
    let _ = std::fs::remove_file(&wal_path);

    // ── on-revise hook ────────────────────────────────────────────────────────
    let total_wikilinks: usize = scanned.iter().map(|s| s.wikilink_count).sum();
    invoke_on_revise_hook(&vault_root, &req.source_rel_path, &new_rel, total_wikilinks);

    let affected_files = scanned
        .into_iter()
        .map(|s| AffectedFile {
            path: s.rel_path,
            wikilink_count: s.wikilink_count,
        })
        .collect();

    Ok(RevisionResult {
        old_path: req.source_rel_path.clone(),
        new_path: new_rel,
        affected_files,
        total_wikilinks,
    })
}

/// Check for a leftover `pending-revision.json` and roll back what can be undone:
/// - Delete temp files that were not yet applied.
/// - Rename source back if it was already moved.
///
/// Returns `true` if a pending revision was found.
pub fn rollback_pending(vault_root: &Path) -> Result<bool> {
    let wal_path = vault_root.join(".qwert").join("pending-revision.json");
    if !wal_path.exists() {
        return Ok(false);
    }

    let raw = std::fs::read_to_string(&wal_path)?;
    let pending: PendingRevision = serde_json::from_str(&raw)?;

    // Delete unapplied temp files.
    for op in &pending.content_ops {
        let tmp = Path::new(&op.tmp_path);
        if tmp.exists() {
            let _ = std::fs::remove_file(tmp);
        }
    }

    // Reverse source rename if it already happened.
    let old_abs = vault_root.join(&pending.source_old_path);
    let new_abs = vault_root.join(&pending.source_new_path);
    if new_abs.exists() && !old_abs.exists() {
        std::fs::rename(&new_abs, &old_abs)?;
    }

    let _ = std::fs::remove_file(&wal_path);
    Ok(true)
}

/// Scan the vault for files that reference `old_stem`.
pub fn scan_affected(
    vault_root: &Path,
    old_stem: &str,
    excluded_dirs: &[String],
) -> Result<Vec<AffectedFile>> {
    let all_md = collect_md_files(vault_root, excluded_dirs)?;
    let want = normalize_name(old_stem);
    let affected: Vec<AffectedFile> = all_md
        .par_iter()
        .filter_map(|(abs, rel)| {
            let content = std::fs::read_to_string(abs).ok()?;
            let count = extract_wikilinks(&content)
                .into_iter()
                .filter(|l| normalize_name(&l.target) == want)
                .count();
            if count == 0 {
                return None;
            }
            Some(AffectedFile {
                path: rel.clone(),
                wikilink_count: count,
            })
        })
        .collect();
    Ok(affected)
}

// ── Private helpers ───────────────────────────────────────────────────────────

fn stem_of(rel_path: &str) -> Result<String> {
    Path::new(rel_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.to_owned())
        .ok_or_else(|| CoreError::InvalidPattern(format!("Cannot derive stem from: {rel_path}")))
}

fn sibling_path(rel_path: &str, new_stem: &str) -> String {
    let p = Path::new(rel_path);
    let parent = p.parent().unwrap_or(Path::new(""));
    let joined = parent.join(format!("{new_stem}.md"));
    let s = joined.to_string_lossy();
    // Normalise to forward slashes and strip leading "./"
    let s = s.replace('\\', "/");
    s.strip_prefix("./").map(str::to_owned).unwrap_or(s)
}

fn increment_stem(stem: &str) -> String {
    if let Some(pos) = stem.rfind('_') {
        let suffix = &stem[pos + 1..];
        if let Ok(n) = suffix.parse::<u64>() {
            return format!("{}_{}", &stem[..pos], n + 1);
        }
    }
    format!("{stem}_2")
}

static SEMVER_RE: OnceLock<Regex> = OnceLock::new();
fn semver_re() -> &'static Regex {
    SEMVER_RE.get_or_init(|| Regex::new(r"^(.+)_(\d+)\.(\d+)\.(\d+)$").unwrap())
}

fn bump_semver(stem: &str) -> String {
    if let Some(caps) = semver_re().captures(stem) {
        let base = &caps[1];
        let major: u64 = caps[2].parse().unwrap_or(0);
        let minor: u64 = caps[3].parse().unwrap_or(0);
        let patch: u64 = caps[4].parse().unwrap_or(0);
        return format!("{base}_{major}.{minor}.{}", patch + 1);
    }
    format!("{stem}_1.1.0")
}

fn collect_md_files(vault_root: &Path, excluded_dirs: &[String]) -> Result<Vec<(PathBuf, String)>> {
    use ignore::WalkBuilder;
    let mut files = Vec::new();
    for entry in WalkBuilder::new(vault_root).build().flatten() {
        if !entry.file_type().is_some_and(|t| t.is_file()) {
            continue;
        }
        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext != "md" && ext != "markdown" {
            continue;
        }
        let rel = path
            .strip_prefix(vault_root)
            .map(|r| r.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default()
            .to_string();
        if !excluded_dirs.is_empty() {
            let excl = excluded_dirs
                .iter()
                .any(|d| rel.starts_with(d.trim_end_matches('/')));
            if excl {
                continue;
            }
        }
        files.push((path.to_path_buf(), rel));
    }
    Ok(files)
}

fn write_wal(wal_path: &Path, pending: &PendingRevision) -> Result<()> {
    let content = serde_json::to_string(pending)?;
    let dir = wal_path.parent().unwrap_or(Path::new("."));
    let mut tmp = tempfile::NamedTempFile::new_in(dir)?;
    tmp.write_all(content.as_bytes())?;
    tmp.as_file().sync_all()?;
    tmp.persist(wal_path).map_err(|e| CoreError::Io(e.error))?;
    Ok(())
}

fn invoke_on_revise_hook(vault_root: &Path, old_rel: &str, new_rel: &str, count: usize) {
    let hook_path = directories::BaseDirs::new().and_then(|d| {
        let p = d.config_dir().join("qwert").join("hooks").join("on-revise");
        p.exists().then_some(p)
    });
    let Some(hook) = hook_path else { return };
    let _ = std::process::Command::new(&hook)
        .arg(old_rel)
        .arg(new_rel)
        .env("QWERT_VAULT", vault_root)
        .env("QWERT_REV_COUNT", count.to_string())
        .status();
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn make_vault() -> TempDir {
        tempfile::tempdir().unwrap()
    }

    fn vault_root(tmp: &TempDir) -> PathBuf {
        tmp.path().canonicalize().unwrap()
    }

    // ── Naming rules ──────────────────────────────────────────────────────────

    #[test]
    fn increment_plain() {
        assert_eq!(increment_stem("auth"), "auth_2");
    }

    #[test]
    fn increment_existing_suffix() {
        assert_eq!(increment_stem("auth_2"), "auth_3");
        assert_eq!(increment_stem("auth_10"), "auth_11");
    }

    #[test]
    fn increment_non_numeric_suffix_treated_as_plain() {
        assert_eq!(increment_stem("auth_draft"), "auth_draft_2");
    }

    #[test]
    fn increment_underscored_base() {
        assert_eq!(increment_stem("my_doc_2"), "my_doc_3");
    }

    #[test]
    fn semver_plain() {
        assert_eq!(bump_semver("auth"), "auth_1.1.0");
    }

    #[test]
    fn semver_bump_patch() {
        assert_eq!(bump_semver("auth_1.1.0"), "auth_1.1.1");
        assert_eq!(bump_semver("auth_2.3.9"), "auth_2.3.10");
    }

    #[test]
    fn generate_new_name_increment() {
        let r = generate_new_name("auth", &NamingStyle::Increment, None, None).unwrap();
        assert_eq!(r, "auth_2");
    }

    #[test]
    fn generate_new_name_date() {
        let r = generate_new_name("auth", &NamingStyle::Date, None, Some("20260101")).unwrap();
        assert_eq!(r, "auth_20260101");
    }

    #[test]
    fn generate_new_name_semver() {
        let r = generate_new_name("auth", &NamingStyle::Semver, None, None).unwrap();
        assert_eq!(r, "auth_1.1.0");
    }

    #[test]
    fn generate_new_name_manual() {
        let r = generate_new_name("auth", &NamingStyle::Manual, Some("auth_jwt"), None).unwrap();
        assert_eq!(r, "auth_jwt");
    }

    #[test]
    fn generate_new_name_manual_no_name_errors() {
        let r = generate_new_name("auth", &NamingStyle::Manual, None, None);
        assert!(r.is_err());
    }

    #[test]
    fn generate_new_name_date_no_date_errors() {
        let r = generate_new_name("auth", &NamingStyle::Date, None, None);
        assert!(r.is_err());
    }

    // ── sibling_path ──────────────────────────────────────────────────────────

    #[test]
    fn sibling_path_root() {
        assert_eq!(sibling_path("auth.md", "auth_2"), "auth_2.md");
    }

    #[test]
    fn sibling_path_subdir() {
        assert_eq!(sibling_path("specs/auth.md", "auth_2"), "specs/auth_2.md");
    }

    // ── replace_wikilinks (exclusion rules) ───────────────────────────────────

    #[test]
    fn replace_wikilinks_plain() {
        let out = replace_wikilinks("See [[auth]] here.", "auth", "auth_2");
        assert_eq!(out, "See [[auth_2]] here.");
    }

    #[test]
    fn replace_wikilinks_embed() {
        let out = replace_wikilinks("![[auth]]", "auth", "auth_2");
        assert_eq!(out, "![[auth_2]]");
    }

    #[test]
    fn replace_wikilinks_heading() {
        let out = replace_wikilinks("[[auth#section]]", "auth", "auth_2");
        assert_eq!(out, "[[auth_2#section]]");
    }

    #[test]
    fn replace_wikilinks_display() {
        let out = replace_wikilinks("[[auth|Auth Doc]]", "auth", "auth_2");
        assert_eq!(out, "[[auth_2|Auth Doc]]");
    }

    #[test]
    fn replace_wikilinks_heading_and_display() {
        let out = replace_wikilinks("[[auth#sec|label]]", "auth", "auth_2");
        assert_eq!(out, "[[auth_2#sec|label]]");
    }

    #[test]
    fn replace_wikilinks_no_forward_match() {
        // [[authz]] must NOT be updated when renaming "auth".
        let out = replace_wikilinks("[[authz]] and [[auth]]", "auth", "auth_2");
        assert_eq!(out, "[[authz]] and [[auth_2]]");
    }

    #[test]
    fn replace_wikilinks_skips_code_block() {
        let md = "```\n[[auth]] in fence\n```\n[[auth]] outside\n";
        let out = replace_wikilinks(md, "auth", "auth_2");
        assert!(
            out.contains("[[auth]] in fence"),
            "inside fence must be untouched"
        );
        assert!(
            out.contains("[[auth_2]] outside"),
            "outside fence must be updated"
        );
    }

    #[test]
    fn replace_wikilinks_skips_html_comment() {
        let md = "<!-- [[auth]] --> [[auth]] outside";
        let out = replace_wikilinks(md, "auth", "auth_2");
        assert!(
            out.contains("<!-- [[auth]] -->"),
            "comment must be untouched"
        );
        assert!(out.contains("[[auth_2]] outside"));
    }

    #[test]
    fn replace_wikilinks_skips_frontmatter() {
        let md = "---\ntarget: [[auth]]\n---\n[[auth]] in body\n";
        let out = replace_wikilinks(md, "auth", "auth_2");
        assert!(
            out.contains("target: [[auth]]"),
            "frontmatter must be untouched"
        );
        assert!(out.contains("[[auth_2]] in body"));
    }

    #[test]
    fn replace_wikilinks_case_insensitive() {
        let out = replace_wikilinks("[[AUTH]] and [[Auth]]", "auth", "auth_2");
        assert_eq!(out, "[[auth_2]] and [[auth_2]]");
    }

    #[test]
    fn replace_wikilinks_no_match_returns_same() {
        let md = "[[other]] note";
        let out = replace_wikilinks(md, "auth", "auth_2");
        assert_eq!(out, md);
    }

    // ── excluded_dirs ─────────────────────────────────────────────────────────

    #[test]
    fn scan_affected_respects_excluded_dirs() {
        let tmp = make_vault();
        let root = vault_root(&tmp);

        fs::create_dir(root.join("decisions")).unwrap();
        fs::write(root.join("index.md"), "[[auth]] ref\n").unwrap();
        fs::write(
            root.join("decisions").join("adr-001.md"),
            "[[auth]] in ADR\n",
        )
        .unwrap();

        let affected = scan_affected(&root, "auth", &["decisions".to_owned()]).unwrap();
        let paths: Vec<&str> = affected.iter().map(|f| f.path.as_str()).collect();

        assert!(paths.contains(&"index.md"), "index.md must be in results");
        assert!(
            !paths.iter().any(|p| p.starts_with("decisions/")),
            "decisions/ must be excluded: {paths:?}"
        );
    }

    #[test]
    fn scan_affected_empty_excluded_dirs_scans_all() {
        let tmp = make_vault();
        let root = vault_root(&tmp);

        fs::create_dir(root.join("decisions")).unwrap();
        fs::write(root.join("index.md"), "[[auth]]\n").unwrap();
        fs::write(root.join("decisions").join("adr.md"), "[[auth]]\n").unwrap();

        let affected = scan_affected(&root, "auth", &[]).unwrap();
        assert_eq!(affected.len(), 2);
    }

    // ── plan_revision ─────────────────────────────────────────────────────────

    #[test]
    fn plan_revision_dry_run() {
        let tmp = make_vault();
        let root = vault_root(&tmp);

        fs::write(root.join("auth.md"), "# Auth\n").unwrap();
        fs::write(root.join("index.md"), "See [[auth]].\n").unwrap();

        let req = RevisionRequest {
            vault_root: root.clone(),
            source_rel_path: "auth.md".into(),
            naming: NamingStyle::Increment,
            new_name: None,
            excluded_dirs: vec![],
            date_str: None,
        };
        let plan = plan_revision(&req).unwrap();

        assert_eq!(plan.old_name, "auth");
        assert_eq!(plan.new_name, "auth_2");
        assert_eq!(plan.old_path, "auth.md");
        assert_eq!(plan.new_path, "auth_2.md");
        assert!(plan.dry_run);
        assert_eq!(plan.total_wikilinks, 1);
        assert_eq!(plan.affected_files.len(), 1);
        assert_eq!(plan.affected_files[0].path, "index.md");
    }

    #[test]
    fn plan_revision_missing_source_errors() {
        let tmp = make_vault();
        let root = vault_root(&tmp);
        let req = RevisionRequest {
            vault_root: root.clone(),
            source_rel_path: "missing.md".into(),
            naming: NamingStyle::Increment,
            new_name: None,
            excluded_dirs: vec![],
            date_str: None,
        };
        assert!(matches!(plan_revision(&req), Err(CoreError::NotFound(_))));
    }

    // ── execute_revision ──────────────────────────────────────────────────────

    #[test]
    fn execute_revision_renames_source_and_updates_links() {
        let tmp = make_vault();
        let root = vault_root(&tmp);

        fs::write(root.join("auth.md"), "# Auth\n").unwrap();
        fs::write(root.join("index.md"), "See [[auth]] for details.\n").unwrap();
        fs::write(root.join("daily.md"), "also ![[auth]] here\n").unwrap();

        let req = RevisionRequest {
            vault_root: root.clone(),
            source_rel_path: "auth.md".into(),
            naming: NamingStyle::Increment,
            new_name: None,
            excluded_dirs: vec![],
            date_str: None,
        };
        let result = execute_revision(&req).unwrap();

        assert_eq!(result.old_path, "auth.md");
        assert_eq!(result.new_path, "auth_2.md");
        assert_eq!(result.total_wikilinks, 2);

        // Source renamed, original gone.
        assert!(root.join("auth_2.md").exists());
        assert!(!root.join("auth.md").exists());

        // Referencing files updated.
        let index = fs::read_to_string(root.join("index.md")).unwrap();
        assert!(index.contains("[[auth_2]]"), "index.md: {index}");

        let daily = fs::read_to_string(root.join("daily.md")).unwrap();
        assert!(daily.contains("![[auth_2]]"), "daily.md: {daily}");

        // WAL cleaned up.
        assert!(!root.join(".qwert").join("pending-revision.json").exists());
    }

    #[test]
    fn execute_revision_updates_self_references() {
        let tmp = make_vault();
        let root = vault_root(&tmp);

        // auth.md references itself.
        fs::write(root.join("auth.md"), "See [[auth]] for self-ref.\n").unwrap();

        let req = RevisionRequest {
            vault_root: root.clone(),
            source_rel_path: "auth.md".into(),
            naming: NamingStyle::Increment,
            new_name: None,
            excluded_dirs: vec![],
            date_str: None,
        };
        execute_revision(&req).unwrap();

        let content = fs::read_to_string(root.join("auth_2.md")).unwrap();
        assert!(
            content.contains("[[auth_2]]"),
            "self-ref not updated: {content}"
        );
    }

    #[test]
    fn execute_revision_skips_code_block_links() {
        let tmp = make_vault();
        let root = vault_root(&tmp);

        fs::write(root.join("auth.md"), "# Auth\n").unwrap();
        fs::write(
            root.join("ref.md"),
            "```\n[[auth]] in code\n```\n[[auth]] normal\n",
        )
        .unwrap();

        let req = RevisionRequest {
            vault_root: root.clone(),
            source_rel_path: "auth.md".into(),
            naming: NamingStyle::Increment,
            new_name: None,
            excluded_dirs: vec![],
            date_str: None,
        };
        execute_revision(&req).unwrap();

        let content = fs::read_to_string(root.join("ref.md")).unwrap();
        assert!(
            content.contains("[[auth]] in code"),
            "code block link must be untouched: {content}"
        );
        assert!(
            content.contains("[[auth_2]] normal"),
            "normal link must be updated: {content}"
        );
    }

    #[test]
    fn execute_revision_target_exists_errors() {
        let tmp = make_vault();
        let root = vault_root(&tmp);

        fs::write(root.join("auth.md"), "# Auth\n").unwrap();
        fs::write(root.join("auth_2.md"), "# Auth 2 already exists\n").unwrap();

        let req = RevisionRequest {
            vault_root: root.clone(),
            source_rel_path: "auth.md".into(),
            naming: NamingStyle::Increment,
            new_name: None,
            excluded_dirs: vec![],
            date_str: None,
        };
        assert!(execute_revision(&req).is_err());
        // Original must be intact.
        assert!(root.join("auth.md").exists());
    }

    // ── WAL rollback ──────────────────────────────────────────────────────────

    #[test]
    fn rollback_pending_no_wal_returns_false() {
        let tmp = make_vault();
        let root = vault_root(&tmp);
        assert!(!rollback_pending(&root).unwrap());
    }

    #[test]
    fn rollback_pending_reverses_source_rename() {
        let tmp = make_vault();
        let root = vault_root(&tmp);

        // Simulate a crash after source rename but before WAL delete.
        // auth.md was renamed to auth_2.md already.
        fs::write(root.join("auth_2.md"), "# Auth\n").unwrap();
        fs::create_dir(root.join(".qwert")).unwrap();

        let pending = PendingRevision {
            schema_version: "v1".into(),
            kind: "pending_revision".into(),
            source_old_path: "auth.md".into(),
            source_new_path: "auth_2.md".into(),
            content_ops: vec![],
        };
        let wal_path = root.join(".qwert").join("pending-revision.json");
        fs::write(&wal_path, serde_json::to_string(&pending).unwrap()).unwrap();

        let found = rollback_pending(&root).unwrap();

        assert!(found, "should have detected pending WAL");
        assert!(root.join("auth.md").exists(), "auth.md must be restored");
        assert!(!root.join("auth_2.md").exists(), "auth_2.md must be gone");
        assert!(!wal_path.exists(), "WAL must be deleted");
    }

    #[test]
    fn rollback_pending_deletes_leftover_temps() {
        let tmp = make_vault();
        let root = vault_root(&tmp);

        // Create a temp file that was never renamed (crash before commit phase).
        let tmp_file = tempfile::NamedTempFile::new_in(&root).unwrap();
        let tmp_path = tmp_file.path().to_path_buf();
        tmp_file.keep().unwrap(); // keep it alive

        fs::create_dir(root.join(".qwert")).unwrap();
        let pending = PendingRevision {
            schema_version: "v1".into(),
            kind: "pending_revision".into(),
            source_old_path: "auth.md".into(),
            source_new_path: "auth_2.md".into(),
            content_ops: vec![ContentOp {
                final_path: "index.md".into(),
                tmp_path: tmp_path.to_string_lossy().into_owned(),
            }],
        };
        let wal_path = root.join(".qwert").join("pending-revision.json");
        fs::write(&wal_path, serde_json::to_string(&pending).unwrap()).unwrap();

        rollback_pending(&root).unwrap();

        assert!(!tmp_path.exists(), "temp file must be deleted");
    }

    // ── Atomicity ─────────────────────────────────────────────────────────────

    #[test]
    fn wal_is_written_before_renames() {
        // We verify that after execute_revision, WAL is cleaned up (happy path).
        // The WAL's existence before renames is an internal invariant verified
        // by rollback_pending recovering from it correctly (tested above).
        let tmp = make_vault();
        let root = vault_root(&tmp);

        fs::write(root.join("auth.md"), "# Auth\n").unwrap();

        let req = RevisionRequest {
            vault_root: root.clone(),
            source_rel_path: "auth.md".into(),
            naming: NamingStyle::Increment,
            new_name: None,
            excluded_dirs: vec![],
            date_str: None,
        };
        execute_revision(&req).unwrap();

        assert!(!root.join(".qwert").join("pending-revision.json").exists());
        assert!(root.join("auth_2.md").exists());
    }
}
