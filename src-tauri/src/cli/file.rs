use std::io::Read;
use std::path::Path;

use qwert_core::error::ActionableError;
use qwert_core::vault::{self, WriteResult};

use super::exit_code::ExitCode;
use super::format::{make_envelope, to_json_string, OutputFormat};
use super::tty::is_tty;

pub fn execute_read(path: &str, format: OutputFormat, vault_root: &Path) -> i32 {
    match vault::read_file_with_mtime(vault_root, path) {
        Ok((content, mtime)) => {
            match format {
                OutputFormat::Raw | OutputFormat::Text | OutputFormat::Diff => print!("{content}"),
                OutputFormat::Json => {
                    let v = make_envelope(
                        "file_content",
                        serde_json::json!({ "path": path, "content": content, "mtime": mtime }),
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

pub fn execute_write(
    path: &str,
    yes: bool,
    if_match: Option<u64>,
    format: OutputFormat,
    vault_root: &Path,
) -> i32 {
    if !yes && !is_tty() {
        eprintln!("error: non-interactive context requires --yes for destructive operations");
        return ExitCode::Usage.as_i32();
    }

    let mut content = String::new();
    if let Err(e) = std::io::stdin().read_to_string(&mut content) {
        let err = ActionableError::new("general", ExitCode::General as u8, e.to_string());
        eprintln!("{}", serde_json::to_string_pretty(&err).unwrap_or_default());
        return ExitCode::General.as_i32();
    }

    if let Some(expected_mtime) = if_match {
        // mtime 楽観ロック経路
        match vault::write_file_safe(vault_root, path, &content, expected_mtime) {
            Ok(WriteResult::Success { new_mtime }) => {
                if format == OutputFormat::Json {
                    let v = make_envelope(
                        "file_written",
                        serde_json::json!({ "path": path, "mtime": new_mtime }),
                    );
                    println!("{}", to_json_string(&v));
                }
                ExitCode::Success.as_i32()
            }
            Ok(WriteResult::Conflict { current_mtime }) => {
                let err = ActionableError::new(
                    ExitCode::Conflict.category_str(),
                    ExitCode::Conflict as u8,
                    format!(
                        "mtime conflict: file '{path}' was modified since mtime {expected_mtime} \
                         (current: {current_mtime})"
                    ),
                )
                .with_next_step(format!(
                    "Re-read with qwert file read {path} --format json, merge, then retry with \
                     --if-match {current_mtime}"
                ));
                eprintln!("{}", serde_json::to_string_pretty(&err).unwrap_or_default());
                ExitCode::Conflict.as_i32()
            }
            Err(ref e) => super::emit_core_error(e),
        }
    } else {
        // 従来の無条件書き込み経路
        match vault::write_file(vault_root, path, &content) {
            Ok(()) => {
                if format == OutputFormat::Json {
                    let v = make_envelope("file_written", serde_json::json!({ "path": path }));
                    println!("{}", to_json_string(&v));
                }
                ExitCode::Success.as_i32()
            }
            Err(ref e) => super::emit_core_error(e),
        }
    }
}

pub fn execute_list(format: OutputFormat, vault_root: &Path) -> i32 {
    match vault::scan_vault(vault_root) {
        Ok(entries) => {
            let mut paths = Vec::new();
            collect_paths(&entries, &mut paths);
            match format {
                OutputFormat::Path => {
                    for p in &paths {
                        println!("{p}");
                    }
                }
                OutputFormat::Json => {
                    let v = make_envelope(
                        "file_list",
                        serde_json::json!({ "paths": paths, "count": paths.len() }),
                    );
                    println!("{}", to_json_string(&v));
                }
                OutputFormat::Text | OutputFormat::Raw | OutputFormat::Diff => {
                    for p in &paths {
                        println!("{p}");
                    }
                }
            }
            ExitCode::Success.as_i32()
        }
        Err(ref e) => super::emit_core_error(e),
    }
}

fn collect_paths(entries: &[vault::VaultEntry], out: &mut Vec<String>) {
    for e in entries {
        if e.is_dir {
            if let Some(ch) = &e.children {
                collect_paths(ch, out);
            }
        } else {
            out.push(e.path.clone());
        }
    }
}
