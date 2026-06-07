use ignore::WalkBuilder;
use regex::{Regex, escape};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SearchHit {
    pub path: String,
    pub line: usize,
    pub snippet: String,
}

/// Search for `query` in all `.md` files under `vault_root`.
///
/// `use_regex = false`: query is treated as a literal string.
/// `use_regex = true`:  query is interpreted as a regex pattern.
///
/// Returns `CoreError::InvalidPattern` if `use_regex` is true and the pattern
/// fails to compile.
pub fn search_vault(
    vault_root: &Path,
    query: &str,
    use_regex: bool,
) -> crate::Result<Vec<SearchHit>> {
    let pattern = build_pattern(query, use_regex)?;
    let mut hits = Vec::new();

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

        let rel = path
            .strip_prefix(vault_root)
            .map(|r| r.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();

        for (idx, line) in content.lines().enumerate() {
            if pattern.is_match(line) {
                hits.push(SearchHit {
                    path: rel.clone(),
                    line: idx + 1,
                    snippet: line.to_owned(),
                });
            }
        }
    }

    Ok(hits)
}

fn build_pattern(query: &str, use_regex: bool) -> crate::Result<Regex> {
    let pat = if use_regex {
        query.to_owned()
    } else {
        escape(query)
    };
    Regex::new(&pat).map_err(|e| crate::CoreError::InvalidPattern(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn make_vault() -> TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn literal_match_returns_correct_line_and_snippet() {
        let vault = make_vault();
        let root = vault.path();
        fs::write(root.join("note.md"), "line1\nTODO: fix me\nline3\n").unwrap();

        let hits = search_vault(root, "TODO", false).unwrap();

        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].line, 2);
        assert_eq!(hits[0].snippet, "TODO: fix me");
        assert_eq!(hits[0].path, "note.md");
    }

    #[test]
    fn multiple_hits_across_files() {
        let vault = make_vault();
        let root = vault.path();
        fs::write(root.join("a.md"), "TODO first\nnormal line\n").unwrap();
        fs::write(root.join("b.md"), "no match here\nTODO second\n").unwrap();

        let mut hits = search_vault(root, "TODO", false).unwrap();
        hits.sort_by(|a, b| a.path.cmp(&b.path));

        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].path, "a.md");
        assert_eq!(hits[0].line, 1);
        assert_eq!(hits[1].path, "b.md");
        assert_eq!(hits[1].line, 2);
    }

    #[test]
    fn multiple_hits_in_single_file() {
        let vault = make_vault();
        let root = vault.path();
        fs::write(root.join("notes.md"), "TODO one\nmiddle\nTODO two\n").unwrap();

        let hits = search_vault(root, "TODO", false).unwrap();

        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].line, 1);
        assert_eq!(hits[1].line, 3);
    }

    #[test]
    fn zero_hits_returns_empty_vec() {
        let vault = make_vault();
        let root = vault.path();
        fs::write(root.join("note.md"), "nothing to find here\n").unwrap();

        let hits = search_vault(root, "TODO", false).unwrap();
        assert!(hits.is_empty());
    }

    #[test]
    fn non_md_files_are_ignored() {
        let vault = make_vault();
        let root = vault.path();
        fs::write(root.join("note.md"), "TODO in markdown\n").unwrap();
        fs::write(root.join("script.sh"), "TODO in shell\n").unwrap();
        fs::write(root.join("data.txt"), "TODO in text\n").unwrap();

        let hits = search_vault(root, "TODO", false).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].path, "note.md");
    }

    #[test]
    fn regex_mode_works() {
        let vault = make_vault();
        let root = vault.path();
        // Two lines starting with TODO, one that does not
        fs::write(
            root.join("note.md"),
            "TODO: fix\nTODO: also fix\nfix TODO\n",
        )
        .unwrap();

        let hits = search_vault(root, r"^TODO", true).unwrap();
        assert_eq!(hits.len(), 2);
        for h in &hits {
            assert!(h.snippet.starts_with("TODO"));
        }
    }

    #[test]
    fn invalid_regex_returns_error() {
        let vault = make_vault();
        let result = search_vault(vault.path(), "[invalid", true);
        assert!(matches!(result, Err(crate::CoreError::InvalidPattern(_))));
    }

    #[test]
    fn literal_special_chars_are_escaped() {
        let vault = make_vault();
        let root = vault.path();
        // A query that would be invalid regex if not escaped
        fs::write(root.join("note.md"), "price: $1.00\nother line\n").unwrap();

        let hits = search_vault(root, "$1.00", false).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].snippet, "price: $1.00");
    }

    #[test]
    fn gitignore_excludes_ignored_files() {
        let vault = make_vault();
        let root = vault.path();
        // ignore::WalkBuilder respects .gitignore only inside a git repository.
        // Creating an empty .git dir is enough to trigger that behavior.
        fs::create_dir(root.join(".git")).unwrap();
        fs::write(root.join("included.md"), "TODO: include me\n").unwrap();
        fs::write(root.join("excluded.md"), "TODO: exclude me\n").unwrap();
        fs::write(root.join(".gitignore"), "excluded.md\n").unwrap();

        let hits = search_vault(root, "TODO", false).unwrap();
        assert!(
            hits.iter().all(|h| h.path != "excluded.md"),
            "excluded.md must not appear: {hits:?}"
        );
        assert!(
            hits.iter().any(|h| h.path == "included.md"),
            "included.md must appear: {hits:?}"
        );
    }
}
