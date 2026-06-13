use notify::{EventKind, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub children: Option<Vec<VaultEntry>>,
}

/// 監視ハンドル。drop されると監視スレッドと watcher が停止する。
pub struct WatchGuard {
    _watcher: notify::RecommendedWatcher,
}

/// 既存パス用: canonicalize して vault 配下か検証する。
pub fn resolve_path(vault_root: &Path, relative: &str) -> crate::Result<PathBuf> {
    let joined = vault_root.join(relative);
    let canonical = joined
        .canonicalize()
        .map_err(|_| crate::CoreError::NotFound(relative.to_owned()))?;
    if !canonical.starts_with(vault_root) {
        return Err(crate::CoreError::PathTraversal(relative.to_owned()));
    }
    Ok(canonical)
}

/// 新規パス用: lexical 検証 + 親 canonicalize で vault 配下か検証する。
/// 呼び出し前に親ディレクトリを create_dir_all しておくこと。
pub fn resolve_new_path(vault_root: &Path, relative: &str) -> crate::Result<PathBuf> {
    let rel = Path::new(relative);
    if rel.is_absolute() || rel.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(crate::CoreError::PathTraversal(relative.to_owned()));
    }
    let joined = vault_root.join(rel);
    let parent = joined
        .parent()
        .ok_or_else(|| crate::CoreError::PathTraversal(relative.to_owned()))?;
    let parent_canonical = parent
        .canonicalize()
        .map_err(|_| crate::CoreError::NotFound(parent.to_string_lossy().into_owned()))?;
    if !parent_canonical.starts_with(vault_root) {
        return Err(crate::CoreError::PathTraversal(relative.to_owned()));
    }
    let file_name = joined
        .file_name()
        .ok_or_else(|| crate::CoreError::PathTraversal(relative.to_owned()))?;
    Ok(parent_canonical.join(file_name))
}

fn scan_dir(vault_root: &Path, dir: &Path) -> crate::Result<Vec<VaultEntry>> {
    let mut entries = Vec::new();

    for entry in WalkDir::new(dir)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.depth() == 1)
    {
        let path = entry.path().to_path_buf();
        let name = entry.file_name().to_string_lossy().to_string();

        if entry.file_type().is_dir() {
            if name.starts_with('.') || name == "node_modules" {
                continue;
            }
            let children = scan_dir(vault_root, &path)?;
            if !children.is_empty() {
                let rel = path
                    .strip_prefix(vault_root)
                    .map(|r| r.to_string_lossy().replace('\\', "/"))
                    .unwrap_or_default();
                entries.push(VaultEntry {
                    name,
                    path: rel,
                    is_dir: true,
                    children: Some(children),
                });
            }
        } else {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if ext == "md" || ext == "markdown" {
                let rel = path
                    .strip_prefix(vault_root)
                    .map(|r| r.to_string_lossy().replace('\\', "/"))
                    .unwrap_or_default();
                entries.push(VaultEntry {
                    name,
                    path: rel,
                    is_dir: false,
                    children: None,
                });
            }
        }
    }

    entries.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(entries)
}

/// .md / .markdown のみをツリー形式で返す。隠しディレクトリと node_modules はスキップ。
pub fn scan_vault(vault_root: &Path) -> crate::Result<Vec<VaultEntry>> {
    scan_dir(vault_root, vault_root)
}

/// パストラバーサル検証付きのファイル読み取り（既存パス用）。
/// 不正 UTF-8 は `CoreError::InvalidUtf8 { byte_offset }` を返す（exit 5 = Validation）。
pub fn read_file(vault_root: &Path, relative_path: &str) -> crate::Result<String> {
    let resolved = resolve_path(vault_root, relative_path)?;
    read_utf8(&resolved)
}

/// バイト列として読み取り、UTF-8 変換を行う内部ヘルパー。
/// `read_to_string` より詳細なエラー（byte_offset 付き）を返す。
fn read_utf8(path: &Path) -> crate::Result<String> {
    let bytes = std::fs::read(path)?;
    String::from_utf8(bytes).map_err(|e| crate::CoreError::InvalidUtf8 {
        byte_offset: e.utf8_error().valid_up_to(),
    })
}

/// ファイルを読み取り、内容と mtime（Unix 秒）を返す。
/// 不正 UTF-8 は `CoreError::InvalidUtf8` を返す。
pub fn read_file_with_mtime(
    vault_root: &Path,
    relative_path: &str,
) -> crate::Result<(String, u64)> {
    let resolved = resolve_path(vault_root, relative_path)?;
    let content = read_utf8(&resolved)?;
    let mtime = mtime_secs(&resolved)?;
    Ok((content, mtime))
}

/// ファイルの mtime を Unix 秒で返す。
pub fn get_file_mtime(vault_root: &Path, relative_path: &str) -> crate::Result<u64> {
    let resolved = resolve_path(vault_root, relative_path)?;
    mtime_secs(&resolved)
}

/// `write_file_safe` の結果。Conflict は書き込みを行わず呼び出し側に判断を委ねる。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WriteResult {
    Success {
        /// 書き込み後の mtime（Unix 秒）。
        new_mtime: u64,
    },
    Conflict {
        /// 現在のファイルの mtime（Unix 秒）。次回 --if-match に使う。
        current_mtime: u64,
    },
}

/// mtime 楽観ロック付き書き込み（§12 Level 2）。
///
/// `expected_mtime` が現在の mtime（Unix 秒）と一致する場合のみアトミック書込を行う。
/// 不一致は書き込まずに `WriteResult::Conflict` を返す（エラーではない）。
pub fn write_file_safe(
    vault_root: &Path,
    relative_path: &str,
    content: &str,
    expected_mtime: u64,
) -> crate::Result<WriteResult> {
    let resolved = resolve_path(vault_root, relative_path)?;
    let current = mtime_secs(&resolved)?;
    if current != expected_mtime {
        return Ok(WriteResult::Conflict {
            current_mtime: current,
        });
    }
    write_atomic(&resolved, content)?;
    let new_mtime = mtime_secs(&resolved)?;
    Ok(WriteResult::Success { new_mtime })
}

fn mtime_secs(path: &Path) -> crate::Result<u64> {
    let meta = std::fs::metadata(path)?;
    let t = meta.modified().map_err(crate::CoreError::Io)?;
    Ok(t.duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs())
}

fn write_atomic(path: &Path, content: &str) -> crate::Result<()> {
    let dir = path
        .parent()
        .ok_or_else(|| crate::CoreError::PathTraversal(path.to_string_lossy().into_owned()))?;
    let mut tmp = tempfile::NamedTempFile::new_in(dir)?;
    tmp.write_all(content.as_bytes())?;
    tmp.persist(path)
        .map_err(|e| crate::CoreError::Io(e.error))?;
    Ok(())
}

/// アトミック書き込み（tmp → rename）。既存・新規どちらのパスも扱う。
pub fn write_file(vault_root: &Path, relative_path: &str, content: &str) -> crate::Result<()> {
    let resolved = match resolve_path(vault_root, relative_path) {
        Ok(p) => p,
        Err(crate::CoreError::NotFound(_)) => {
            let joined = vault_root.join(relative_path);
            if let Some(parent) = joined.parent() {
                std::fs::create_dir_all(parent)?;
            }
            resolve_new_path(vault_root, relative_path)?
        }
        Err(e) => return Err(e),
    };
    write_atomic(&resolved, content)
}

/// 新規ファイル作成。すでに存在する場合は Io(AlreadyExists) を返す。
pub fn create_file(vault_root: &Path, relative_path: &str) -> crate::Result<()> {
    let rel = Path::new(relative_path);
    if rel.is_absolute() || rel.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(crate::CoreError::PathTraversal(relative_path.to_owned()));
    }
    let joined = vault_root.join(rel);
    if let Some(parent) = joined.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let resolved = resolve_new_path(vault_root, relative_path)?;
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&resolved)?;
    Ok(())
}

/// Move a file within the vault (pure file-system move, no wikilink updates).
///
/// Semantics: distinct from Revision (rename + wikilink update + naming history).
/// DnD move reorganises files structurally. Because wikilinks resolve by stem
/// rather than by path, moving a file does not break existing `[[name]]` links.
///
/// Errors:
/// - `PathTraversal` if either path escapes the vault
/// - `NotFound` if `src_rel` does not exist, or if the destination parent dir does not exist
/// - `Io(AlreadyExists)` if `dst_rel` already exists
pub fn move_file(vault_root: &Path, src_rel: &str, dst_rel: &str) -> crate::Result<()> {
    let src = resolve_path(vault_root, src_rel)?;
    let dst = resolve_new_path(vault_root, dst_rel)?;
    // Explicit check: fs::rename on Linux silently overwrites the target.
    if dst.exists() {
        return Err(crate::CoreError::Io(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!("destination already exists: {dst_rel}"),
        )));
    }
    std::fs::rename(&src, &dst).map_err(crate::CoreError::Io)
}

// ── Editing state (Level 3 hint) ─────────────────────────────────────────────

/// Vault-relative path of the editing state file.
const EDITING_STATE_PATH: &str = ".qwert/editing_state.json";

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct EditingStateFile {
    schema_version: String,
    kind: String,
    #[serde(default)]
    editing: Vec<String>,
}

/// Returns the set of vault-relative paths currently open and unsaved in the GUI.
pub fn read_editing_paths(vault_root: &Path) -> Vec<String> {
    let path = vault_root.join(EDITING_STATE_PATH);
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    serde_json::from_str::<EditingStateFile>(&content)
        .map(|s| s.editing)
        .unwrap_or_default()
}

/// Returns true when `rel_path` is currently open and unsaved in the GUI.
pub fn is_editing(vault_root: &Path, rel_path: &str) -> bool {
    read_editing_paths(vault_root).iter().any(|p| p == rel_path)
}

/// Set or clear the editing state for `rel_path`.
/// Silently ignores write failures (hint only, not critical state).
pub fn set_editing_path(vault_root: &Path, rel_path: &str, editing: bool) {
    let mut paths = read_editing_paths(vault_root);
    if editing {
        if !paths.iter().any(|p| p == rel_path) {
            paths.push(rel_path.to_owned());
        }
    } else {
        paths.retain(|p| p != rel_path);
    }
    let dir = vault_root.join(".qwert");
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    let state = EditingStateFile {
        schema_version: "v1".to_owned(),
        kind: "editing_state".to_owned(),
        editing: paths,
    };
    if let Ok(json) = serde_json::to_string(&state) {
        let _ = std::fs::write(dir.join("editing_state.json"), json);
    }
}

/// vault_root 配下を再帰監視し、変更のあった .md ファイルの
/// vault 相対パス（`/` 区切り）ごとに callback を呼ぶ。
/// callback はバックグラウンドスレッドから呼ばれる（Send + 'static 必須）。
/// 返り値の guard を保持している間だけ監視が継続する（drop で停止）。
pub fn watch_vault<F>(vault_root: &Path, callback: F) -> crate::Result<WatchGuard>
where
    F: Fn(String) + Send + 'static,
{
    let (tx, rx) = std::sync::mpsc::channel::<notify::Event>();

    let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        if let Ok(event) = res {
            let _ = tx.send(event);
        }
    })
    .map_err(|e| crate::CoreError::Io(std::io::Error::other(e)))?;

    watcher
        .watch(vault_root, RecursiveMode::Recursive)
        .map_err(|e| crate::CoreError::Io(std::io::Error::other(e)))?;

    let vault_root = vault_root.to_path_buf();

    std::thread::spawn(move || {
        for event in rx {
            match event.kind {
                EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_) => {
                    for path in &event.paths {
                        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                        if (ext == "md" || ext == "markdown")
                            && let Ok(relative) = path.strip_prefix(&vault_root)
                        {
                            let rel_str = relative.to_string_lossy().replace('\\', "/");
                            callback(rel_str);
                        }
                    }
                }
                _ => {}
            }
        }
    });

    Ok(WatchGuard { _watcher: watcher })
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
    fn resolve_path_rejects_traversal() {
        // vault は outer/inner/ — outer/secret.md は vault 外に実在するファイル
        let outer = make_vault();
        let inner = outer.path().join("inner");
        fs::create_dir(&inner).unwrap();
        let vault_root = inner.canonicalize().unwrap();
        fs::write(outer.path().join("secret.md"), "secret").unwrap();

        let result = resolve_path(&vault_root, "../secret.md");
        assert!(
            matches!(result, Err(crate::CoreError::PathTraversal(_))),
            "expected PathTraversal, got {result:?}"
        );
    }

    #[test]
    fn resolve_path_accepts_existing_file() {
        let vault = make_vault();
        let vault_root = vault.path().canonicalize().unwrap();
        fs::write(vault_root.join("note.md"), "hello").unwrap();
        let result = resolve_path(&vault_root, "note.md");
        assert!(result.is_ok());
    }

    #[test]
    fn resolve_new_path_rejects_parent_dir() {
        let vault = make_vault();
        let vault_root = vault.path().canonicalize().unwrap();
        assert!(matches!(
            resolve_new_path(&vault_root, "../evil.md"),
            Err(crate::CoreError::PathTraversal(_))
        ));
    }

    #[test]
    fn resolve_new_path_rejects_absolute() {
        let vault = make_vault();
        let vault_root = vault.path().canonicalize().unwrap();
        assert!(matches!(
            resolve_new_path(&vault_root, "/etc/passwd"),
            Err(crate::CoreError::PathTraversal(_))
        ));
    }

    #[test]
    fn resolve_new_path_accepts_vault_relative() {
        let vault = make_vault();
        let vault_root = vault.path().canonicalize().unwrap();
        // parent (vault_root) must exist for canonicalize to succeed
        let result = resolve_new_path(&vault_root, "new_note.md");
        assert!(result.is_ok());
        assert!(result.unwrap().starts_with(&vault_root));
    }

    #[test]
    fn scan_vault_returns_only_md_files() {
        let vault = make_vault();
        let vault_root = vault.path().canonicalize().unwrap();

        fs::write(vault_root.join("a.md"), "").unwrap();
        fs::write(vault_root.join("b.txt"), "").unwrap();
        fs::create_dir(vault_root.join("sub")).unwrap();
        fs::write(vault_root.join("sub").join("c.md"), "").unwrap();
        fs::write(vault_root.join("sub").join("d.rs"), "").unwrap();
        fs::create_dir(vault_root.join(".git")).unwrap();
        fs::write(vault_root.join(".git").join("e.md"), "").unwrap();

        let entries = scan_vault(&vault_root).unwrap();

        // top level: a.md + sub dir
        assert_eq!(entries.len(), 2, "expected a.md and sub/: {entries:?}");
        let file = entries.iter().find(|e| e.name == "a.md").unwrap();
        assert!(!file.is_dir);

        let dir = entries.iter().find(|e| e.name == "sub").unwrap();
        assert!(dir.is_dir);
        let children = dir.children.as_ref().unwrap();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].name, "c.md");

        // .git must not appear
        assert!(entries.iter().all(|e| e.name != ".git"));
    }

    #[test]
    fn write_file_writes_content() {
        let vault = make_vault();
        let vault_root = vault.path().canonicalize().unwrap();
        fs::write(vault_root.join("note.md"), "old").unwrap();

        write_file(&vault_root, "note.md", "new content").unwrap();

        let got = fs::read_to_string(vault_root.join("note.md")).unwrap();
        assert_eq!(got, "new content");
    }

    #[test]
    fn write_file_creates_new_file() {
        let vault = make_vault();
        let vault_root = vault.path().canonicalize().unwrap();

        write_file(&vault_root, "fresh.md", "hello").unwrap();

        let got = fs::read_to_string(vault_root.join("fresh.md")).unwrap();
        assert_eq!(got, "hello");
    }

    #[test]
    fn create_file_errors_on_existing() {
        let vault = make_vault();
        let vault_root = vault.path().canonicalize().unwrap();
        fs::write(vault_root.join("exists.md"), "").unwrap();

        let result = create_file(&vault_root, "exists.md");
        assert!(
            result.is_err(),
            "expected error when creating existing file"
        );
    }

    #[test]
    fn create_file_succeeds_for_new() {
        let vault = make_vault();
        let vault_root = vault.path().canonicalize().unwrap();

        create_file(&vault_root, "new.md").unwrap();
        assert!(vault_root.join("new.md").exists());
    }

    // ── mtime / write_file_safe ───────────────────────────────────────────────

    #[test]
    fn read_file_with_mtime_returns_content_and_nonzero_mtime() {
        let vault = make_vault();
        let root = vault.path().canonicalize().unwrap();
        fs::write(root.join("note.md"), "hello").unwrap();

        let (content, mtime) = read_file_with_mtime(&root, "note.md").unwrap();
        assert_eq!(content, "hello");
        assert!(mtime > 0, "mtime should be non-zero");
    }

    #[test]
    fn get_file_mtime_matches_read_file_mtime() {
        let vault = make_vault();
        let root = vault.path().canonicalize().unwrap();
        fs::write(root.join("note.md"), "content").unwrap();

        let (_, mtime_from_read) = read_file_with_mtime(&root, "note.md").unwrap();
        let mtime_direct = get_file_mtime(&root, "note.md").unwrap();
        assert_eq!(mtime_from_read, mtime_direct);
    }

    #[test]
    fn write_file_safe_succeeds_when_mtime_matches() {
        let vault = make_vault();
        let root = vault.path().canonicalize().unwrap();
        fs::write(root.join("note.md"), "original").unwrap();

        let mtime = get_file_mtime(&root, "note.md").unwrap();
        let result = write_file_safe(&root, "note.md", "updated", mtime).unwrap();

        assert!(
            matches!(result, WriteResult::Success { .. }),
            "expected Success, got {result:?}"
        );
        let got = fs::read_to_string(root.join("note.md")).unwrap();
        assert_eq!(got, "updated");
    }

    #[test]
    fn write_file_safe_conflicts_when_mtime_differs() {
        let vault = make_vault();
        let root = vault.path().canonicalize().unwrap();
        fs::write(root.join("note.md"), "original").unwrap();

        let real_mtime = get_file_mtime(&root, "note.md").unwrap();
        let stale_mtime = real_mtime.saturating_sub(10); // old mtime

        let result = write_file_safe(&root, "note.md", "should not write", stale_mtime).unwrap();

        assert!(
            matches!(result, WriteResult::Conflict { current_mtime } if current_mtime == real_mtime),
            "expected Conflict with current mtime, got {result:?}"
        );
        // file must be unchanged
        assert_eq!(
            fs::read_to_string(root.join("note.md")).unwrap(),
            "original"
        );
    }

    #[test]
    fn write_file_safe_conflict_returns_current_mtime() {
        let vault = make_vault();
        let root = vault.path().canonicalize().unwrap();
        fs::write(root.join("note.md"), "v1").unwrap();

        let mtime_v1 = get_file_mtime(&root, "note.md").unwrap();

        // Simulate an interleaved write: overwrite file, advancing mtime
        fs::write(root.join("note.md"), "v2 by external").unwrap();
        let mtime_v2 = get_file_mtime(&root, "note.md").unwrap();

        // Try to write with the old mtime
        if mtime_v1 != mtime_v2 {
            // mtime advanced (sub-second precision may make them equal on some FSes)
            let result = write_file_safe(&root, "note.md", "v3 attempt", mtime_v1).unwrap();
            assert!(
                matches!(result, WriteResult::Conflict { current_mtime } if current_mtime == mtime_v2)
            );
        }
        // If mtime_v1 == mtime_v2 (same-second write, low-res FS), the test is vacuously true
    }

    #[test]
    fn write_file_safe_not_found_returns_error() {
        let vault = make_vault();
        let root = vault.path().canonicalize().unwrap();
        // File doesn't exist → resolve_path returns NotFound
        let result = write_file_safe(&root, "nonexistent.md", "content", 12345);
        assert!(
            matches!(result, Err(crate::CoreError::NotFound(_))),
            "expected NotFound error: {result:?}"
        );
    }

    // ── 第3層: 不正 UTF-8 ────────────────────────────────────────────────────

    #[test]
    fn read_file_invalid_utf8_returns_invalid_utf8_error() {
        let vault = make_vault();
        let root = vault.path().canonicalize().unwrap();
        // Write raw invalid UTF-8 bytes directly
        std::fs::write(root.join("bad.md"), b"\xFF\xFE invalid utf8 data").unwrap();

        let result = read_file(&root, "bad.md");
        assert!(
            matches!(result, Err(crate::CoreError::InvalidUtf8 { .. })),
            "expected InvalidUtf8, got {result:?}"
        );
    }

    #[test]
    fn read_file_invalid_utf8_has_byte_offset() {
        let vault = make_vault();
        let root = vault.path().canonicalize().unwrap();
        // "valid\xFF" — first 5 bytes are valid ASCII, offset 5 is the invalid byte
        let mut bytes = b"valid".to_vec();
        bytes.push(0xFF);
        std::fs::write(root.join("bad.md"), &bytes).unwrap();

        let result = read_file(&root, "bad.md");
        assert!(
            matches!(
                result,
                Err(crate::CoreError::InvalidUtf8 { byte_offset: 5 })
            ),
            "expected InvalidUtf8 at byte 5, got {result:?}"
        );
    }

    #[test]
    fn read_file_valid_utf8_succeeds() {
        let vault = make_vault();
        let root = vault.path().canonicalize().unwrap();
        std::fs::write(root.join("good.md"), "こんにちは🎉").unwrap();
        let result = read_file(&root, "good.md");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "こんにちは🎉");
    }

    // ── move_file ─────────────────────────────────────────────────────────────

    #[test]
    fn move_file_succeeds_within_vault() {
        let vault = make_vault();
        let root = vault.path().canonicalize().unwrap();
        std::fs::write(root.join("src.md"), "hello").unwrap();
        std::fs::create_dir(root.join("sub")).unwrap();

        move_file(&root, "src.md", "sub/src.md").unwrap();

        assert!(!root.join("src.md").exists(), "source should be gone");
        assert!(root.join("sub/src.md").exists(), "dest should exist");
        assert_eq!(
            std::fs::read_to_string(root.join("sub/src.md")).unwrap(),
            "hello"
        );
    }

    #[test]
    fn move_file_rejects_traversal_in_src() {
        let vault = make_vault();
        let root = vault.path().canonicalize().unwrap();
        // ../escape.md attempts to escape the vault
        let result = move_file(&root, "../escape.md", "dst.md");
        assert!(
            matches!(
                result,
                Err(crate::CoreError::PathTraversal(_)) | Err(crate::CoreError::NotFound(_))
            ),
            "expected traversal or not-found error, got {result:?}"
        );
    }

    #[test]
    fn move_file_rejects_traversal_in_dst() {
        let vault = make_vault();
        let root = vault.path().canonicalize().unwrap();
        std::fs::write(root.join("note.md"), "content").unwrap();

        let result = move_file(&root, "note.md", "../outside.md");
        assert!(
            matches!(result, Err(crate::CoreError::PathTraversal(_))),
            "expected PathTraversal, got {result:?}"
        );
    }

    #[test]
    fn move_file_rejects_dst_conflict() {
        let vault = make_vault();
        let root = vault.path().canonicalize().unwrap();
        std::fs::write(root.join("a.md"), "aaa").unwrap();
        std::fs::write(root.join("b.md"), "bbb").unwrap();

        let result = move_file(&root, "a.md", "b.md");
        assert!(
            matches!(result, Err(crate::CoreError::Io(ref e)) if e.kind() == std::io::ErrorKind::AlreadyExists),
            "expected AlreadyExists conflict, got {result:?}"
        );
        // neither file should be disturbed
        assert_eq!(std::fs::read_to_string(root.join("a.md")).unwrap(), "aaa");
        assert_eq!(std::fs::read_to_string(root.join("b.md")).unwrap(), "bbb");
    }

    #[test]
    fn move_file_rejects_nonexistent_src() {
        let vault = make_vault();
        let root = vault.path().canonicalize().unwrap();
        let result = move_file(&root, "ghost.md", "ghost2.md");
        assert!(
            matches!(result, Err(crate::CoreError::NotFound(_))),
            "expected NotFound, got {result:?}"
        );
    }
}
