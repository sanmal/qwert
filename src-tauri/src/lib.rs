pub mod cli;
mod commands;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

pub struct AppState {
    pub vault_root: Mutex<Option<PathBuf>>,
    // B2: 直近の自己書き込み（vault 相対パス → 書き込み時刻）。watcher の自己トリガ抑制に使う。
    pub recent_writes: Arc<Mutex<HashMap<String, Instant>>>,
    // B3: 監視ハンドル。保持している間だけ監視が続く（t09 で設定）。
    pub watch_guard: Mutex<Option<qwert_core::vault::WatchGuard>>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            vault_root: Mutex::new(None),
            recent_writes: Arc::new(Mutex::new(HashMap::new())),
            watch_guard: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            commands::file::list_dir,
            commands::file::read_file,
            commands::file::write_file,
            commands::file::create_file,
            commands::vault::get_vault_root,
            commands::vault::open_vault_dialog,
            commands::vault::get_vault_status,
            commands::markdown::render_markdown,
            commands::appearance::load_appearance,
            commands::link::get_backlinks,
            commands::link::resolve_wikilink,
            commands::revision::plan_revision_note,
            commands::revision::execute_revision_note,
            commands::sanitize::scan_note,
            commands::sanitize::scan_vault_files,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
