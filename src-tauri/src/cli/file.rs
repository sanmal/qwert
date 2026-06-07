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

pub fn execute_list(tree: bool, format: OutputFormat, vault_root: &Path) -> i32 {
    match vault::scan_vault(vault_root) {
        Ok(entries) => {
            // --format path is always flat regardless of --tree (xargs-friendly)
            if format == OutputFormat::Path {
                let mut paths = Vec::new();
                collect_paths(&entries, &mut paths);
                for p in &paths {
                    println!("{p}");
                }
                return ExitCode::Success.as_i32();
            }

            if tree {
                match format {
                    OutputFormat::Json => {
                        let nodes: Vec<serde_json::Value> =
                            entries.iter().map(entry_to_json).collect();
                        let v =
                            make_envelope("file_tree", serde_json::json!({ "tree": nodes }));
                        println!("{}", to_json_string(&v));
                    }
                    _ => {
                        for line in tree_to_lines(&entries, 0) {
                            println!("{line}");
                        }
                    }
                }
            } else {
                let mut paths = Vec::new();
                collect_paths(&entries, &mut paths);
                match format {
                    OutputFormat::Json => {
                        let v = make_envelope(
                            "file_list",
                            serde_json::json!({ "paths": paths, "count": paths.len() }),
                        );
                        println!("{}", to_json_string(&v));
                    }
                    _ => {
                        for p in &paths {
                            println!("{p}");
                        }
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

fn entry_to_json(e: &vault::VaultEntry) -> serde_json::Value {
    let ty = if e.is_dir { "dir" } else { "file" };
    if e.is_dir {
        let children: Vec<serde_json::Value> =
            e.children.as_deref().unwrap_or(&[]).iter().map(entry_to_json).collect();
        serde_json::json!({ "name": e.name, "path": e.path, "type": ty, "children": children })
    } else {
        serde_json::json!({ "name": e.name, "path": e.path, "type": ty })
    }
}

fn tree_to_lines(entries: &[vault::VaultEntry], depth: usize) -> Vec<String> {
    let indent = "  ".repeat(depth);
    let mut lines = Vec::new();
    for e in entries {
        if e.is_dir {
            lines.push(format!("{}{}/", indent, e.name));
            if let Some(ch) = &e.children {
                lines.extend(tree_to_lines(ch, depth + 1));
            }
        } else {
            lines.push(format!("{}{}", indent, e.name));
        }
    }
    lines
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use qwert_core::vault::VaultEntry;

    fn file(name: &str, path: &str) -> VaultEntry {
        VaultEntry { name: name.to_owned(), path: path.to_owned(), is_dir: false, children: None }
    }

    fn dir(name: &str, path: &str, ch: Vec<VaultEntry>) -> VaultEntry {
        VaultEntry {
            name: name.to_owned(),
            path: path.to_owned(),
            is_dir: true,
            children: Some(ch),
        }
    }

    #[test]
    fn entry_to_json_file_has_no_children_key() {
        let v = entry_to_json(&file("auth.md", "auth.md"));
        assert_eq!(v["type"], "file");
        assert_eq!(v["name"], "auth.md");
        assert_eq!(v["path"], "auth.md");
        assert!(v.get("children").is_none(), "file nodes must omit children");
    }

    #[test]
    fn entry_to_json_dir_has_children_array() {
        let v = entry_to_json(&dir("notes", "notes", vec![file("a.md", "notes/a.md")]));
        assert_eq!(v["type"], "dir");
        let children = v["children"].as_array().expect("dir must have children array");
        assert_eq!(children.len(), 1);
        assert_eq!(children[0]["name"], "a.md");
        assert_eq!(children[0]["type"], "file");
    }

    #[test]
    fn entry_to_json_empty_dir_has_empty_children() {
        let v = entry_to_json(&dir("empty", "empty", vec![]));
        assert_eq!(v["type"], "dir");
        assert_eq!(v["children"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn tree_to_lines_flat_file() {
        let lines = tree_to_lines(&[file("readme.md", "readme.md")], 0);
        assert_eq!(lines, vec!["readme.md"]);
    }

    #[test]
    fn tree_to_lines_dir_with_child() {
        let entries = vec![dir("notes", "notes", vec![file("auth.md", "notes/auth.md")])];
        let lines = tree_to_lines(&entries, 0);
        assert_eq!(lines, vec!["notes/", "  auth.md"]);
    }

    #[test]
    fn tree_to_lines_nested_two_levels() {
        let entries = vec![
            file("readme.md", "readme.md"),
            dir(
                "notes",
                "notes",
                vec![
                    file("auth.md", "notes/auth.md"),
                    dir("api", "notes/api", vec![file("ep.md", "notes/api/ep.md")]),
                ],
            ),
        ];
        let lines = tree_to_lines(&entries, 0);
        assert_eq!(
            lines,
            vec!["readme.md", "notes/", "  auth.md", "  api/", "    ep.md"]
        );
    }

    #[test]
    fn tree_json_kind_is_file_tree() {
        let nodes: Vec<serde_json::Value> =
            [file("a.md", "a.md")].iter().map(entry_to_json).collect();
        let v = make_envelope("file_tree", serde_json::json!({ "tree": nodes }));
        assert_eq!(v["kind"], "file_tree");
        assert!(v["tree"].is_array());
    }
}
