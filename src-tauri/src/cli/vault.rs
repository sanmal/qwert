use std::path::Path;

use qwert_core::{sanitize, search, status, vault};

use super::exit_code::ExitCode;
use super::format::{make_envelope, to_json_string, tsv_row, OutputFormat};

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
                OutputFormat::Text | OutputFormat::Raw | OutputFormat::Diff => {
                    for h in &hits {
                        println!("{}:{}: {}", h.path, h.line, h.snippet);
                    }
                    eprintln!("{total} hit(s)");
                }
                OutputFormat::Tsv => {
                    println!("{}", tsv_row(["path", "line", "snippet"]));
                    for h in &hits {
                        println!(
                            "{}",
                            tsv_row([h.path.as_str(), &h.line.to_string(), h.snippet.as_str()])
                        );
                    }
                }
            }
            ExitCode::Success.as_i32()
        }
        Err(ref e) => super::emit_core_error(e),
    }
}

pub fn execute_status(format: OutputFormat, vault_root: &Path) -> i32 {
    match status::check_vault_status(vault_root) {
        Ok(s) => {
            match format {
                OutputFormat::Json => {
                    let payload = serde_json::to_value(&s).unwrap_or_default();
                    let v = make_envelope("vault_status", payload);
                    println!("{}", to_json_string(&v));
                }
                OutputFormat::Text
                | OutputFormat::Raw
                | OutputFormat::Diff
                | OutputFormat::Path
                | OutputFormat::Tsv => {
                    if s.healthy {
                        println!("vault status: ok");
                    } else {
                        println!("vault status: warnings");
                        for w in &s.warnings {
                            println!("  ! {w}");
                        }
                    }
                }
            }
            // vault-level state は exit code に昇格させない（§9）
            ExitCode::Success.as_i32()
        }
        Err(ref e) => super::emit_core_error(e),
    }
}

pub fn execute_scan(format: OutputFormat, vault_root: &Path) -> i32 {
    let tree = match vault::scan_vault(vault_root) {
        Ok(t) => t,
        Err(ref e) => return super::emit_core_error(e),
    };

    let mut all_findings: Vec<(String, Vec<sanitize::InvisibleCharFinding>)> = Vec::new();
    collect_findings(vault_root, &tree, &mut all_findings);

    let total: usize = all_findings.iter().map(|(_, f)| f.len()).sum();

    match format {
        OutputFormat::Path => {
            for (path, _) in &all_findings {
                println!("{path}");
            }
        }
        OutputFormat::Json => {
            let items: Vec<serde_json::Value> = all_findings
                .iter()
                .flat_map(|(path, findings)| {
                    findings.iter().map(move |f| {
                        serde_json::json!({
                            "path": path,
                            "line": f.line,
                            "column": f.column,
                            "char_code": f.char_value as u32,
                            "char_hex": f.char_hex(),
                            "category": f.category_str(),
                        })
                    })
                })
                .collect();
            let v = make_envelope(
                "vault_scan_result",
                serde_json::json!({
                    "findings": items,
                    "files_with_findings": all_findings.len(),
                    "total": total,
                }),
            );
            println!("{}", to_json_string(&v));
        }
        OutputFormat::Text | OutputFormat::Raw | OutputFormat::Diff => {
            if all_findings.is_empty() {
                println!("vault scan: ok");
            } else {
                for (path, findings) in &all_findings {
                    for f in findings {
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
                eprintln!("{total} finding(s) in {} file(s)", all_findings.len());
            }
        }
        OutputFormat::Tsv => {
            println!(
                "{}",
                tsv_row([
                    "path",
                    "line",
                    "column",
                    "char_code",
                    "char_hex",
                    "category"
                ])
            );
            for (path, findings) in &all_findings {
                for f in findings {
                    println!(
                        "{}",
                        tsv_row([
                            path.as_str(),
                            &f.line.to_string(),
                            &f.column.to_string(),
                            &(f.char_value as u32).to_string(),
                            &f.char_hex(),
                            f.category_str(),
                        ])
                    );
                }
            }
        }
    }
    ExitCode::Success.as_i32()
}

fn collect_findings(
    vault_root: &Path,
    entries: &[vault::VaultEntry],
    out: &mut Vec<(String, Vec<sanitize::InvisibleCharFinding>)>,
) {
    for e in entries {
        if e.is_dir {
            if let Some(ch) = &e.children {
                collect_findings(vault_root, ch, out);
            }
        } else if let Ok(content) = vault::read_file(vault_root, &e.path) {
            let findings = sanitize::detect_invisible_chars(&content);
            if !findings.is_empty() {
                out.push((e.path.clone(), findings));
            }
        }
    }
}
