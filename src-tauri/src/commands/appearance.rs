use crate::AppState;
use qwert_core::appearance::{AppearanceStatus, AppearanceUpdate, AppearanceWatchGuard};
use std::collections::HashMap;
use std::path::Path;
use tauri::{Emitter, State};

/// C1/C3: CSS 変数マップを返す。vault config が壊れていればグローバルへフォールバックし
/// `appearance-warning` イベントを emit する（アプリは必ず起動する）。
#[tauri::command]
pub fn load_appearance(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> HashMap<String, String> {
    let vault_root = state.vault_root.lock().unwrap().clone();
    let res = qwert_core::appearance::resolve_appearance_with_fallback(vault_root.as_deref());
    if let Some(warning) = res.warning {
        let _ = app.emit("appearance-warning", warning);
    }
    qwert_core::appearance::to_css_vars(&res.config).unwrap_or_default()
}

/// C9: 解決済み appearance の contrast_ratio / level を返す。
/// フロントエンドの StatusBar は受け取った値をそのまま表示する（WCAG 式の TS 実装不要）。
#[tauri::command]
pub fn get_appearance_status(state: State<'_, AppState>) -> AppearanceStatus {
    let vault_root = state.vault_root.lock().unwrap().clone();
    qwert_core::appearance::compute_appearance_status(vault_root.as_deref())
}

/// C2/C3: vault の `.qwert/appearance.toml` を監視し、300ms debounce 後に:
/// - 成功 → `appearance-changed` に CSS 変数を emit。
/// - エラー → `appearance-warning` に警告文を emit（直前の見た目を維持）。
pub fn watch_appearance(
    app: tauri::AppHandle,
    vault_root: &Path,
) -> Result<AppearanceWatchGuard, String> {
    qwert_core::appearance::watch_vault_appearance(vault_root, move |update| match update {
        AppearanceUpdate::Changed(config) => {
            let vars = qwert_core::appearance::to_css_vars(&config).unwrap_or_default();
            let _ = app.emit("appearance-changed", vars);
        }
        AppearanceUpdate::Error(warning) => {
            let _ = app.emit("appearance-warning", warning);
        }
    })
    .map_err(|e| e.to_string())
}
