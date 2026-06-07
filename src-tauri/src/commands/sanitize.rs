use crate::AppState;
use qwert_core::{sanitize, vault};
use serde::Serialize;
use tauri::State;

/// JSON-serializable scan finding (code point as u32, category as string).
#[derive(Serialize)]
pub struct ScanFinding {
    pub line: usize,
    pub column: usize,
    pub char_code: u32,
    pub char_hex: String,
    pub category: &'static str,
}

/// Per-file result for vault-wide scan.
#[derive(Serialize)]
pub struct FileScanResult {
    pub path: String,
    pub findings: Vec<ScanFinding>,
}

fn to_dto(f: &sanitize::InvisibleCharFinding) -> ScanFinding {
    ScanFinding {
        line: f.line,
        column: f.column,
        char_code: f.char_value as u32,
        char_hex: f.char_hex(),
        category: f.category_str(),
    }
}

/// Scan a single file for invisible characters.
#[tauri::command]
pub fn scan_note(path: String, state: State<'_, AppState>) -> Result<Vec<ScanFinding>, String> {
    let root_lock = state.vault_root.lock().unwrap();
    let root = root_lock.as_ref().ok_or("No vault open")?;
    let content = vault::read_file(root, &path).map_err(|e| e.to_string())?;
    Ok(sanitize::detect_invisible_chars(&content)
        .iter()
        .map(to_dto)
        .collect())
}

/// Scan all .md files in the vault; return only files that have findings.
#[tauri::command]
pub fn scan_vault_files(state: State<'_, AppState>) -> Result<Vec<FileScanResult>, String> {
    let root_lock = state.vault_root.lock().unwrap();
    let root = root_lock.as_ref().ok_or("No vault open")?;

    let tree = vault::scan_vault(root).map_err(|e| e.to_string())?;
    let mut results = Vec::new();
    collect_scan_results(root, &tree, &mut results);
    Ok(results)
}

fn collect_scan_results(
    root: &std::path::Path,
    entries: &[vault::VaultEntry],
    out: &mut Vec<FileScanResult>,
) {
    for e in entries {
        if e.is_dir {
            if let Some(ch) = &e.children {
                collect_scan_results(root, ch, out);
            }
        } else if let Ok(content) = vault::read_file(root, &e.path) {
            let findings: Vec<ScanFinding> = sanitize::detect_invisible_chars(&content)
                .iter()
                .map(to_dto)
                .collect();
            if !findings.is_empty() {
                out.push(FileScanResult {
                    path: e.path.clone(),
                    findings,
                });
            }
        }
    }
}
