use crate::AppState;
use std::collections::HashMap;
use tauri::State;

#[tauri::command]
pub fn load_appearance(state: State<'_, AppState>) -> HashMap<String, String> {
    let config = qwert_core::appearance::load_global_appearance().unwrap_or_default();

    // vault スコープが存在すればそちらを優先（グローバルは無視）
    let vault_root = state.vault_root.lock().unwrap().clone();
    if let Some(root) = vault_root {
        if let Ok(Some(vault_cfg)) = qwert_core::appearance::load_vault_appearance(&root) {
            return qwert_core::appearance::to_css_vars(&vault_cfg).unwrap_or_default();
        }
    }

    qwert_core::appearance::to_css_vars(&config).unwrap_or_default()
}
