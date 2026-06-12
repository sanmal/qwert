pub mod appearance;
pub mod describe;
pub mod exit_code;
pub mod file;
pub mod format;
pub mod note;
pub mod tty;
pub mod vault;

use std::path::{Path, PathBuf};

use clap::{CommandFactory, Parser, Subcommand};
use exit_code::ExitCode;
use format::OutputFormat;
use qwert_core::error::ActionableError;
use qwert_core::revision::NamingStyle;

/// Naming style for `note revision` (matches qwert_core::revision::NamingStyle).
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, Default)]
pub enum CliNamingStyle {
    #[default]
    Increment,
    Date,
    Semver,
    Manual,
}

impl From<CliNamingStyle> for NamingStyle {
    fn from(s: CliNamingStyle) -> Self {
        match s {
            CliNamingStyle::Increment => NamingStyle::Increment,
            CliNamingStyle::Date => NamingStyle::Date,
            CliNamingStyle::Semver => NamingStyle::Semver,
            CliNamingStyle::Manual => NamingStyle::Manual,
        }
    }
}

#[derive(Debug, Parser)]
#[command(name = "qwert", about = "Markdown note tool for agents and humans")]
#[command(version)]
pub struct Cli {
    /// Vault root directory (default: current directory)
    #[arg(long, global = true)]
    pub vault: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// File operations (read / write / list)
    File {
        #[command(subcommand)]
        cmd: FileCmd,
    },

    /// Note operations (render / backlinks)
    Note {
        #[command(subcommand)]
        cmd: NoteCmd,
    },

    /// Vault operations (search / status)
    #[command(name = "vault")]
    VaultCmd {
        #[command(subcommand)]
        cmd: VaultSubCmd,
    },

    /// Appearance settings (global scope)
    Appearance {
        #[command(subcommand)]
        cmd: AppearanceCmd,
    },

    /// MCP server mode (stdio JSON-RPC)
    Mcp,

    /// Print JSON schema for a subcommand's arguments
    Describe {
        /// Subcommand to describe, e.g. "file read" or "note" (omit for all)
        subcommand: Option<String>,
        /// Output format: json (default) | text
        #[arg(long, default_value = "json")]
        format: OutputFormat,
    },

    /// Generate man page
    #[command(hide = true)]
    GenerateMan,

    // ── Short aliases (hidden, human-friendly shortcuts) ────────────────────
    #[command(hide = true)]
    Read {
        path: String,
        #[arg(long, default_value = "raw")]
        format: OutputFormat,
    },
    #[command(hide = true)]
    Write {
        path: String,
        #[arg(long)]
        yes: bool,
        #[arg(long, value_name = "MTIME")]
        if_match: Option<u64>,
        #[arg(long, default_value = "json")]
        format: OutputFormat,
    },
    #[command(hide = true)]
    List {
        #[arg(long)]
        tree: bool,
        #[arg(long, default_value = "json")]
        format: OutputFormat,
    },
    #[command(hide = true)]
    Render {
        path: String,
        #[arg(long, default_value = "raw")]
        format: OutputFormat,
    },
    #[command(hide = true)]
    Backlinks {
        path: String,
        #[arg(long, default_value = "json")]
        format: OutputFormat,
    },
    #[command(hide = true)]
    Revision {
        path: String,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        diff: bool,
        #[arg(long, default_value = "text")]
        format: OutputFormat,
        #[arg(long)]
        naming: Option<CliNamingStyle>,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        yes: bool,
    },
    #[command(hide = true)]
    Search {
        query: String,
        #[arg(long)]
        regex: bool,
        #[arg(long, default_value = "json")]
        format: OutputFormat,
    },
    #[command(hide = true)]
    Status {
        #[arg(long, default_value = "json")]
        format: OutputFormat,
    },
    /// `note scan` alias (hidden)
    #[command(hide = true)]
    Scan {
        path: String,
        #[arg(long, default_value = "json")]
        format: OutputFormat,
    },
}

#[derive(Debug, Subcommand)]
pub enum FileCmd {
    /// Read a file and output its contents
    Read {
        path: String,
        #[arg(long, default_value = "raw")]
        format: OutputFormat,
    },
    /// Write stdin to a file (atomic)
    Write {
        path: String,
        #[arg(long)]
        yes: bool,
        /// Optimistic mtime lock (Unix seconds from `file read --format json`).
        /// Reject with exit 4 if the file's mtime has changed since this value.
        #[arg(long, value_name = "MTIME")]
        if_match: Option<u64>,
        #[arg(long, default_value = "json")]
        format: OutputFormat,
    },
    /// List .md files in the vault
    List {
        #[arg(long)]
        tree: bool,
        #[arg(long, default_value = "json")]
        format: OutputFormat,
    },
}

#[derive(Debug, Subcommand)]
pub enum NoteCmd {
    /// Render Markdown to HTML
    Render {
        path: String,
        #[arg(long, default_value = "raw")]
        format: OutputFormat,
    },
    /// Show backlinks to a note
    Backlinks {
        path: String,
        #[arg(long, default_value = "json")]
        format: OutputFormat,
    },
    /// Scan a note for invisible characters (§11 第1層)
    Scan {
        path: String,
        #[arg(long, default_value = "json")]
        format: OutputFormat,
    },
    /// Rename a note and update all wikilink references (Revision system)
    Revision {
        /// Vault-relative path of the note to revise
        path: String,
        /// Plan without applying (always safe)
        #[arg(long)]
        dry_run: bool,
        /// (with --dry-run) Write unified diff to a temp file; add diff_path to JSON
        #[arg(long)]
        diff: bool,
        /// Output format: text (default) | json | diff
        #[arg(long, default_value = "text")]
        format: OutputFormat,
        /// Naming style (default: read from config.toml)
        #[arg(long)]
        naming: Option<CliNamingStyle>,
        /// Explicit new name (required when --naming=manual)
        #[arg(long)]
        name: Option<String>,
        /// Apply without interactive confirmation (required in non-TTY)
        #[arg(long)]
        yes: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum VaultSubCmd {
    /// Full-text search
    Search {
        query: String,
        /// Treat query as a regex pattern instead of a literal string
        #[arg(long)]
        regex: bool,
        #[arg(long, default_value = "json")]
        format: OutputFormat,
    },
    /// Vault state report
    Status {
        #[arg(long, default_value = "json")]
        format: OutputFormat,
    },
    /// Scan entire vault for invisible characters (§11 第1層)
    Scan {
        #[arg(long, default_value = "json")]
        format: OutputFormat,
    },
}

#[derive(Debug, Subcommand)]
pub enum AppearanceCmd {
    /// Calculate WCAG 2.x contrast ratio between two hex colors
    Contrast {
        /// Foreground color (#rrggbb or #rgb)
        fg: String,
        /// Background color (#rrggbb or #rgb)
        bg: String,
        /// Exit 5 if ratio is below WCAG AA (≥4.5)
        #[arg(long)]
        assert_aa: bool,
        /// Exit 5 if ratio is below WCAG AAA (≥7.0)
        #[arg(long)]
        assert_aaa: bool,
        #[arg(long, default_value = "text")]
        format: OutputFormat,
    },
    /// Set appearance configuration (vault scope by default)
    Set {
        /// Apply a named color preset
        #[arg(long)]
        preset: Option<String>,
        /// Foreground hex color (requires --bg; F24)
        #[arg(long)]
        fg: Option<String>,
        /// Background hex color (requires --fg; F24)
        #[arg(long)]
        bg: Option<String>,
        /// Reject custom fg/bg if WCAG AA contrast (4.5) is not met
        #[arg(long)]
        require_aa: bool,
        /// Configuration scope: vault (default) or global
        #[arg(long, default_value = "vault")]
        scope: String,
        #[arg(long, default_value = "text")]
        format: OutputFormat,
    },
    /// Output a template appearance.toml
    Template {
        #[arg(long, default_value = "text")]
        format: OutputFormat,
    },
    /// Show the effective appearance configuration and WCAG contrast status
    Status {
        /// Output format: json (default) | text
        #[arg(long, default_value = "json")]
        format: OutputFormat,
    },
}

/// Returns process exit code.
pub fn run() -> i32 {
    let cli = Cli::parse();

    if matches!(cli.command, Command::Mcp) && cli.vault.is_none() {
        eprintln!("error: mcp requires --vault <path>");
        return ExitCode::Usage.as_i32();
    }

    let vault_root = cli
        .vault
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."));

    dispatch(cli.command, &vault_root)
}

fn dispatch(command: Command, vault_root: &Path) -> i32 {
    match command {
        Command::File { cmd } => match cmd {
            FileCmd::Read { path, format } => file::execute_read(&path, format, vault_root),
            FileCmd::Write {
                path,
                yes,
                if_match,
                format,
            } => file::execute_write(&path, yes, if_match, format, vault_root),
            FileCmd::List { tree, format } => file::execute_list(tree, format, vault_root),
        },

        Command::Note { cmd } => match cmd {
            NoteCmd::Render { path, format } => note::execute_render(&path, format, vault_root),
            NoteCmd::Backlinks { path, format } => {
                note::execute_backlinks(&path, format, vault_root)
            }
            NoteCmd::Scan { path, format } => note::execute_scan(&path, format, vault_root),
            NoteCmd::Revision {
                path,
                dry_run,
                diff,
                format,
                naming,
                name,
                yes,
            } => note::execute_revision(
                note::RevisionArgs {
                    path,
                    dry_run,
                    diff_flag: diff,
                    format,
                    naming: naming.map(Into::into),
                    name,
                    yes,
                },
                vault_root,
            ),
        },

        Command::VaultCmd { cmd } => match cmd {
            VaultSubCmd::Search {
                query,
                regex,
                format,
            } => vault::execute_search(&query, regex, format, vault_root),
            VaultSubCmd::Status { format } => vault::execute_status(format, vault_root),
            VaultSubCmd::Scan { format } => vault::execute_scan(format, vault_root),
        },

        Command::Appearance { cmd } => match cmd {
            AppearanceCmd::Contrast {
                fg,
                bg,
                assert_aa,
                assert_aaa,
                format,
            } => appearance::execute_contrast(appearance::ContrastArgs {
                fg,
                bg,
                assert_aa,
                assert_aaa,
                format,
            }),
            AppearanceCmd::Set {
                preset,
                fg,
                bg,
                require_aa,
                scope,
                format,
            } => appearance::execute_set(appearance::SetArgs {
                preset,
                fg,
                bg,
                require_aa,
                scope,
                vault_root: vault_root.to_path_buf(),
                format,
            }),
            AppearanceCmd::Template { format } => appearance::execute_template(format),
            AppearanceCmd::Status { format } => appearance::execute_status(format, vault_root),
        },

        // Short aliases
        Command::Read { path, format } => file::execute_read(&path, format, vault_root),
        Command::Write {
            path,
            yes,
            if_match,
            format,
        } => file::execute_write(&path, yes, if_match, format, vault_root),
        Command::List { tree, format } => file::execute_list(tree, format, vault_root),
        Command::Render { path, format } => note::execute_render(&path, format, vault_root),
        Command::Backlinks { path, format } => note::execute_backlinks(&path, format, vault_root),
        Command::Revision {
            path,
            dry_run,
            diff,
            format,
            naming,
            name,
            yes,
        } => note::execute_revision(
            note::RevisionArgs {
                path,
                dry_run,
                diff_flag: diff,
                format,
                naming: naming.map(Into::into),
                name,
                yes,
            },
            vault_root,
        ),
        Command::Search {
            query,
            regex,
            format,
        } => vault::execute_search(&query, regex, format, vault_root),
        Command::Status { format } => vault::execute_status(format, vault_root),
        Command::Scan { path, format } => note::execute_scan(&path, format, vault_root),

        Command::GenerateMan => generate_man(),

        Command::Mcp => {
            let rt = tokio::runtime::Runtime::new().unwrap_or_else(|e| {
                eprintln!("failed to create tokio runtime: {e}");
                std::process::exit(ExitCode::General.as_i32());
            });
            rt.block_on(crate::mcp::run_server(vault_root.to_path_buf()))
        }

        Command::Describe { subcommand, format } => {
            describe::execute_describe(subcommand.as_deref(), format)
        }
    }
}

fn generate_man() -> i32 {
    let cmd = Cli::command();
    let man = clap_mangen::Man::new(cmd);
    let mut buf = Vec::new();
    if let Err(e) = man.render(&mut buf) {
        eprintln!("error generating man page: {e}");
        return ExitCode::General.as_i32();
    }
    use std::io::Write;
    if let Err(e) = std::io::stdout().write_all(&buf) {
        eprintln!("error writing man page: {e}");
        return ExitCode::General.as_i32();
    }
    ExitCode::Success.as_i32()
}

/// Emit a CoreError to stderr as structured JSON and return the exit code.
pub(super) fn emit_core_error(e: &qwert_core::CoreError) -> i32 {
    let code = ExitCode::from(e);
    let next = match e {
        qwert_core::CoreError::NotFound(_) => {
            Some("Run qwert with --vault <PATH> or cd into a vault".to_owned())
        }
        qwert_core::CoreError::PathTraversal(_) => {
            Some("Use a path relative to the vault root".to_owned())
        }
        qwert_core::CoreError::InvalidUtf8 { byte_offset } => Some(format!(
            "The file contains invalid UTF-8 bytes at offset {byte_offset}. \
             Inspect with: xxd <file> | grep -A2 '{byte_offset:x}'"
        )),
        _ => None,
    };
    let mut err = ActionableError::new(code.category_str(), code as u8, e.to_string());
    if let Some(s) = next {
        err = err.with_next_step(s);
    }
    eprintln!("{}", serde_json::to_string_pretty(&err).unwrap_or_default());
    code.as_i32()
}

/// Returns true when argv indicates CLI mode (not Tauri GUI).
pub fn is_cli_mode(args: &[String]) -> bool {
    const CLI_SUBCOMMANDS: &[&str] = &[
        "file",
        "note",
        "vault",
        "appearance",
        "mcp",
        "describe",
        "generate-man",
        "read",
        "write",
        "list",
        "render",
        "backlinks",
        "revision",
        "scan",
        "search",
        "status",
        "--help",
        "-h",
        "--version",
        "-V",
    ];
    args.get(1)
        .map(|a| CLI_SUBCOMMANDS.contains(&a.as_str()))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Default format per command (§14) ─────────────────────────────────────

    #[test]
    fn file_read_default_format_is_raw() {
        let cli = Cli::parse_from(["qwert", "file", "read", "test.md"]);
        let Command::File {
            cmd: FileCmd::Read { format, .. },
        } = cli.command
        else {
            panic!("wrong command");
        };
        assert_eq!(format, OutputFormat::Raw);
    }

    #[test]
    fn file_list_default_format_is_json() {
        let cli = Cli::parse_from(["qwert", "file", "list"]);
        let Command::File {
            cmd: FileCmd::List { format, .. },
        } = cli.command
        else {
            panic!("wrong command");
        };
        assert_eq!(format, OutputFormat::Json);
    }

    #[test]
    fn note_render_default_format_is_raw() {
        let cli = Cli::parse_from(["qwert", "note", "render", "foo.md"]);
        let Command::Note {
            cmd: NoteCmd::Render { format, .. },
        } = cli.command
        else {
            panic!("wrong command");
        };
        assert_eq!(format, OutputFormat::Raw);
    }

    #[test]
    fn note_backlinks_default_format_is_json() {
        let cli = Cli::parse_from(["qwert", "note", "backlinks", "foo.md"]);
        let Command::Note {
            cmd: NoteCmd::Backlinks { format, .. },
        } = cli.command
        else {
            panic!("wrong command");
        };
        assert_eq!(format, OutputFormat::Json);
    }

    #[test]
    fn alias_read_default_format_is_raw() {
        let cli = Cli::parse_from(["qwert", "read", "foo.md"]);
        let Command::Read { format, .. } = cli.command else {
            panic!("wrong command");
        };
        assert_eq!(format, OutputFormat::Raw);
    }

    // ── --format override ────────────────────────────────────────────────────

    #[test]
    fn file_list_format_path_override() {
        let cli = Cli::parse_from(["qwert", "file", "list", "--format", "path"]);
        let Command::File {
            cmd: FileCmd::List { format, .. },
        } = cli.command
        else {
            panic!("wrong command");
        };
        assert_eq!(format, OutputFormat::Path);
    }

    #[test]
    fn file_read_format_json_override() {
        let cli = Cli::parse_from(["qwert", "file", "read", "a.md", "--format", "json"]);
        let Command::File {
            cmd: FileCmd::Read { format, .. },
        } = cli.command
        else {
            panic!("wrong command");
        };
        assert_eq!(format, OutputFormat::Json);
    }

    // ── Global --vault flag ──────────────────────────────────────────────────

    #[test]
    fn global_vault_flag_is_captured() {
        let cli = Cli::parse_from(["qwert", "--vault", "/tmp/vault", "file", "list"]);
        assert_eq!(cli.vault, Some(PathBuf::from("/tmp/vault")));
    }

    #[test]
    fn vault_flag_after_subcommand_is_captured() {
        let cli = Cli::parse_from(["qwert", "file", "list", "--vault", "/tmp/vault"]);
        assert_eq!(cli.vault, Some(PathBuf::from("/tmp/vault")));
    }

    // ── is_cli_mode ──────────────────────────────────────────────────────────

    #[test]
    fn is_cli_mode_detects_file_subcommand() {
        let args: Vec<String> = ["qwert", "file", "list"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert!(is_cli_mode(&args));
    }

    #[test]
    fn is_cli_mode_false_for_no_args() {
        let args: Vec<String> = vec!["qwert".to_string()];
        assert!(!is_cli_mode(&args));
    }

    #[test]
    fn is_cli_mode_false_for_tauri_args() {
        let args: Vec<String> = ["qwert", "--webview"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert!(!is_cli_mode(&args));
    }

    // ── note revision (§14: default format = text) ───────────────────────────

    #[test]
    fn note_revision_default_format_is_text() {
        let cli = Cli::parse_from(["qwert", "note", "revision", "auth.md"]);
        let Command::Note {
            cmd: NoteCmd::Revision { format, .. },
        } = cli.command
        else {
            panic!("wrong command");
        };
        assert_eq!(format, OutputFormat::Text);
    }

    #[test]
    fn note_revision_naming_none_when_not_specified() {
        // When --naming is omitted, naming is None (resolved from config at runtime).
        let cli = Cli::parse_from(["qwert", "note", "revision", "auth.md"]);
        let Command::Note {
            cmd: NoteCmd::Revision { naming, .. },
        } = cli.command
        else {
            panic!("wrong command");
        };
        assert_eq!(naming, None);
    }

    #[test]
    fn note_revision_naming_some_when_specified() {
        let cli = Cli::parse_from(["qwert", "note", "revision", "auth.md", "--naming", "date"]);
        let Command::Note {
            cmd: NoteCmd::Revision { naming, .. },
        } = cli.command
        else {
            panic!("wrong command");
        };
        assert_eq!(naming, Some(CliNamingStyle::Date));
    }

    #[test]
    fn note_revision_format_diff() {
        let cli = Cli::parse_from(["qwert", "note", "revision", "auth.md", "--format", "diff"]);
        let Command::Note {
            cmd: NoteCmd::Revision { format, .. },
        } = cli.command
        else {
            panic!("wrong command");
        };
        assert_eq!(format, OutputFormat::Diff);
    }

    #[test]
    fn note_revision_format_json() {
        let cli = Cli::parse_from(["qwert", "note", "revision", "auth.md", "--format", "json"]);
        let Command::Note {
            cmd: NoteCmd::Revision { format, .. },
        } = cli.command
        else {
            panic!("wrong command");
        };
        assert_eq!(format, OutputFormat::Json);
    }

    #[test]
    fn note_revision_dry_run_flag() {
        let cli = Cli::parse_from(["qwert", "note", "revision", "auth.md", "--dry-run"]);
        let Command::Note {
            cmd: NoteCmd::Revision { dry_run, .. },
        } = cli.command
        else {
            panic!("wrong command");
        };
        assert!(dry_run);
    }

    #[test]
    fn note_revision_diff_flag() {
        let cli = Cli::parse_from([
            "qwert",
            "note",
            "revision",
            "auth.md",
            "--dry-run",
            "--diff",
        ]);
        let Command::Note {
            cmd: NoteCmd::Revision { diff, .. },
        } = cli.command
        else {
            panic!("wrong command");
        };
        assert!(diff);
    }

    #[test]
    fn note_revision_manual_naming_with_name() {
        let cli = Cli::parse_from([
            "qwert", "note", "revision", "auth.md", "--naming", "manual", "--name", "auth_jwt",
        ]);
        let Command::Note {
            cmd: NoteCmd::Revision { naming, name, .. },
        } = cli.command
        else {
            panic!("wrong command");
        };
        assert_eq!(naming, Some(CliNamingStyle::Manual));
        assert_eq!(name, Some("auth_jwt".to_owned()));
    }

    #[test]
    fn note_revision_yes_flag() {
        let cli = Cli::parse_from(["qwert", "note", "revision", "auth.md", "--yes"]);
        let Command::Note {
            cmd: NoteCmd::Revision { yes, .. },
        } = cli.command
        else {
            panic!("wrong command");
        };
        assert!(yes);
    }

    #[test]
    fn naming_style_converts_to_core() {
        assert_eq!(
            NamingStyle::from(CliNamingStyle::Increment),
            NamingStyle::Increment
        );
        assert_eq!(NamingStyle::from(CliNamingStyle::Date), NamingStyle::Date);
        assert_eq!(
            NamingStyle::from(CliNamingStyle::Semver),
            NamingStyle::Semver
        );
        assert_eq!(
            NamingStyle::from(CliNamingStyle::Manual),
            NamingStyle::Manual
        );
    }

    #[test]
    fn alias_revision_parsed() {
        let cli = Cli::parse_from(["qwert", "revision", "auth.md", "--dry-run"]);
        let Command::Revision { path, dry_run, .. } = cli.command else {
            panic!("wrong command");
        };
        assert_eq!(path, "auth.md");
        assert!(dry_run);
    }

    // ── file write --if-match (§12 Level 2) ─────────────────────────────────

    #[test]
    fn file_write_if_match_parses_u64() {
        let cli = Cli::parse_from([
            "qwert",
            "file",
            "write",
            "note.md",
            "--if-match",
            "1699000000",
            "--yes",
        ]);
        let Command::File {
            cmd: FileCmd::Write { if_match, yes, .. },
        } = cli.command
        else {
            panic!("wrong command");
        };
        assert_eq!(if_match, Some(1699000000u64));
        assert!(yes);
    }

    #[test]
    fn file_write_without_if_match_is_none() {
        let cli = Cli::parse_from(["qwert", "file", "write", "note.md", "--yes"]);
        let Command::File {
            cmd: FileCmd::Write { if_match, .. },
        } = cli.command
        else {
            panic!("wrong command");
        };
        assert_eq!(if_match, None);
    }

    // ── appearance (§14: default format = text) ──────────────────────────────

    #[test]
    fn appearance_contrast_default_format_is_text() {
        let cli = Cli::parse_from(["qwert", "appearance", "contrast", "#000", "#fff"]);
        let Command::Appearance {
            cmd: AppearanceCmd::Contrast { format, .. },
        } = cli.command
        else {
            panic!("wrong command");
        };
        assert_eq!(format, OutputFormat::Text);
    }

    #[test]
    fn appearance_contrast_assert_flags_parsed() {
        let cli = Cli::parse_from([
            "qwert",
            "appearance",
            "contrast",
            "#000",
            "#fff",
            "--assert-aa",
            "--assert-aaa",
        ]);
        let Command::Appearance {
            cmd:
                AppearanceCmd::Contrast {
                    assert_aa,
                    assert_aaa,
                    ..
                },
        } = cli.command
        else {
            panic!("wrong command");
        };
        assert!(assert_aa);
        assert!(assert_aaa);
    }

    #[test]
    fn appearance_set_default_format_is_text() {
        let cli = Cli::parse_from(["qwert", "appearance", "set", "--preset", "dark"]);
        let Command::Appearance {
            cmd: AppearanceCmd::Set { format, .. },
        } = cli.command
        else {
            panic!("wrong command");
        };
        assert_eq!(format, OutputFormat::Text);
    }

    #[test]
    fn appearance_set_default_scope_is_vault() {
        // C4: default changed from "global" (Phase 2 暫定) to "vault".
        let cli = Cli::parse_from(["qwert", "appearance", "set", "--preset", "dark"]);
        let Command::Appearance {
            cmd: AppearanceCmd::Set { scope, .. },
        } = cli.command
        else {
            panic!("wrong command");
        };
        assert_eq!(scope, "vault");
    }

    #[test]
    fn appearance_set_custom_colors_parsed() {
        let cli = Cli::parse_from([
            "qwert",
            "appearance",
            "set",
            "--fg",
            "#1a1a1a",
            "--bg",
            "#ffffff",
        ]);
        let Command::Appearance {
            cmd: AppearanceCmd::Set { fg, bg, preset, .. },
        } = cli.command
        else {
            panic!("wrong command");
        };
        assert_eq!(fg.as_deref(), Some("#1a1a1a"));
        assert_eq!(bg.as_deref(), Some("#ffffff"));
        assert!(preset.is_none());
    }

    #[test]
    fn appearance_template_default_format_is_text() {
        let cli = Cli::parse_from(["qwert", "appearance", "template"]);
        let Command::Appearance {
            cmd: AppearanceCmd::Template { format },
        } = cli.command
        else {
            panic!("wrong command");
        };
        assert_eq!(format, OutputFormat::Text);
    }

    #[test]
    fn appearance_status_default_format_is_json() {
        let cli = Cli::parse_from(["qwert", "appearance", "status"]);
        let Command::Appearance {
            cmd: AppearanceCmd::Status { format },
        } = cli.command
        else {
            panic!("wrong command");
        };
        assert_eq!(format, OutputFormat::Json);
    }

    #[test]
    fn alias_write_if_match_parses_u64() {
        let cli = Cli::parse_from([
            "qwert",
            "write",
            "note.md",
            "--if-match",
            "1234567890",
            "--yes",
        ]);
        let Command::Write { if_match, .. } = cli.command else {
            panic!("wrong command");
        };
        assert_eq!(if_match, Some(1234567890u64));
    }
}
