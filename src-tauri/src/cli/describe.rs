use clap::CommandFactory;
use serde::Serialize;

use super::exit_code::ExitCode;
use super::format::{make_envelope, to_json_string, OutputFormat};
use super::Cli;

// ── Schema types (pub for B2 reuse) ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ArgSchema {
    pub name: String,
    /// "positional" | "option" | "flag"
    pub kind: String,
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct CommandSchema {
    /// Full canonical name, e.g. "file read"
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub args: Vec<ArgSchema>,
}

// ── Core introspection (single source of truth for B2) ───────────────────────

/// Build schemas from the clap command tree.
///
/// - `None`          → all canonical (non-hidden) leaf commands
/// - `Some("file")`  → all verbs under the "file" noun
/// - `Some("file read")` → exactly the "file read" leaf
pub fn build_schemas(noun_verb: Option<&str>) -> Result<Vec<CommandSchema>, String> {
    let root = Cli::command();

    match noun_verb {
        None => {
            let mut out = Vec::new();
            for noun in root.get_subcommands() {
                if noun.is_hide_set() {
                    continue;
                }
                collect_leaf_schemas(noun, noun.get_name(), &mut out);
            }
            Ok(out)
        }
        Some(s) => {
            let parts: Vec<&str> = s.split_whitespace().collect();
            match parts.as_slice() {
                [noun] => {
                    let noun_cmd = root
                        .get_subcommands()
                        .find(|c| c.get_name() == *noun && !c.is_hide_set())
                        .ok_or_else(|| format!("unknown subcommand: {noun}"))?;
                    let mut out = Vec::new();
                    collect_leaf_schemas(noun_cmd, noun, &mut out);
                    if out.is_empty() {
                        return Err(format!("no schema found for: {noun}"));
                    }
                    Ok(out)
                }
                [noun, verb] => {
                    let noun_cmd = root
                        .get_subcommands()
                        .find(|c| c.get_name() == *noun && !c.is_hide_set())
                        .ok_or_else(|| format!("unknown noun: {noun}"))?;
                    let verb_cmd = noun_cmd
                        .get_subcommands()
                        .find(|c| c.get_name() == *verb && !c.is_hide_set())
                        .ok_or_else(|| format!("unknown verb: {verb}"))?;
                    Ok(vec![schema_from_cmd(verb_cmd, &format!("{noun} {verb}"))])
                }
                _ => Err(format!("invalid subcommand specifier: {s}")),
            }
        }
    }
}

fn collect_leaf_schemas(cmd: &clap::Command, prefix: &str, out: &mut Vec<CommandSchema>) {
    let visible_subs: Vec<&clap::Command> =
        cmd.get_subcommands().filter(|c| !c.is_hide_set()).collect();

    if visible_subs.is_empty() {
        // leaf — emit schema for this command
        out.push(schema_from_cmd(cmd, prefix));
    } else {
        for sub in visible_subs {
            collect_leaf_schemas(sub, &format!("{prefix} {}", sub.get_name()), out);
        }
    }
}

fn schema_from_cmd(cmd: &clap::Command, full_name: &str) -> CommandSchema {
    const SKIP_IDS: &[&str] = &["help", "version", "vault"];

    let mut args: Vec<ArgSchema> = Vec::new();

    for arg in cmd.get_arguments() {
        let id = arg.get_id().to_string();
        if SKIP_IDS.contains(&id.as_str()) {
            continue;
        }

        let is_positional = arg.is_positional();
        let kind = if is_positional {
            "positional"
        } else {
            match arg.get_action() {
                clap::ArgAction::SetTrue | clap::ArgAction::SetFalse => "flag",
                _ => "option",
            }
        };

        let default = arg
            .get_default_values()
            .first()
            .and_then(|v| v.to_str())
            .map(str::to_owned);

        let description = arg.get_help().map(|s| s.to_string());
        let required = arg.is_required_set();

        let name = if is_positional {
            id
        } else {
            arg.get_long().map(str::to_owned).unwrap_or_else(|| {
                arg.get_id().to_string()
            })
        };

        args.push(ArgSchema {
            name,
            kind: kind.to_owned(),
            required,
            default,
            description,
        });
    }

    CommandSchema {
        name: full_name.to_owned(),
        description: cmd.get_about().map(|s| s.to_string()),
        args,
    }
}

// ── CLI entry point ───────────────────────────────────────────────────────────

pub fn execute_describe(subcommand: Option<&str>, format: OutputFormat) -> i32 {
    let schemas = match build_schemas(subcommand) {
        Ok(s) => s,
        Err(msg) => {
            eprintln!(
                "{}",
                to_json_string(&make_envelope(
                    "error",
                    serde_json::json!({
                        "category": "not_found",
                        "exit_code": ExitCode::NotFound as u8,
                        "message": msg,
                        "next_step": "Run `qwert describe` (no args) to list all subcommands",
                    }),
                ))
            );
            return ExitCode::NotFound.as_i32();
        }
    };

    match format {
        OutputFormat::Json => {
            let envelope = if schemas.len() == 1 {
                let s = &schemas[0];
                make_envelope(
                    "command_schema",
                    serde_json::json!({
                        "name": s.name,
                        "description": s.description,
                        "args": s.args,
                    }),
                )
            } else {
                make_envelope(
                    "command_schema_list",
                    serde_json::json!({ "commands": schemas }),
                )
            };
            println!("{}", to_json_string(&envelope));
        }
        _ => {
            for s in &schemas {
                println!("{}", s.name);
                if let Some(d) = &s.description {
                    println!("  {d}");
                }
                for arg in &s.args {
                    let req_tag = if arg.required { " (required)" } else { "" };
                    let def_tag = arg
                        .default
                        .as_deref()
                        .map(|d| format!(" [default: {d}]"))
                        .unwrap_or_default();
                    let kind_prefix = if arg.kind == "positional" {
                        format!("<{}>", arg.name)
                    } else {
                        format!("--{}", arg.name)
                    };
                    let desc_tag = arg
                        .description
                        .as_deref()
                        .map(|d| format!("  — {d}"))
                        .unwrap_or_default();
                    println!("  {kind_prefix}{req_tag}{def_tag}{desc_tag}");
                }
                println!();
            }
        }
    }

    ExitCode::Success.as_i32()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // All canonical nouns appear; hidden commands do not
    #[test]
    fn all_schemas_includes_canonical_nouns() {
        let schemas = build_schemas(None).unwrap();
        let names: Vec<&str> = schemas.iter().map(|s| s.name.as_str()).collect();
        // canonical verbs must be present
        assert!(names.contains(&"file read"));
        assert!(names.contains(&"file write"));
        assert!(names.contains(&"file list"));
        assert!(names.contains(&"note render"));
        assert!(names.contains(&"note backlinks"));
        assert!(names.contains(&"note revision"));
        assert!(names.contains(&"note scan"));
        assert!(names.contains(&"vault search"));
        assert!(names.contains(&"vault status"));
        assert!(names.contains(&"vault scan"));
        assert!(names.contains(&"appearance contrast"));
        assert!(names.contains(&"appearance set"));
        assert!(names.contains(&"appearance template"));
        assert!(names.contains(&"appearance status"));
    }

    // Short aliases (hidden) must not appear
    #[test]
    fn hidden_aliases_not_in_all_schemas() {
        let schemas = build_schemas(None).unwrap();
        let names: Vec<&str> = schemas.iter().map(|s| s.name.as_str()).collect();
        assert!(!names.contains(&"read"));
        assert!(!names.contains(&"write"));
        assert!(!names.contains(&"search"));
        assert!(!names.contains(&"status"));
        assert!(!names.contains(&"revision"));
        assert!(!names.contains(&"generate-man"));
    }

    // Noun filter: "file" returns all file verbs
    #[test]
    fn noun_filter_file_returns_all_verbs() {
        let schemas = build_schemas(Some("file")).unwrap();
        let names: Vec<&str> = schemas.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["file read", "file write", "file list"]);
    }

    // Specific "noun verb" returns exactly one schema
    #[test]
    fn noun_verb_returns_single_schema() {
        let schemas = build_schemas(Some("file read")).unwrap();
        assert_eq!(schemas.len(), 1);
        assert_eq!(schemas[0].name, "file read");
    }

    // Unknown noun returns Err
    #[test]
    fn unknown_noun_returns_err() {
        let result = build_schemas(Some("unknown"));
        assert!(result.is_err());
    }

    // Unknown verb returns Err
    #[test]
    fn unknown_verb_returns_err() {
        let result = build_schemas(Some("file unknown_verb"));
        assert!(result.is_err());
    }

    // "file read" has positional "path" arg
    #[test]
    fn file_read_has_path_positional() {
        let schemas = build_schemas(Some("file read")).unwrap();
        let schema = &schemas[0];
        let path_arg = schema.args.iter().find(|a| a.name == "path").unwrap();
        assert_eq!(path_arg.kind, "positional");
        assert!(path_arg.required);
    }

    // "file read" has "--format" option with default "raw"
    #[test]
    fn file_read_has_format_option_with_default_raw() {
        let schemas = build_schemas(Some("file read")).unwrap();
        let schema = &schemas[0];
        let fmt_arg = schema.args.iter().find(|a| a.name == "format").unwrap();
        assert_eq!(fmt_arg.kind, "option");
        assert_eq!(fmt_arg.default.as_deref(), Some("raw"));
    }

    // "note revision" has "--dry-run" flag
    #[test]
    fn note_revision_has_dry_run_flag() {
        let schemas = build_schemas(Some("note revision")).unwrap();
        let schema = &schemas[0];
        let dry = schema.args.iter().find(|a| a.name == "dry-run").unwrap();
        assert_eq!(dry.kind, "flag");
    }

    // "file write" has "--if-match" option
    #[test]
    fn file_write_has_if_match_option() {
        let schemas = build_schemas(Some("file write")).unwrap();
        let schema = &schemas[0];
        let if_match = schema
            .args
            .iter()
            .find(|a| a.name == "if-match")
            .unwrap();
        assert_eq!(if_match.kind, "option");
    }

    // JSON output envelope has correct shape for single command
    #[test]
    fn json_envelope_single_command_kind_is_command_schema() {
        let schemas = build_schemas(Some("file list")).unwrap();
        assert_eq!(schemas.len(), 1);
        let envelope = make_envelope(
            "command_schema",
            serde_json::json!({
                "name": schemas[0].name,
                "description": schemas[0].description,
                "args": schemas[0].args,
            }),
        );
        assert_eq!(envelope["schema_version"], "v1");
        assert_eq!(envelope["kind"], "command_schema");
        assert_eq!(envelope["name"], "file list");
        assert!(envelope.get("data").is_none());
    }

    // JSON output envelope has correct shape for list
    #[test]
    fn json_envelope_list_kind_is_command_schema_list() {
        let schemas = build_schemas(Some("file")).unwrap();
        let envelope = make_envelope(
            "command_schema_list",
            serde_json::json!({ "commands": schemas }),
        );
        assert_eq!(envelope["schema_version"], "v1");
        assert_eq!(envelope["kind"], "command_schema_list");
        assert!(envelope["commands"].is_array());
        assert!(envelope.get("data").is_none());
    }
}
