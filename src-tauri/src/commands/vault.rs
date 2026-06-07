use crate::AppState;
use qwert_core::status;
use std::time::Duration;
use tauri::Emitter;
use tauri::State;

#[tauri::command]
pub fn get_vault_root(state: State<'_, AppState>) -> Option<String> {
    state
        .vault_root
        .lock()
        .unwrap()
        .as_ref()
        .map(|p| p.to_string_lossy().into_owned())
}

// A1: v2 の pick_folder はコールバック型。async コマンド内で blocking_pick_folder を使う。
#[tauri::command]
pub async fn open_vault_dialog(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let Some(fp) = app.dialog().file().blocking_pick_folder() else {
        return Ok(None);
    };
    // A1: FilePath → PathBuf は .into_path()（Result を返す）。
    let path = fp.into_path().map_err(|e| e.to_string())?;
    let canonical = path.canonicalize().map_err(|e| e.to_string())?;
    *state.vault_root.lock().unwrap() = Some(canonical.clone());

    // t09 申し送り: 前回クラッシュした Revision の WAL があれば自動ロールバック（非致命的）
    if let Err(e) = qwert_core::revision::rollback_pending(&canonical) {
        eprintln!("rollback_pending warning (non-fatal): {e}");
    }

    {
        let app_for_cb = app.clone();
        let recent = state.recent_writes.clone(); // Arc<Mutex<HashMap<String, Instant>>>
        let guard = qwert_core::vault::watch_vault(&canonical, move |relative: String| {
            // B2: 直近 1500ms 以内に自分が書いたパスなら自己トリガとみなして無視。
            // 1回の保存で notify が複数イベント（Create + Modify 等）を出し得るため、
            // 即削除せず「窓が切れるまで」抑制し、古いエントリは retain で掃除する。
            const SELF_WRITE_WINDOW: Duration = Duration::from_millis(1500);
            {
                let mut map = recent.lock().unwrap();
                map.retain(|_, t| t.elapsed() < SELF_WRITE_WINDOW); // 期限切れ掃除
                if map.contains_key(&relative) {
                    return; // 窓内の自己書き込み → 無視
                }
            }
            let _ = app_for_cb.emit("file-changed", relative);
        })
        .map_err(|e| e.to_string())?;

        // guard を保持（drop すると監視停止）。前の vault の guard はここで置き換えて落とす。
        *state.watch_guard.lock().unwrap() = Some(guard);
    }

    Ok(Some(canonical.to_string_lossy().into_owned()))
}

/// Return the current vault health status (sync-conflicts, pending WAL, etc.).
#[tauri::command]
pub fn get_vault_status(state: State<'_, AppState>) -> Result<status::VaultStatus, String> {
    let root_lock = state.vault_root.lock().unwrap();
    let root = root_lock.as_ref().ok_or("No vault open")?;
    status::check_vault_status(root).map_err(|e| e.to_string())
}
