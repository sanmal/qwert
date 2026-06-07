use crate::AppState;
use qwert_core::revision::{self, NamingStyle, RevisionRequest};
use qwert_core::vault;
use qwert_core::revision_diff;
use serde::Serialize;
use tauri::State;

// ── Response DTOs ─────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct AffectedFileDto {
    pub path: String,
    pub wikilink_count: usize,
}

#[derive(Serialize)]
pub struct RevisionPlanDto {
    pub old_name: String,
    pub new_name: String,
    pub old_path: String,
    pub new_path: String,
    pub affected_files: Vec<AffectedFileDto>,
    pub total_wikilinks: usize,
    /// Unified diff for all affected files (empty when no links change).
    pub diff: String,
}

#[derive(Serialize)]
pub struct RevisionResultDto {
    pub old_path: String,
    pub new_path: String,
    pub total_wikilinks: usize,
}

// ── Tauri commands ────────────────────────────────────────────────────────────

/// Dry-run: return the revision plan without applying it.
#[tauri::command]
pub fn plan_revision_note(
    path: String,
    naming: String,
    name: Option<String>,
    state: State<'_, AppState>,
) -> Result<RevisionPlanDto, String> {
    let root = {
        let lock = state.vault_root.lock().unwrap();
        lock.as_ref().ok_or("No vault open")?.to_path_buf()
    };

    // A4: validate path is inside the vault before use (resolve_path uses canonicalize).
    vault::resolve_path(&root, &path).map_err(|e| e.to_string())?;

    let naming_style = parse_naming(&naming);
    let date_str = if naming_style == NamingStyle::Date {
        Some(today_yyyymmdd())
    } else {
        None
    };

    let req = RevisionRequest {
        vault_root: root.clone(),
        source_rel_path: path,
        naming: naming_style,
        new_name: name,
        excluded_dirs: vec![],
        date_str,
    };

    let plan = revision::plan_revision(&req).map_err(|e| e.to_string())?;

    // Compute unified diff for all affected files.
    let diffs = revision_diff::compute_diffs_for_plan(&root, &plan).map_err(|e| e.to_string())?;
    let diff: String = diffs
        .iter()
        .filter_map(|d| revision_diff::generate_diff(d).ok())
        .collect();

    Ok(RevisionPlanDto {
        old_name: plan.old_name,
        new_name: plan.new_name,
        old_path: plan.old_path,
        new_path: plan.new_path,
        affected_files: plan
            .affected_files
            .into_iter()
            .map(|f| AffectedFileDto {
                path: f.path,
                wikilink_count: f.wikilink_count,
            })
            .collect(),
        total_wikilinks: plan.total_wikilinks,
        diff,
    })
}

/// Execute the revision atomically via WAL.
#[tauri::command]
pub fn execute_revision_note(
    path: String,
    naming: String,
    name: Option<String>,
    state: State<'_, AppState>,
) -> Result<RevisionResultDto, String> {
    let root = {
        let lock = state.vault_root.lock().unwrap();
        lock.as_ref().ok_or("No vault open")?.to_path_buf()
    };

    // A4: validate path is inside the vault before use.
    vault::resolve_path(&root, &path).map_err(|e| e.to_string())?;

    let naming_style = parse_naming(&naming);
    let date_str = if naming_style == NamingStyle::Date {
        Some(today_yyyymmdd())
    } else {
        None
    };

    let req = RevisionRequest {
        vault_root: root,
        source_rel_path: path,
        naming: naming_style,
        new_name: name,
        excluded_dirs: vec![],
        date_str,
    };

    let result = revision::execute_revision(&req).map_err(|e| e.to_string())?;

    Ok(RevisionResultDto {
        old_path: result.old_path,
        new_path: result.new_path,
        total_wikilinks: result.total_wikilinks,
    })
}

// ── Private helpers ───────────────────────────────────────────────────────────

fn parse_naming(s: &str) -> NamingStyle {
    match s {
        "date" => NamingStyle::Date,
        "semver" => NamingStyle::Semver,
        "manual" => NamingStyle::Manual,
        _ => NamingStyle::Increment,
    }
}

/// Returns today's date in YYYYMMDD format using only `std`.
fn today_yyyymmdd() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let z = (secs / 86400) as u32 + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}{m:02}{d:02}")
}
