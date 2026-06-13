use qwert_core::config::{self, KeybindingsConfig};
use tauri::State;

use crate::AppState;

#[tauri::command]
pub fn get_keybindings(_state: State<'_, AppState>) -> Result<KeybindingsConfig, String> {
    Ok(config::load_global_config().keybindings)
}

#[tauri::command]
pub fn save_keybindings(kb: KeybindingsConfig, _state: State<'_, AppState>) -> Result<(), String> {
    let fields = [
        ("save", &kb.save),
        ("new_note", &kb.new_note),
        ("command_palette", &kb.command_palette),
        ("full_search", &kb.full_search),
        ("view_mode_toggle", &kb.view_mode_toggle),
        ("sidebar_toggle", &kb.sidebar_toggle),
        ("settings", &kb.settings),
    ];
    for (name, spec) in &fields {
        if !config::is_valid_key_spec(spec) {
            return Err(format!("invalid key spec for {name}: {spec:?}"));
        }
    }
    let dups = config::duplicate_key_specs(&kb);
    if !dups.is_empty() {
        return Err(format!("duplicate key specs: {}", dups.join(", ")));
    }
    let mut full = config::load_global_config();
    full.keybindings = kb;
    config::save_global_config(&full).map_err(|e| e.to_string())
}
