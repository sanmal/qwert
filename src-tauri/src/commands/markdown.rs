#[tauri::command]
pub fn render_markdown(content: String) -> String {
    qwert_core::markdown::render_markdown(&content)
}
