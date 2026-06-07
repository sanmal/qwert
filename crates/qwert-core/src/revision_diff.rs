use std::io::Write as _;
use std::path::Path;

use similar::TextDiff;

use crate::link_index::replace_wikilinks;
use crate::revision::RevisionPlan;
use crate::{CoreError, Result};

// ── Types ─────────────────────────────────────────────────────────────────────

pub struct DiffRequest {
    /// Vault-relative path shown in the diff header for the old side.
    pub old_path: String,
    /// Vault-relative path shown in the diff header for the new side.
    pub new_path: String,
    pub old_content: String,
    pub new_content: String,
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Generate a unified diff string for a single file change.
/// Output is patch(1) / git apply compatible.
pub fn generate_diff(req: &DiffRequest) -> Result<String> {
    let diff = TextDiff::from_lines(&req.old_content, &req.new_content);
    let output = format!(
        "{}",
        diff.unified_diff().header(
            &format!("a/{}", req.old_path),
            &format!("b/{}", req.new_path)
        )
    );
    Ok(output)
}

/// Compute per-file diffs for all affected files in `plan`.
/// The source file (being renamed) uses old_path → new_path in the diff header.
pub fn compute_diffs_for_plan(vault_root: &Path, plan: &RevisionPlan) -> Result<Vec<DiffRequest>> {
    let mut diffs = Vec::new();
    for af in &plan.affected_files {
        let abs = vault_root.join(&af.path);
        let old_content = std::fs::read_to_string(&abs)?;
        let new_content = replace_wikilinks(&old_content, &plan.old_name, &plan.new_name);

        // Source file gets renamed: use distinct paths in the diff header.
        let (old_path, new_path) = if af.path == plan.old_path {
            (plan.old_path.clone(), plan.new_path.clone())
        } else {
            (af.path.clone(), af.path.clone())
        };

        diffs.push(DiffRequest {
            old_path,
            new_path,
            old_content,
            new_content,
        });
    }
    Ok(diffs)
}

/// Write `content` to a persistent named temp file and return its path.
/// The caller is responsible for cleanup.
pub fn write_diff_to_tempfile(content: &str) -> Result<String> {
    let mut tmp = tempfile::Builder::new()
        .prefix("qwert-diff-")
        .suffix(".patch")
        .tempfile()
        .map_err(CoreError::Io)?;
    tmp.write_all(content.as_bytes()).map_err(CoreError::Io)?;
    let (_, path) = tmp.keep().map_err(|e| CoreError::Io(e.error))?;
    Ok(path.to_string_lossy().into_owned())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn req(old: &str, new: &str) -> DiffRequest {
        DiffRequest {
            old_path: "auth.md".into(),
            new_path: "auth_2.md".into(),
            old_content: old.into(),
            new_content: new.into(),
        }
    }

    #[test]
    fn generate_diff_has_unified_header() {
        let r = req("line1\nline2\n", "line1\nline2_changed\n");
        let diff = generate_diff(&r).unwrap();
        assert!(diff.contains("--- a/auth.md"), "missing old header: {diff}");
        assert!(
            diff.contains("+++ b/auth_2.md"),
            "missing new header: {diff}"
        );
    }

    #[test]
    fn generate_diff_shows_deletion_and_insertion() {
        let r = req("[[auth]] ref\n", "[[auth_2]] ref\n");
        let diff = generate_diff(&r).unwrap();
        assert!(diff.contains("-[[auth]] ref"), "missing deletion: {diff}");
        assert!(
            diff.contains("+[[auth_2]] ref"),
            "missing insertion: {diff}"
        );
    }

    #[test]
    fn generate_diff_empty_when_no_change() {
        let r = req("same content\n", "same content\n");
        let diff = generate_diff(&r).unwrap();
        // No hunks → empty or just headers
        assert!(
            !diff.contains("@@"),
            "unexpected hunk for identical content: {diff}"
        );
    }

    #[test]
    fn compute_diffs_for_plan_finds_affected_files() {
        use std::fs;
        use tempfile::TempDir;

        let tmp = TempDir::new().unwrap();
        let root = tmp.path().canonicalize().unwrap();
        fs::write(root.join("auth.md"), "# Auth\n").unwrap();
        fs::write(root.join("index.md"), "See [[auth]] here.\n").unwrap();

        let plan = crate::revision::RevisionPlan {
            schema_version: "v1".into(),
            kind: "revision_plan".into(),
            dry_run: true,
            old_name: "auth".into(),
            new_name: "auth_2".into(),
            old_path: "auth.md".into(),
            new_path: "auth_2.md".into(),
            affected_files: vec![crate::revision::AffectedFile {
                path: "index.md".into(),
                wikilink_count: 1,
            }],
            total_wikilinks: 1,
        };

        let diffs = compute_diffs_for_plan(&root, &plan).unwrap();
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].old_path, "index.md");
        assert_eq!(diffs[0].new_path, "index.md");
        assert!(diffs[0].new_content.contains("[[auth_2]]"));
    }

    #[test]
    fn compute_diffs_source_file_uses_new_path_header() {
        use std::fs;
        use tempfile::TempDir;

        let tmp = TempDir::new().unwrap();
        let root = tmp.path().canonicalize().unwrap();
        // Source file has a self-reference.
        fs::write(root.join("auth.md"), "See [[auth]] self-ref.\n").unwrap();

        let plan = crate::revision::RevisionPlan {
            schema_version: "v1".into(),
            kind: "revision_plan".into(),
            dry_run: true,
            old_name: "auth".into(),
            new_name: "auth_2".into(),
            old_path: "auth.md".into(),
            new_path: "auth_2.md".into(),
            affected_files: vec![crate::revision::AffectedFile {
                path: "auth.md".into(),
                wikilink_count: 1,
            }],
            total_wikilinks: 1,
        };

        let diffs = compute_diffs_for_plan(&root, &plan).unwrap();
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].old_path, "auth.md");
        assert_eq!(diffs[0].new_path, "auth_2.md");
    }
}
