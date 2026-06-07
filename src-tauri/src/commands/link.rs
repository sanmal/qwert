use crate::AppState;
use qwert_core::link_index;
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
pub struct BacklinkEntry {
    pub path: String,
    pub wikilink_count: usize,
}

/// Return all vault files that link to the note identified by `path`.
/// The target stem is derived from `path` (e.g. "specs/auth.md" → "auth").
#[tauri::command]
pub fn get_backlinks(
    path: String,
    state: State<'_, AppState>,
) -> Result<Vec<BacklinkEntry>, String> {
    let root_lock = state.vault_root.lock().unwrap();
    let root = root_lock.as_ref().ok_or("No vault open")?;

    let stem = std::path::Path::new(&path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(&path)
        .to_owned();

    let sources = link_index::build_backlinks(root, &stem).map_err(|e| e.to_string())?;

    Ok(sources
        .into_iter()
        .map(|s| BacklinkEntry {
            path: s.path,
            wikilink_count: s.wikilink_count,
        })
        .collect())
}

/// Resolve a wikilink target name to a vault-relative path using core normalization
/// (NFC + case-insensitive).  Returns `null` when the target cannot be found.
#[tauri::command]
pub fn resolve_wikilink(target: String, state: State<'_, AppState>) -> Option<String> {
    let root_lock = state.vault_root.lock().unwrap();
    let root = root_lock.as_ref()?;
    link_index::resolve_link_to_path(root, &target)
}
