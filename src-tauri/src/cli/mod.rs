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

    /// MCP server mode (Phase 3, not yet available)
    Mcp,

    /// Describe a subcommand schema (Phase 3; shows --help for now)
    Describe {
        subcommand: Option<String>,
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
}

/// Returns process exit code.
pub fn run() -> i32 {
    let cli = Cli::parse();
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
            FileCmd::Write { path, yes, format } => file::execute_write(&path, yes, format, vault_root),
            FileCmd::List { tree: _, format } => file::execute_list(format, vault_root),
        },

        Command::Note { cmd } => match cmd {
            NoteCmd::Render { path, format } => note::execute_render(&path, format, vault_root),
            NoteCmd::Backlinks { path, format } => note::execute_backlinks(&path, format, vault_root),
        },

        Command::VaultCmd { cmd } => match cmd {
            VaultSubCmd::Search { query, regex, format } => {
                vault::execute_search(&query, regex, format, vault_root)
            }
            VaultSubCmd::Status { format } => vault::execute_status(format, vault_root),
        },

        // Short aliases
        Command::Read { path, format } => file::execute_read(&path, format, vault_root),
        Command::Write { path, yes, format } => file::execute_write(&path, yes, format, vault_root),
        Command::List { tree: _, format } => file::execute_list(format, vault_root),
        Command::Render { path, format } => note::execute_render(&path, format, vault_root),
        Command::Backlinks { path, format } => note::execute_backlinks(&path, format, vault_root),
        Command::Search { query, regex, format } => {
            vault::execute_search(&query, regex, format, vault_root)
        }
        Command::Status { format } => vault::execute_status(format, vault_root),

        Command::GenerateMan => generate_man(),

        Command::Mcp => {
            eprintln!("MCP server mode is not yet supported (Phase 3)");
            ExitCode::Usage.as_i32()
        }

        Command::Describe { .. } => {
            Cli::command().print_help().ok();
            ExitCode::Success.as_i32()
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
        let Command::File { cmd: FileCmd::Read { format, .. } } = cli.command else {
            panic!("wrong command");
        };
        assert_eq!(format, OutputFormat::Raw);
    }

    #[test]
    fn file_list_default_format_is_json() {
        let cli = Cli::parse_from(["qwert", "file", "list"]);
        let Command::File { cmd: FileCmd::List { format, .. } } = cli.command else {
            panic!("wrong command");
        };
        assert_eq!(format, OutputFormat::Json);
    }

    #[test]
    fn note_render_default_format_is_raw() {
        let cli = Cli::parse_from(["qwert", "note", "render", "foo.md"]);
        let Command::Note { cmd: NoteCmd::Render { format, .. } } = cli.command else {
            panic!("wrong command");
        };
        assert_eq!(format, OutputFormat::Raw);
    }

    #[test]
    fn note_backlinks_default_format_is_json() {
        let cli = Cli::parse_from(["qwert", "note", "backlinks", "foo.md"]);
        let Command::Note { cmd: NoteCmd::Backlinks { format, .. } } = cli.command else {
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
        let Command::File { cmd: FileCmd::List { format, .. } } = cli.command else {
            panic!("wrong command");
        };
        assert_eq!(format, OutputFormat::Path);
    }

    #[test]
    fn file_read_format_json_override() {
        let cli = Cli::parse_from(["qwert", "file", "read", "a.md", "--format", "json"]);
        let Command::File { cmd: FileCmd::Read { format, .. } } = cli.command else {
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
        let args: Vec<String> = ["qwert", "file", "list"].iter().map(|s| s.to_string()).collect();
        assert!(is_cli_mode(&args));
    }

    #[test]
    fn is_cli_mode_false_for_no_args() {
        let args: Vec<String> = vec!["qwert".to_string()];
        assert!(!is_cli_mode(&args));
    }

    #[test]
    fn is_cli_mode_false_for_tauri_args() {
        let args: Vec<String> = ["qwert", "--webview"].iter().map(|s| s.to_string()).collect();
        assert!(!is_cli_mode(&args));
    }
}
