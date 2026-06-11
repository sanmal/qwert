use std::path::PathBuf;

use rmcp::{
    ServerHandler, ServiceExt,
    handler::server::wrapper::Parameters,
    schemars, tool, tool_handler, tool_router,
};
use serde::Deserialize;

use crate::cli::exit_code::ExitCode;
use crate::cli::format::{make_envelope, to_json_string};

// ── Parameter structs ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct FileReadParams {
    /// Vault-relative path to the file.
    path: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct FileWriteParams {
    /// Vault-relative path to the file.
    path: String,
    /// File contents to write.
    content: String,
    /// Optimistic mtime lock (Unix seconds from file_read). Reject with conflict if file changed.
    #[serde(default)]
    if_match: Option<u64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct FileListParams {
    /// Return directory tree structure instead of flat list.
    #[serde(default)]
    tree: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct NotePathParams {
    /// Vault-relative path to the note.
    path: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct NoteRevisionParams {
    /// Vault-relative path of the note to revise.
    path: String,
    /// When true (default), return the revision plan without modifying files.
    /// Set false to execute the rename and update all wikilink references.
    #[serde(default = "default_true")]
    dry_run: bool,
    /// Naming style: increment (default) | date | semver | manual.
    naming: Option<String>,
    /// Explicit new name (required when naming = "manual").
    name: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct VaultSearchParams {
    /// Search query string.
    query: String,
    /// Treat query as a regex pattern.
    #[serde(default)]
    regex: bool,
}

#[derive(Debug, Default, Deserialize, schemars::JsonSchema)]
struct NoParams {}

fn default_true() -> bool {
    true
}

// ── Error helper ──────────────────────────────────────────────────────────────

fn core_error_json(e: &qwert_core::CoreError) -> String {
    let code = ExitCode::from(e);
    to_json_string(&make_envelope(
        "error",
        serde_json::json!({
            "category": code.category_str(),
            "exit_code": code as u8,
            "message": e.to_string(),
        }),
    ))
}

// ── Tree helpers ──────────────────────────────────────────────────────────────

fn collect_paths(entries: &[qwert_core::vault::VaultEntry], out: &mut Vec<String>) {
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

fn entry_to_json(e: &qwert_core::vault::VaultEntry) -> serde_json::Value {
    if e.is_dir {
        let children: Vec<serde_json::Value> = e
            .children
            .as_deref()
            .unwrap_or(&[])
            .iter()
            .map(entry_to_json)
            .collect();
        serde_json::json!({
            "name": e.name, "path": e.path, "type": "dir", "children": children
        })
    } else {
        serde_json::json!({ "name": e.name, "path": e.path, "type": "file" })
    }
}

fn collect_vault_findings(
    vault_root: &std::path::Path,
    entries: &[qwert_core::vault::VaultEntry],
    out: &mut Vec<(String, Vec<qwert_core::sanitize::InvisibleCharFinding>)>,
) {
    for e in entries {
        if e.is_dir {
            if let Some(ch) = &e.children {
                collect_vault_findings(vault_root, ch, out);
            }
        } else if let Ok(content) = qwert_core::vault::read_file(vault_root, &e.path) {
            let findings = qwert_core::sanitize::detect_invisible_chars(&content);
            if !findings.is_empty() {
                out.push((e.path.clone(), findings));
            }
        }
    }
}

// ── Server ────────────────────────────────────────────────────────────────────

pub struct QwertMcpServer {
    vault_root: PathBuf,
}

#[tool_router]
impl QwertMcpServer {
    pub fn new(vault_root: PathBuf) -> Self {
        Self { vault_root }
    }

    // ── file ──────────────────────────────────────────────────────────────────

    #[tool(description = "Read a file from the vault. Returns content and mtime (Unix seconds).")]
    fn file_read(&self, Parameters(p): Parameters<FileReadParams>) -> String {
        match qwert_core::vault::read_file_with_mtime(&self.vault_root, &p.path) {
            Ok((content, mtime)) => to_json_string(&make_envelope(
                "file_content",
                serde_json::json!({ "path": p.path, "content": content, "mtime": mtime }),
            )),
            Err(ref e) => core_error_json(e),
        }
    }

    #[tool(description = "Write content to a file in the vault (atomic). Provide if_match (mtime from file_read) for safe concurrent writes.")]
    fn file_write(&self, Parameters(p): Parameters<FileWriteParams>) -> String {
        if let Some(expected) = p.if_match {
            return match qwert_core::vault::write_file_safe(
                &self.vault_root,
                &p.path,
                &p.content,
                expected,
            ) {
                Ok(qwert_core::vault::WriteResult::Success { new_mtime }) => {
                    to_json_string(&make_envelope(
                        "file_written",
                        serde_json::json!({ "path": p.path, "mtime": new_mtime }),
                    ))
                }
                Ok(qwert_core::vault::WriteResult::Conflict { current_mtime }) => {
                    to_json_string(&make_envelope(
                        "error",
                        serde_json::json!({
                            "category": "conflict",
                            "exit_code": ExitCode::Conflict as u8,
                            "message": format!(
                                "mtime conflict on '{}': expected {expected}, current {current_mtime}",
                                p.path
                            ),
                            "next_step": format!(
                                "Re-read with file_read path={}, use the returned mtime in if_match",
                                p.path
                            ),
                        }),
                    ))
                }
                Err(ref e) => core_error_json(e),
            };
        }
        match qwert_core::vault::write_file(&self.vault_root, &p.path, &p.content) {
            Ok(()) => to_json_string(&make_envelope(
                "file_written",
                serde_json::json!({ "path": p.path }),
            )),
            Err(ref e) => core_error_json(e),
        }
    }

    #[tool(description = "List .md files in the vault. Set tree=true for a directory tree.")]
    fn file_list(&self, Parameters(p): Parameters<FileListParams>) -> String {
        match qwert_core::vault::scan_vault(&self.vault_root) {
            Ok(entries) => {
                if p.tree {
                    let nodes: Vec<serde_json::Value> =
                        entries.iter().map(entry_to_json).collect();
                    to_json_string(&make_envelope(
                        "file_tree",
                        serde_json::json!({ "tree": nodes }),
                    ))
                } else {
                    let mut paths = Vec::new();
                    collect_paths(&entries, &mut paths);
                    to_json_string(&make_envelope(
                        "file_list",
                        serde_json::json!({ "paths": paths, "count": paths.len() }),
                    ))
                }
            }
            Err(ref e) => core_error_json(e),
        }
    }

    // ── note ──────────────────────────────────────────────────────────────────

    #[tool(description = "Render a Markdown note to HTML.")]
    fn note_render(&self, Parameters(p): Parameters<NotePathParams>) -> String {
        match qwert_core::vault::read_file(&self.vault_root, &p.path) {
            Ok(content) => {
                let html = qwert_core::markdown::render_markdown(&content);
                to_json_string(&make_envelope(
                    "note_render",
                    serde_json::json!({ "path": p.path, "html": html }),
                ))
            }
            Err(ref e) => core_error_json(e),
        }
    }

    #[tool(description = "Show all notes that link to the given note (backlinks).")]
    fn note_backlinks(&self, Parameters(p): Parameters<NotePathParams>) -> String {
        let stem = std::path::PathBuf::from(&p.path)
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| p.path.clone());
        match qwert_core::link_index::build_backlinks(&self.vault_root, &stem) {
            Ok(sources) => {
                let total: usize = sources.iter().map(|s| s.wikilink_count).sum();
                let items: Vec<serde_json::Value> = sources
                    .iter()
                    .map(|s| {
                        serde_json::json!({
                            "path": s.path,
                            "wikilink_count": s.wikilink_count,
                        })
                    })
                    .collect();
                to_json_string(&make_envelope(
                    "note_backlinks",
                    serde_json::json!({
                        "path": p.path,
                        "backlinks": items,
                        "count": sources.len(),
                        "total_wikilinks": total,
                    }),
                ))
            }
            Err(ref e) => core_error_json(e),
        }
    }

    #[tool(description = "Rename a note and update all wikilink references. dry_run=true (default) returns the plan. Set dry_run=false to apply.")]
    fn note_revision(&self, Parameters(p): Parameters<NoteRevisionParams>) -> String {
        use qwert_core::revision::{NamingStyle, RevisionRequest};

        let naming = match p.naming.as_deref().unwrap_or("increment") {
            "increment" => NamingStyle::Increment,
            "date" => NamingStyle::Date,
            "semver" => NamingStyle::Semver,
            "manual" => NamingStyle::Manual,
            other => {
                return to_json_string(&make_envelope(
                    "error",
                    serde_json::json!({
                        "category": "validation",
                        "exit_code": ExitCode::Validation as u8,
                        "message": format!("Unknown naming style: {other}"),
                        "next_step": "Use one of: increment | date | semver | manual",
                    }),
                ));
            }
        };

        let date_str = (naming == NamingStyle::Date)
            .then(crate::cli::note::today_yyyymmdd);

        let req = RevisionRequest {
            vault_root: self.vault_root.clone(),
            source_rel_path: p.path.clone(),
            naming,
            new_name: p.name.clone(),
            excluded_dirs: qwert_core::config::load_global_config()
                .revision
                .excluded_dirs,
            date_str,
        };

        if p.dry_run {
            match qwert_core::revision::plan_revision(&req) {
                Ok(plan) => to_json_string(&serde_json::to_value(&plan).unwrap_or_default()),
                Err(ref e) => core_error_json(e),
            }
        } else {
            match qwert_core::revision::execute_revision(&req) {
                Ok(result) => {
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
                    to_json_string(&make_envelope(
                        "revision_result",
                        serde_json::json!({
                            "old_path": result.old_path,
                            "new_path": result.new_path,
                            "affected_files": affected,
                            "total_wikilinks": result.total_wikilinks,
                        }),
                    ))
                }
                Err(ref e) => core_error_json(e),
            }
        }
    }

    #[tool(description = "Scan a note for invisible characters (Unicode control chars, null bytes, etc.).")]
    fn note_scan(&self, Parameters(p): Parameters<NotePathParams>) -> String {
        match qwert_core::vault::read_file(&self.vault_root, &p.path) {
            Ok(content) => {
                let findings = qwert_core::sanitize::detect_invisible_chars(&content);
                let total = findings.len();
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
                to_json_string(&make_envelope(
                    "scan_result",
                    serde_json::json!({ "path": p.path, "findings": items, "total": total }),
                ))
            }
            Err(ref e) => core_error_json(e),
        }
    }

    // ── vault ─────────────────────────────────────────────────────────────────

    #[tool(description = "Full-text search across all notes in the vault.")]
    fn vault_search(&self, Parameters(p): Parameters<VaultSearchParams>) -> String {
        match qwert_core::search::search_vault(&self.vault_root, &p.query, p.regex) {
            Ok(hits) => {
                let total = hits.len();
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
                to_json_string(&make_envelope(
                    "search_results",
                    serde_json::json!({
                        "query": p.query,
                        "hits": json_hits,
                        "total_hits": total,
                    }),
                ))
            }
            Err(ref e) => core_error_json(e),
        }
    }

    #[tool(description = "Report vault health: sync conflicts, appearance conflicts, etc.")]
    fn vault_status(&self, _: Parameters<NoParams>) -> String {
        match qwert_core::status::check_vault_status(&self.vault_root) {
            Ok(s) => {
                let payload = serde_json::to_value(&s).unwrap_or_default();
                to_json_string(&make_envelope("vault_status", payload))
            }
            Err(ref e) => core_error_json(e),
        }
    }

    #[tool(description = "Scan all notes in the vault for invisible characters.")]
    fn vault_scan(&self, _: Parameters<NoParams>) -> String {
        match qwert_core::vault::scan_vault(&self.vault_root) {
            Ok(tree) => {
                let mut all_findings: Vec<(
                    String,
                    Vec<qwert_core::sanitize::InvisibleCharFinding>,
                )> = Vec::new();
                collect_vault_findings(&self.vault_root, &tree, &mut all_findings);
                let total: usize = all_findings.iter().map(|(_, f)| f.len()).sum();
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
                to_json_string(&make_envelope(
                    "vault_scan_result",
                    serde_json::json!({
                        "findings": items,
                        "files_with_findings": all_findings.len(),
                        "total": total,
                    }),
                ))
            }
            Err(ref e) => core_error_json(e),
        }
    }
}

#[tool_handler]
impl ServerHandler for QwertMcpServer {}

pub async fn run_server(vault_root: PathBuf) -> i32 {
    let server = QwertMcpServer::new(vault_root);
    match server.serve(rmcp::transport::stdio()).await {
        Ok(service) => match service.waiting().await {
            Ok(_) => 0,
            Err(e) => {
                eprintln!("MCP service error: {e}");
                1
            }
        },
        Err(e) => {
            eprintln!("MCP server startup failed: {e}");
            1
        }
    }
}
