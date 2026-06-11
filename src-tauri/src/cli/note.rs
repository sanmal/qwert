use std::path::{Path, PathBuf};

use qwert_core::revision::{NamingStyle, RevisionRequest};
use qwert_core::{link_index, markdown, revision, revision_diff, sanitize, vault};

use super::exit_code::ExitCode;
use super::format::{make_envelope, to_json_string, OutputFormat};

pub fn execute_render(path: &str, format: OutputFormat, vault_root: &Path) -> i32 {
    match vault::read_file(vault_root, path) {
        Ok(content) => {
            let html = markdown::render_markdown(&content);
            match format {
                OutputFormat::Raw | OutputFormat::Text | OutputFormat::Diff => print!("{html}"),
                OutputFormat::Json => {
                    let v = make_envelope(
                        "note_render",
                        serde_json::json!({ "path": path, "html": html }),
                    );
                    println!("{}", to_json_string(&v));
                }
                OutputFormat::Path => println!("{path}"),
            }
            ExitCode::Success.as_i32()
        }
        Err(ref e) => super::emit_core_error(e),
    }
}

pub fn execute_backlinks(path: &str, format: OutputFormat, vault_root: &Path) -> i32 {
    let stem = PathBuf::from(path)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_owned());

    match link_index::build_backlinks(vault_root, &stem) {
        Ok(sources) => {
            let total: usize = sources.iter().map(|s| s.wikilink_count).sum();
            match format {
                OutputFormat::Path => {
                    for s in &sources {
                        println!("{}", s.path);
                    }
                }
                OutputFormat::Json => {
                    let items: Vec<serde_json::Value> = sources
                        .iter()
                        .map(|s| {
                            serde_json::json!({
                                "path": s.path,
                                "wikilink_count": s.wikilink_count,
                            })
                        })
                        .collect();
                    let v = make_envelope(
                        "note_backlinks",
                        serde_json::json!({
                            "path": path,
                            "backlinks": items,
                            "count": sources.len(),
                            "total_wikilinks": total,
                        }),
                    );
                    println!("{}", to_json_string(&v));
                }
                OutputFormat::Text | OutputFormat::Raw | OutputFormat::Diff => {
                    if sources.is_empty() {
                        println!("No backlinks found for '{stem}'");
                    } else {
                        for s in &sources {
                            println!("{} ({} link(s))", s.path, s.wikilink_count);
                        }
                    }
                }
            }
            ExitCode::Success.as_i32()
        }
        Err(ref e) => super::emit_core_error(e),
    }
}

pub struct RevisionArgs {
    pub path: String,
    pub dry_run: bool,
    pub diff_flag: bool,
    pub format: OutputFormat,
    /// `None` means "read from config.revision.naming".
    pub naming: Option<NamingStyle>,
    pub name: Option<String>,
    pub yes: bool,
}

/// `note revision` handler.
///
/// Modes:
/// - `--format diff`: print unified diff to stdout (implies dry-run).
/// - `--dry-run [--diff]`: print plan JSON; if `--diff` also write diff to a temp file
///   and add `diff_path` to the JSON.
/// - real execution: TTY + confirm_before_execute → prompt; --yes → skip prompt.
pub fn execute_revision(args: RevisionArgs, vault_root: &Path) -> i32 {
    let config = qwert_core::config::load_global_config();
    let RevisionArgs {
        path,
        dry_run,
        diff_flag,
        format,
        naming,
        name,
        yes,
    } = args;

    // Resolve naming: use config default when not explicitly specified.
    let naming = match naming {
        Some(n) => n,
        None => match naming_style_from_str(&config.revision.naming) {
            Ok(n) => n,
            Err(code) => return code,
        },
    };

    let date_str = if naming == NamingStyle::Date {
        Some(today_yyyymmdd())
    } else {
        None
    };

    let req = RevisionRequest {
        vault_root: vault_root.to_path_buf(),
        source_rel_path: path.to_owned(),
        naming,
        new_name: name,
        excluded_dirs: config.revision.excluded_dirs.clone(),
        date_str,
    };

    // Diff-only output (implies dry-run) — no confirmation needed.
    if format == OutputFormat::Diff {
        return dry_run_diff_to_stdout(&req, vault_root);
    }

    if dry_run {
        return dry_run_json(&req, vault_root, diff_flag, format);
    }

    // Real execution: TTY / non-TTY / confirm guard.
    let is_tty = super::tty::is_tty();
    if !yes && !is_tty {
        // Non-interactive context: --yes required.
        let err = qwert_core::error::ActionableError::new(
            "usage",
            ExitCode::Usage as u8,
            "Non-interactive context: --yes is required to apply revision",
        )
        .with_next_step("Add --yes flag, or use --dry-run to preview");
        eprintln!("{}", serde_json::to_string_pretty(&err).unwrap_or_default());
        return ExitCode::Usage.as_i32();
    }
    if should_prompt(is_tty, yes, config.revision.confirm_before_execute) {
        match prompt_confirm(&req) {
            Ok(true) => {}
            Ok(false) => {
                eprintln!("Aborted.");
                return ExitCode::Success.as_i32();
            }
            Err(code) => return code,
        }
    }

    // Execute.
    match revision::execute_revision(&req) {
        Ok(result) => {
            match format {
                OutputFormat::Json => {
                    let affected: Vec<serde_json::Value> = result
                        .affected_files
                        .iter()
                        .map(|f| {
                            serde_json::json!({
                                "path": f.path,
                                "wikilink_count": f.wikilink_count,
                            })
                        })
                        .collect();
                    let v = make_envelope(
                        "revision_result",
                        serde_json::json!({
                            "old_path": result.old_path,
                            "new_path": result.new_path,
                            "affected_files": affected,
                            "total_wikilinks": result.total_wikilinks,
                        }),
                    );
                    println!("{}", to_json_string(&v));
                }
                _ => {
                    println!("Revision: {} → {}", result.old_path, result.new_path);
                    println!(
                        "Updated {} wikilink(s) in {} file(s)",
                        result.total_wikilinks,
                        result.affected_files.len()
                    );
                }
            }
            ExitCode::Success.as_i32()
        }
        Err(ref e) => super::emit_core_error(e),
    }
}

pub fn execute_scan(path: &str, format: OutputFormat, vault_root: &Path) -> i32 {
    let content = match vault::read_file(vault_root, path) {
        Ok(c) => c,
        Err(ref e) => return super::emit_core_error(e),
    };
    let findings = sanitize::detect_invisible_chars(&content);
    let total = findings.len();

    match format {
        OutputFormat::Json => {
            let items: Vec<serde_json::Value> = findings
                .iter()
                .map(|f| {
                    serde_json::json!({
                        "line": f.line,
                        "column": f.column,
                        "char_code": f.char_value as u32,
                        "char_hex": f.char_hex(),
                        "category": f.category_str(),
                    })
                })
                .collect();
            let v = make_envelope(
                "scan_result",
                serde_json::json!({ "path": path, "findings": items, "total": total }),
            );
            println!("{}", to_json_string(&v));
        }
        OutputFormat::Path => {
            if total > 0 {
                println!("{path}");
            }
        }
        OutputFormat::Text | OutputFormat::Raw | OutputFormat::Diff => {
            if findings.is_empty() {
                println!("{path}: ok");
            } else {
                for f in &findings {
                    println!(
                        "{}:{}:{}: {} ({})",
                        path,
                        f.line,
                        f.column,
                        f.category_str(),
                        f.char_hex()
                    );
                }
            }
        }
    }
    ExitCode::Success.as_i32()
}

// ── Private helpers ───────────────────────────────────────────────────────────

/// Returns true when an interactive confirmation prompt should be shown.
fn should_prompt(is_tty: bool, yes: bool, confirm_cfg: bool) -> bool {
    !yes && is_tty && confirm_cfg
}

/// Show a plan summary and ask `Execute? [y/N]`. Returns `Ok(true)` on `y`/`Y`.
fn prompt_confirm(req: &RevisionRequest) -> Result<bool, i32> {
    let plan = match revision::plan_revision(req) {
        Ok(p) => p,
        Err(ref e) => return Err(super::emit_core_error(e)),
    };
    println!("Revision: {} → {}", plan.old_path, plan.new_path);
    println!(
        "Affects: {} file(s) ({} wikilink(s))",
        plan.affected_files.len(),
        plan.total_wikilinks
    );
    use std::io::Write as _;
    print!("Execute? [y/N] ");
    std::io::stdout().flush().ok();
    let mut line = String::new();
    std::io::stdin().read_line(&mut line).ok();
    Ok(matches!(line.trim(), "y" | "Y"))
}

/// Convert a naming style string to NamingStyle; exit 5 on unknown values.
fn naming_style_from_str(s: &str) -> Result<NamingStyle, i32> {
    match s {
        "increment" => Ok(NamingStyle::Increment),
        "date" => Ok(NamingStyle::Date),
        "semver" => Ok(NamingStyle::Semver),
        "manual" => Ok(NamingStyle::Manual),
        _ => {
            let err = qwert_core::error::ActionableError::new(
                "validation",
                ExitCode::Validation as u8,
                format!("Unknown naming style: {s}"),
            )
            .with_next_step("Use one of: increment | date | semver | manual");
            eprintln!("{}", serde_json::to_string_pretty(&err).unwrap_or_default());
            Err(ExitCode::Validation.as_i32())
        }
    }
}

fn dry_run_diff_to_stdout(req: &RevisionRequest, vault_root: &Path) -> i32 {
    let plan = match revision::plan_revision(req) {
        Ok(p) => p,
        Err(ref e) => return super::emit_core_error(e),
    };
    let diffs = match revision_diff::compute_diffs_for_plan(vault_root, &plan) {
        Ok(d) => d,
        Err(ref e) => return super::emit_core_error(e),
    };
    for d in &diffs {
        match revision_diff::generate_diff(d) {
            Ok(s) => print!("{s}"),
            Err(ref e) => return super::emit_core_error(e),
        }
    }
    ExitCode::Success.as_i32()
}

fn dry_run_json(
    req: &RevisionRequest,
    vault_root: &Path,
    diff_flag: bool,
    format: OutputFormat,
) -> i32 {
    let plan = match revision::plan_revision(req) {
        Ok(p) => p,
        Err(ref e) => return super::emit_core_error(e),
    };

    if format == OutputFormat::Json || diff_flag {
        let mut v = serde_json::to_value(&plan).unwrap_or_default();

        if diff_flag {
            match build_diff_file(&plan, vault_root) {
                Ok(diff_path) => {
                    if let Some(obj) = v.as_object_mut() {
                        obj.insert("diff_path".into(), serde_json::Value::String(diff_path));
                    }
                }
                Err(code) => return code,
            }
        }

        println!("{}", to_json_string(&v));
    } else {
        // Text output
        println!("Revision: {} → {}", plan.old_path, plan.new_path);
        println!("Affected files: {}", plan.affected_files.len());
        println!("Total wikilinks: {}", plan.total_wikilinks);
        if !plan.affected_files.is_empty() {
            for f in &plan.affected_files {
                println!("  {} ({} link(s))", f.path, f.wikilink_count);
            }
        }
    }

    ExitCode::Success.as_i32()
}

fn build_diff_file(
    plan: &qwert_core::revision::RevisionPlan,
    vault_root: &Path,
) -> Result<String, i32> {
    let diffs = revision_diff::compute_diffs_for_plan(vault_root, plan)
        .map_err(|e| super::emit_core_error(&e))?;

    let mut combined = String::new();
    for d in &diffs {
        let s = revision_diff::generate_diff(d).map_err(|e| super::emit_core_error(&e))?;
        combined.push_str(&s);
    }

    revision_diff::write_diff_to_tempfile(&combined).map_err(|e| super::emit_core_error(&e))
}

/// Returns today's date in YYYYMMDD format using only `std`.
pub(crate) fn today_yyyymmdd() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (y, m, d) = days_to_ymd((secs / 86400) as u32);
    format!("{y:04}{m:02}{d:02}")
}

/// Convert days since 1970-01-01 to (year, month, day) using the Gregorian calendar.
fn days_to_ymd(z: u32) -> (u32, u32, u32) {
    let z = z + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── should_prompt ─────────────────────────────────────────────────────────

    #[test]
    fn should_prompt_tty_no_yes_confirm_true() {
        assert!(should_prompt(true, false, true));
    }

    #[test]
    fn should_prompt_yes_flag_suppresses() {
        assert!(!should_prompt(true, true, true));
    }

    #[test]
    fn should_prompt_non_tty_suppresses() {
        assert!(!should_prompt(false, false, true));
    }

    #[test]
    fn should_prompt_confirm_cfg_false_suppresses() {
        assert!(!should_prompt(true, false, false));
    }

    // ── today_yyyymmdd ────────────────────────────────────────────────────────

    #[test]
    fn today_yyyymmdd_has_8_digits() {
        let s = today_yyyymmdd();
        assert_eq!(s.len(), 8, "expected YYYYMMDD: {s}");
        assert!(s.chars().all(|c| c.is_ascii_digit()), "must be digits: {s}");
    }

    #[test]
    fn days_to_ymd_epoch() {
        assert_eq!(days_to_ymd(0), (1970, 1, 1));
    }

    #[test]
    fn days_to_ymd_known_date() {
        // 2026-06-07 = 20611 days since epoch (1970-01-01).
        // Days to 2026-01-01: 56×365 + 14 leap-years = 20454.
        // Days from Jan-1 to Jun-7: 31+28+31+30+31 + 6 (offset within June) = 157.
        // Total: 20454 + 157 = 20611.
        let (y, m, d) = days_to_ymd(20611);
        assert_eq!(y, 2026);
        assert_eq!(m, 6);
        assert_eq!(d, 7);
    }

    // ── dry_run_json output ───────────────────────────────────────────────────

    #[test]
    fn dry_run_produces_revision_plan_json_shape() {
        use std::fs;
        use tempfile::TempDir;

        let tmp = TempDir::new().unwrap();
        let root = tmp.path().canonicalize().unwrap();
        fs::write(root.join("auth.md"), "# Auth\n").unwrap();
        fs::write(root.join("index.md"), "[[auth]] ref\n").unwrap();

        let req = RevisionRequest {
            vault_root: root.clone(),
            source_rel_path: "auth.md".into(),
            naming: NamingStyle::Increment,
            new_name: None,
            excluded_dirs: vec![],
            date_str: None,
        };

        let plan = qwert_core::revision::plan_revision(&req).unwrap();
        let v = serde_json::to_value(&plan).unwrap();

        // Envelope fields
        assert_eq!(v["schema_version"], "v1");
        assert_eq!(v["kind"], "revision_plan");
        assert!(v.get("data").is_none(), "data wrapper must not exist");

        // Payload fields at top level
        assert_eq!(v["old_name"], "auth");
        assert_eq!(v["new_name"], "auth_2");
        assert_eq!(v["old_path"], "auth.md");
        assert_eq!(v["new_path"], "auth_2.md");
        assert_eq!(v["dry_run"], true);
        assert_eq!(v["total_wikilinks"], 1);
        assert!(v["affected_files"].is_array());
    }

    #[test]
    fn execute_revision_json_result_shape() {
        use std::fs;
        use tempfile::TempDir;

        let tmp = TempDir::new().unwrap();
        let root = tmp.path().canonicalize().unwrap();
        fs::write(root.join("auth.md"), "# Auth\n").unwrap();
        fs::write(root.join("index.md"), "[[auth]] ref\n").unwrap();

        let req = RevisionRequest {
            vault_root: root.clone(),
            source_rel_path: "auth.md".into(),
            naming: NamingStyle::Increment,
            new_name: None,
            excluded_dirs: vec![],
            date_str: None,
        };

        let result = qwert_core::revision::execute_revision(&req).unwrap();
        let v = serde_json::json!({
            "old_path": result.old_path,
            "new_path": result.new_path,
            "total_wikilinks": result.total_wikilinks,
            "affected_files": result.affected_files.iter().map(|f| serde_json::json!({
                "path": f.path,
                "wikilink_count": f.wikilink_count,
            })).collect::<Vec<_>>(),
        });

        assert_eq!(v["old_path"], "auth.md");
        assert_eq!(v["new_path"], "auth_2.md");
        assert_eq!(v["total_wikilinks"], 1);
    }

    // ── excluded_dirs ─────────────────────────────────────────────────────────

    #[test]
    fn plan_revision_respects_excluded_dirs() {
        use std::fs;
        use tempfile::TempDir;

        let tmp = TempDir::new().unwrap();
        let root = tmp.path().canonicalize().unwrap();
        fs::create_dir(root.join("archive")).unwrap();
        fs::write(root.join("auth.md"), "# Auth\n").unwrap();
        fs::write(root.join("index.md"), "[[auth]] ref\n").unwrap();
        fs::write(root.join("archive").join("old.md"), "[[auth]] archived\n").unwrap();

        let req = RevisionRequest {
            vault_root: root.clone(),
            source_rel_path: "auth.md".into(),
            naming: NamingStyle::Increment,
            new_name: None,
            excluded_dirs: vec!["archive".to_owned()],
            date_str: None,
        };

        let plan = qwert_core::revision::plan_revision(&req).unwrap();
        let paths: Vec<&str> = plan
            .affected_files
            .iter()
            .map(|f| f.path.as_str())
            .collect();
        assert!(paths.contains(&"index.md"), "index.md must be in plan");
        assert!(
            !paths.iter().any(|p| p.starts_with("archive/")),
            "archive/ must be excluded from plan: {paths:?}"
        );
    }

    #[test]
    fn execute_revision_respects_excluded_dirs() {
        use std::fs;
        use tempfile::TempDir;

        let tmp = TempDir::new().unwrap();
        let root = tmp.path().canonicalize().unwrap();
        fs::create_dir(root.join("archive")).unwrap();
        fs::write(root.join("auth.md"), "# Auth\n").unwrap();
        fs::write(root.join("index.md"), "[[auth]] ref\n").unwrap();
        fs::write(root.join("archive").join("old.md"), "[[auth]] archived\n").unwrap();

        let req = RevisionRequest {
            vault_root: root.clone(),
            source_rel_path: "auth.md".into(),
            naming: NamingStyle::Increment,
            new_name: None,
            excluded_dirs: vec!["archive".to_owned()],
            date_str: None,
        };

        let result = qwert_core::revision::execute_revision(&req).unwrap();
        assert_eq!(result.new_path, "auth_2.md");

        // archived file must NOT be updated
        let archived = fs::read_to_string(root.join("archive").join("old.md")).unwrap();
        assert!(
            archived.contains("[[auth]]"),
            "archive/old.md must not be updated: {archived}"
        );

        // included file must be updated
        let index = fs::read_to_string(root.join("index.md")).unwrap();
        assert!(
            index.contains("[[auth_2]]"),
            "index.md must be updated: {index}"
        );
    }

    // ── conflict exit code ────────────────────────────────────────────────────

    #[test]
    fn already_exists_error_maps_to_conflict() {
        use super::super::exit_code::ExitCode;
        use qwert_core::CoreError;

        let e = CoreError::Io(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "target exists",
        ));
        assert_eq!(ExitCode::from(&e), ExitCode::Conflict);
        assert_eq!(ExitCode::from(&e).as_i32(), 4);
    }
}
