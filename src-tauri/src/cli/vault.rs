use std::path::Path;

use qwert_core::search;

use super::exit_code::ExitCode;
use super::format::{make_envelope, to_json_string, OutputFormat};

pub fn execute_search(
    query: &str,
    use_regex: bool,
    format: OutputFormat,
    vault_root: &Path,
) -> i32 {
    match search::search_vault(vault_root, query, use_regex) {
        Ok(hits) => {
            let total = hits.len();
            match format {
                OutputFormat::Path => {
                    let mut seen = std::collections::HashSet::new();
                    for h in &hits {
                        if seen.insert(&h.path) {
                            println!("{}", h.path);
                        }
                    }
                }
                OutputFormat::Json => {
                    let json_hits: Vec<serde_json::Value> = hits
                        .iter()
                        .map(|h| {
                            serde_json::json!({
                                "path": h.path,
                                "line": h.line,
                                "snippet": h.snippet,
                            })
                        })
                        .collect();
                    let v = make_envelope(
                        "search_results",
                        serde_json::json!({
                            "query": query,
                            "hits": json_hits,
                            "total_hits": total,
                        }),
                    );
                    println!("{}", to_json_string(&v));
                }
                OutputFormat::Text | OutputFormat::Raw => {
                    for h in &hits {
                        println!("{}:{}: {}", h.path, h.line, h.snippet);
                    }
                    eprintln!("{total} hit(s)");
                }
            }
            ExitCode::Success.as_i32()
        }
        Err(ref e) => super::emit_core_error(e),
    }
}

/// Stub: vault status report is implemented in a later task.
pub fn execute_status(format: OutputFormat, _vault_root: &Path) -> i32 {
    match format {
        OutputFormat::Json => {
            let v = make_envelope(
                "vault_status",
                serde_json::json!({ "sync_conflicts": 0, "pending_revisions": 0 }),
            );
            println!("{}", to_json_string(&v));
        }
        OutputFormat::Text | OutputFormat::Raw | OutputFormat::Path => {
            println!("vault status: ok");
        }
    }
    ExitCode::Success.as_i32()
}
