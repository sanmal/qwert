use crate::AppState;
use qwert_core::vault;
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub children: Option<Vec<FileEntry>>,
}

impl From<vault::VaultEntry> for FileEntry {
    fn from(e: vault::VaultEntry) -> Self {
        FileEntry {
            name: e.name,
            path: e.path,
            is_dir: e.is_dir,
            children: e
                .children
                .map(|ch| ch.into_iter().map(FileEntry::from).collect()),
        }
    }
}

#[tauri::command]
pub fn list_dir(state: State<'_, AppState>) -> Result<Vec<FileEntry>, String> {
    let root = state.vault_root.lock().unwrap();
    let root = root.as_ref().ok_or("No vault open")?;
    vault::scan_vault(root)
        .map(|entries| entries.into_iter().map(FileEntry::from).collect())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn read_file(path: String, state: State<'_, AppState>) -> Result<String, String> {
    let root = state.vault_root.lock().unwrap();
    let root = root.as_ref().ok_or("No vault open")?;
    vault::read_file(root, &path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn write_file(path: String, content: String, state: State<'_, AppState>) -> Result<(), String> {
    // B2: 書き込み前に記録し、watcher の自己トリガを抑制する（配線は t09）。
    //
    // C7 trust-boundary audit: if `path` is `.qwert/appearance.toml`, this raw
    // write is intentionally allowed — the C2 hot-reload watcher fires on the
    // saved file, calls `to_css_vars` (the Rust sanitizer), and emits only the
    // sanitized CSS-var map to the frontend. No raw TOML value ever reaches the
    // frontend directly. The worst-case outcome of a malicious write is an ugly
    // theme, not code execution or external resource loading.
    state
        .recent_writes
        .lock()
        .unwrap()
        .insert(path.clone(), std::time::Instant::now());

    let root = state.vault_root.lock().unwrap();
    let root = root.as_ref().ok_or("No vault open")?;
    vault::write_file(root, &path, &content).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_file(path: String, state: State<'_, AppState>) -> Result<(), String> {
    let root = state.vault_root.lock().unwrap();
    let root = root.as_ref().ok_or("No vault open")?;
    vault::create_file(root, &path).map_err(|e| e.to_string())
}

/// Move a file within the vault (pure file-system rename, no wikilink updates).
/// Semantics differ from Revision: this is structural reorganisation, not document revision.
/// Wikilinks remain valid because they resolve by file-stem, not by path.
#[tauri::command]
pub fn move_file(src: String, dst: String, state: State<'_, AppState>) -> Result<(), String> {
    let root = state.vault_root.lock().unwrap();
    let root = root.as_ref().ok_or("No vault open")?;
    vault::move_file(root, &src, &dst).map_err(|e| e.to_string())
}

/// Level 3 editing hint: record or clear the unsaved state for `path`.
/// Called by the frontend whenever saveState transitions between UNSAVED and SAVED.
#[tauri::command]
pub fn set_editing_state(
    path: String,
    is_editing: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let root = state.vault_root.lock().unwrap();
    let root = root.as_ref().ok_or("No vault open")?;
    vault::set_editing_path(root, &path, is_editing);
    Ok(())
}
