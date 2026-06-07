use std::path::Path;
use std::sync::OnceLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

static SYNC_CONFLICT_RE: OnceLock<Regex> = OnceLock::new();
fn sync_conflict_re() -> &'static Regex {
    SYNC_CONFLICT_RE.get_or_init(|| {
        // Syncthing conflict format: <name>.sync-conflict-YYYYMMDD-HHMMSS-DEVICEID.<ext>
        Regex::new(r"\.sync-conflict-\d{8}-\d{6}-[A-Z0-9]+(\.[^./]+)?$").unwrap()
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SyncConflict {
    /// Vault-relative path of the base (original) file.
    pub base: String,
    /// Vault-relative path of the conflict file.
    pub conflict_file: String,
}

/// Summary of an incomplete Revision WAL (`.qwert/pending-revision.json`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PendingRevisionInfo {
    pub source_old_path: String,
    pub source_new_path: String,
}

/// Snapshot of vault health. Matches spec §9 `vault_status` JSON shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultStatus {
    /// Absolute path of the vault root.
    pub vault: String,
    /// Syncthing `.sync-conflict-*` files found anywhere in the vault.
    pub sync_conflicts: Vec<SyncConflict>,
    /// Leftover WAL from a crashed Revision, or `null`.
    pub pending_revision: Option<PendingRevisionInfo>,
    /// `true` when `warnings` is empty.
    pub healthy: bool,
    /// Human-readable warning messages.
    pub warnings: Vec<String>,
}

/// Inspect the vault and return its health status.
pub fn check_vault_status(vault_root: &Path) -> crate::Result<VaultStatus> {
    let vault_str = vault_root.to_string_lossy().into_owned();
    let mut sync_conflicts: Vec<SyncConflict> = Vec::new();
    let mut pending_revision: Option<PendingRevisionInfo> = None;
    let mut appearance_conflicts = 0usize;

    // ── Scan vault for *.sync-conflict-*.md files ──────────────────────────
    // Walk the entire vault, skipping hidden dirs (except .qwert).
    // filter_entry with `depth == 0` ensures the root is never filtered.
    for entry in walkdir::WalkDir::new(vault_root)
        .into_iter()
        .filter_entry(|e| {
            // Always allow the root entry (depth 0).
            if e.depth() == 0 {
                return true;
            }
            let name = e.file_name().to_str().unwrap_or("");
            if e.file_type().is_dir() {
                // Enter .qwert for appearance conflicts; skip other hidden dirs and node_modules
                name == ".qwert" || (!name.starts_with('.') && name != "node_modules")
            } else {
                true // never filter out files
            }
        })
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let file_name = entry.file_name().to_str().unwrap_or("");
        let path = entry.path();

        // Check for Syncthing .sync-conflict-*.md
        if sync_conflict_re().is_match(file_name) && file_name.ends_with(".md") {
            let rel = path
                .strip_prefix(vault_root)
                .map(|r| r.to_string_lossy().replace('\\', "/"))
                .unwrap_or_default();
            let base = conflict_base(&rel).unwrap_or_else(|| rel.clone());
            sync_conflicts.push(SyncConflict {
                base,
                conflict_file: rel,
            });
        }

        // Check for .qwert/appearance.sync-conflict-*.toml
        if file_name.starts_with("appearance.sync-conflict-")
            && file_name.ends_with(".toml")
            && path
                .parent()
                .map(|p| p.ends_with(".qwert"))
                .unwrap_or(false)
        {
            appearance_conflicts += 1;
        }
    }

    // ── Check .qwert/pending-revision.json ────────────────────────────────
    let wal_path = vault_root.join(".qwert").join("pending-revision.json");
    if wal_path.exists()
        && let Ok(raw) = std::fs::read_to_string(&wal_path)
        && let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw)
    {
        let old = v["source_old_path"].as_str().unwrap_or("?").to_owned();
        let new = v["source_new_path"].as_str().unwrap_or("?").to_owned();
        pending_revision = Some(PendingRevisionInfo {
            source_old_path: old,
            source_new_path: new,
        });
    }

    // ── Build warnings ─────────────────────────────────────────────────────
    let mut warnings = Vec::new();

    if !sync_conflicts.is_empty() {
        warnings.push(format!(
            "{} sync-conflict file(s) detected. Resolve manually before continuing.",
            sync_conflicts.len()
        ));
    }
    if pending_revision.is_some() {
        warnings.push(
            "Incomplete revision detected (.qwert/pending-revision.json). \
             Run `qwert vault status` to inspect, then resolve manually."
                .to_owned(),
        );
    }
    if appearance_conflicts > 0 {
        warnings.push(format!(
            "{appearance_conflicts} appearance.toml conflict(s) in .qwert/. \
             Resolve manually."
        ));
    }

    Ok(VaultStatus {
        vault: vault_str,
        sync_conflicts,
        pending_revision,
        healthy: warnings.is_empty(),
        warnings,
    })
}

/// Derive the base file path from a Syncthing conflict file path.
///
/// `"specs/auth.sync-conflict-20260422-123456-ABC123.md"` → `"specs/auth.md"`
fn conflict_base(conflict_rel: &str) -> Option<String> {
    let re = sync_conflict_re();
    let m = re.find(conflict_rel)?;
    // capture group 1 is the extension (e.g. ".md")
    let ext = re
        .captures(&conflict_rel[m.start()..])
        .and_then(|c| c.get(1))
        .map(|e| e.as_str())
        .unwrap_or("");
    Some(format!("{}{}", &conflict_rel[..m.start()], ext))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn make_vault() -> TempDir {
        tempfile::tempdir().unwrap()
    }

    // ── conflict_base ─────────────────────────────────────────────────────────

    #[test]
    fn conflict_base_simple() {
        assert_eq!(
            conflict_base("auth.sync-conflict-20260422-123456-ABC1234.md"),
            Some("auth.md".into())
        );
    }

    #[test]
    fn conflict_base_with_subdirectory() {
        assert_eq!(
            conflict_base("specs/auth.sync-conflict-20260422-123456-ABC1234.md"),
            Some("specs/auth.md".into())
        );
    }

    #[test]
    fn conflict_base_non_conflict_returns_none() {
        assert_eq!(conflict_base("specs/auth.md"), None);
    }

    // ── check_vault_status ────────────────────────────────────────────────────

    #[test]
    fn healthy_vault_has_no_warnings() {
        let vault = make_vault();
        let root = vault.path().canonicalize().unwrap();
        fs::write(root.join("note.md"), "hello").unwrap();

        let status = check_vault_status(&root).unwrap();
        assert!(status.healthy, "expected healthy vault");
        assert!(status.warnings.is_empty());
        assert!(status.sync_conflicts.is_empty());
        assert!(status.pending_revision.is_none());
    }

    #[test]
    fn detects_sync_conflict_file() {
        let vault = make_vault();
        let root = vault.path().canonicalize().unwrap();
        fs::write(
            root.join("auth.sync-conflict-20260422-123456-ABC1234.md"),
            "conflict",
        )
        .unwrap();
        fs::write(root.join("auth.md"), "original").unwrap();

        let status = check_vault_status(&root).unwrap();
        assert!(!status.healthy);
        assert_eq!(status.sync_conflicts.len(), 1);
        assert_eq!(status.sync_conflicts[0].base, "auth.md");
        assert!(
            status.sync_conflicts[0]
                .conflict_file
                .contains("sync-conflict")
        );
        assert!(!status.warnings.is_empty());
    }

    #[test]
    fn detects_pending_revision_wal() {
        let vault = make_vault();
        let root = vault.path().canonicalize().unwrap();
        let qwert_dir = root.join(".qwert");
        fs::create_dir(&qwert_dir).unwrap();
        let wal = qwert_dir.join("pending-revision.json");
        fs::write(
            &wal,
            r#"{"kind":"pending_revision","source_old_path":"specs/auth.md","source_new_path":"specs/auth_2.md","content_ops":[]}"#,
        )
        .unwrap();

        let status = check_vault_status(&root).unwrap();
        assert!(!status.healthy);
        let pr = status
            .pending_revision
            .as_ref()
            .expect("should have pending_revision");
        assert_eq!(pr.source_old_path, "specs/auth.md");
        assert_eq!(pr.source_new_path, "specs/auth_2.md");
        assert!(!status.warnings.is_empty());
    }

    #[test]
    fn detects_appearance_conflict() {
        let vault = make_vault();
        let root = vault.path().canonicalize().unwrap();
        let qwert_dir = root.join(".qwert");
        fs::create_dir(&qwert_dir).unwrap();
        fs::write(
            qwert_dir.join("appearance.sync-conflict-20260422-123456-ABC1234.toml"),
            "",
        )
        .unwrap();

        let status = check_vault_status(&root).unwrap();
        assert!(!status.healthy);
        assert!(status.warnings.iter().any(|w| w.contains("appearance")));
    }

    #[test]
    fn no_false_positive_for_normal_md() {
        let vault = make_vault();
        let root = vault.path().canonicalize().unwrap();
        fs::write(root.join("note.md"), "content").unwrap();
        fs::write(root.join("sync-notes.md"), "another").unwrap(); // name with "sync" but not conflict

        let status = check_vault_status(&root).unwrap();
        assert!(status.healthy);
        assert!(status.sync_conflicts.is_empty());
    }
}
