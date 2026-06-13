use crate::AppState;
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
pub struct BacklinkEntry {
    pub path: String,
    pub wikilink_count: usize,
}

/// Return all vault files that link to the note identified by `path`.
/// Uses the in-memory `LinkIndex` cache (E1): warm calls skip file reads,
/// only re-reading files whose mtime changed since the last call.
#[tauri::command]
pub fn get_backlinks(
    path: String,
    state: State<'_, AppState>,
) -> Result<Vec<BacklinkEntry>, String> {
    let stem = std::path::Path::new(&path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(&path)
        .to_owned();

    let mut cache_lock = state.link_index.lock().unwrap();
    let cache = cache_lock.as_mut().ok_or("No vault open")?;
    let sources = cache.backlinks(&stem);

    Ok(sources
        .into_iter()
        .map(|s| BacklinkEntry {
            path: s.path,
            wikilink_count: s.wikilink_count,
        })
        .collect())
}

/// Resolve a wikilink target name to a vault-relative path.
/// Uses the `LinkIndex` cache (E1).
#[tauri::command]
pub fn resolve_wikilink(target: String, state: State<'_, AppState>) -> Option<String> {
    let mut cache_lock = state.link_index.lock().unwrap();
    let cache = cache_lock.as_mut()?;
    cache.resolve(&target)
}
