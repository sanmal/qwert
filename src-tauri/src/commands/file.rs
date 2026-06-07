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
