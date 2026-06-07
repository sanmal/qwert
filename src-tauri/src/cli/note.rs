use std::path::{Path, PathBuf};

use qwert_core::{link_index, markdown, vault};

use super::exit_code::ExitCode;
use super::format::{make_envelope, to_json_string, OutputFormat};

pub fn execute_render(path: &str, format: OutputFormat, vault_root: &Path) -> i32 {
    match vault::read_file(vault_root, path) {
        Ok(content) => {
            let html = markdown::render_markdown(&content);
            match format {
                OutputFormat::Raw | OutputFormat::Text => print!("{html}"),
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
    // Derive target stem from the given path (e.g. "specs/auth.md" → "auth")
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
                OutputFormat::Text | OutputFormat::Raw => {
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
