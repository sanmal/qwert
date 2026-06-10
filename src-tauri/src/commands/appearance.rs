use crate::AppState;
use qwert_core::appearance::AppearanceWatchGuard;
use std::collections::HashMap;
use std::path::Path;
use tauri::{Emitter, State};

#[tauri::command]
pub fn load_appearance(state: State<'_, AppState>) -> HashMap<String, String> {
    // スコープ解決（vault があれば vault のみ、なければ global）は core に集約。
    // vault があればグローバルとはマージしない（二者択一）。
    let vault_root = state.vault_root.lock().unwrap().clone();
    let (config, _scope) =
        qwert_core::appearance::resolve_appearance(vault_root.as_deref()).unwrap_or_default();
    qwert_core::appearance::to_css_vars(&config).unwrap_or_default()
}

/// C2: vault の `.qwert/appearance.toml` を監視し、直接編集を 300ms debounce 後に
/// `appearance-changed` イベントで解決済み CSS 変数としてフロントへ emit する。
/// 返り値の guard を保持している間だけ監視が続く（drop で停止）。
pub fn watch_appearance(
    app: tauri::AppHandle,
    vault_root: &Path,
) -> Result<AppearanceWatchGuard, String> {
    qwert_core::appearance::watch_vault_appearance(vault_root, move |config| {
        let vars = qwert_core::appearance::to_css_vars(&config).unwrap_or_default();
        let _ = app.emit("appearance-changed", vars);
    })
    .map_err(|e| e.to_string())
}
