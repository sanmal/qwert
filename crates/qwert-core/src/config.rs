use serde::{Deserialize, Serialize};

pub fn load_global_config() -> Config {
    let Some(dirs) = directories::ProjectDirs::from("", "", "qwert") else {
        return Config::default();
    };
    load_config_from_path(&dirs.config_dir().join("config.toml"))
}

pub fn save_global_config(config: &Config) -> Result<(), std::io::Error> {
    let Some(dirs) = directories::ProjectDirs::from("", "", "qwert") else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "could not determine config directory",
        ));
    };
    let path = dirs.config_dir().join("config.toml");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = toml::to_string_pretty(config)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
    std::fs::write(path, content)
}

/// Returns `true` if `spec` is a syntactically valid key specification.
/// Format: one or more modifiers (Ctrl, Alt, Shift, Meta) separated by `+`,
/// followed by a single non-empty key name.  E.g. "Ctrl+S", "Ctrl+Shift+F".
pub fn is_valid_key_spec(spec: &str) -> bool {
    let parts: Vec<&str> = spec.split('+').collect();
    if parts.len() < 2 {
        return false;
    }
    let Some((key, modifiers)) = parts.split_last() else {
        return false;
    };
    if key.trim().is_empty() {
        return false;
    }
    let valid_modifiers = ["Ctrl", "Alt", "Shift", "Meta"];
    !modifiers.is_empty() && modifiers.iter().all(|m| valid_modifiers.contains(m))
}

/// Returns key specs that appear more than once across all keybinding actions.
/// Comparison is case-insensitive.
pub fn duplicate_key_specs(kb: &KeybindingsConfig) -> Vec<String> {
    let specs = [
        &kb.save,
        &kb.new_note,
        &kb.command_palette,
        &kb.full_search,
        &kb.view_mode_toggle,
        &kb.sidebar_toggle,
        &kb.settings,
    ];
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut dups: Vec<String> = Vec::new();
    for spec in specs {
        let lower = spec.to_lowercase();
        if !seen.insert(lower) && !dups.iter().any(|d: &String| d.eq_ignore_ascii_case(spec)) {
            dups.push(spec.clone());
        }
    }
    dups
}

pub fn load_config_from_path(path: &std::path::Path) -> Config {
    let content = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Config::default(),
        Err(_) => return Config::default(),
    };
    match toml::from_str(&content) {
        Ok(c) => c,
        Err(_) => {
            eprintln!("config.toml: parse error, using defaults");
            Config::default()
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub general: GeneralConfig,
    pub editor: EditorConfig,
    pub preview: PreviewConfig,
    pub revision: RevisionConfig,
    pub sanitize: SanitizeConfig,
    pub keybindings: KeybindingsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    pub restore_last_vault: bool,
    pub autosave_delay_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct EditorConfig {
    pub show_line_numbers: bool,
    pub word_wrap: bool,
    pub tab_size: u32,
    pub use_spaces: bool,
    pub vim_mode: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PreviewConfig {
    pub default_view: String,
    pub sync_scroll: bool,
    pub render_mermaid: bool,
    pub render_math: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RevisionConfig {
    pub naming: String,
    pub confirm_before_execute: bool,
    pub excluded_dirs: Vec<String>,
}

impl Default for RevisionConfig {
    fn default() -> Self {
        Self {
            naming: "increment".to_owned(),
            confirm_before_execute: true,
            excluded_dirs: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SanitizeConfig {
    /// 第1層 不可視文字検出の有効/無効（デフォルト ON）。
    pub warn_invisible_chars: bool,
}

impl Default for SanitizeConfig {
    fn default() -> Self {
        Self {
            warn_invisible_chars: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct KeybindingsConfig {
    pub save: String,
    pub new_note: String,
    pub command_palette: String,
    pub full_search: String,
    pub view_mode_toggle: String,
    pub sidebar_toggle: String,
    pub settings: String,
}

impl Default for KeybindingsConfig {
    fn default() -> Self {
        Self {
            save: "Ctrl+S".to_owned(),
            new_note: "Ctrl+N".to_owned(),
            command_palette: "Ctrl+P".to_owned(),
            full_search: "Ctrl+Shift+F".to_owned(),
            view_mode_toggle: "Ctrl+E".to_owned(),
            sidebar_toggle: "Ctrl+B".to_owned(),
            settings: "Ctrl+,".to_owned(),
        }
    }
}

// 非ゼロ・非 false 既定値があるため #[derive(Default)] ではなく手動実装（A5）。
impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            restore_last_vault: true,
            autosave_delay_ms: 3000,
        }
    }
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            show_line_numbers: true,
            word_wrap: true,
            tab_size: 4,
            use_spaces: true,
            vim_mode: false,
        }
    }
}

impl Default for PreviewConfig {
    fn default() -> Self {
        Self {
            default_view: "split".to_owned(),
            sync_scroll: true,
            render_mermaid: true,
            render_math: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_values_match_spec() {
        let c = Config::default();
        assert!(c.general.restore_last_vault);
        assert_eq!(c.general.autosave_delay_ms, 3000);

        assert!(c.editor.show_line_numbers);
        assert!(c.editor.word_wrap);
        assert_eq!(c.editor.tab_size, 4);
        assert!(c.editor.use_spaces);
        assert!(!c.editor.vim_mode);

        assert_eq!(c.preview.default_view, "split");
        assert!(c.preview.sync_scroll);
        assert!(c.preview.render_mermaid);
        assert!(c.preview.render_math);
    }

    #[test]
    fn empty_toml_yields_defaults() {
        let c: Config = toml::from_str("").unwrap();
        assert_eq!(c.general.autosave_delay_ms, 3000);
        assert_eq!(c.editor.tab_size, 4);
        assert_eq!(c.preview.default_view, "split");
        assert!(c.editor.show_line_numbers);
        assert!(c.preview.render_mermaid);
    }

    #[test]
    fn sanitize_warn_invisible_chars_defaults_true() {
        let c = Config::default();
        assert!(c.sanitize.warn_invisible_chars);
    }

    #[test]
    fn sanitize_section_parsed_from_toml() {
        let c: Config = toml::from_str("[sanitize]\nwarn_invisible_chars = false").unwrap();
        assert!(!c.sanitize.warn_invisible_chars);
    }

    #[test]
    fn load_config_from_path_nonexistent_returns_defaults() {
        let c = load_config_from_path(std::path::Path::new(
            "/nonexistent/__qwert_test__/config.toml",
        ));
        assert_eq!(c.general.autosave_delay_ms, 3000);
        assert_eq!(c.revision.naming, "increment");
        assert!(c.sanitize.warn_invisible_chars);
    }

    #[test]
    fn load_config_from_path_valid_toml_parses() {
        use std::io::Write as _;
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(
            tmp,
            "[revision]\nnaming = \"date\"\nexcluded_dirs = [\"archive\"]"
        )
        .unwrap();
        let c = load_config_from_path(tmp.path());
        assert_eq!(c.revision.naming, "date");
        assert_eq!(c.revision.excluded_dirs, vec!["archive"]);
    }

    #[test]
    fn load_config_from_path_invalid_toml_returns_defaults() {
        use std::io::Write as _;
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(tmp, "this is not valid toml = {{").unwrap();
        let c = load_config_from_path(tmp.path());
        assert_eq!(c.general.autosave_delay_ms, 3000);
        assert_eq!(c.revision.naming, "increment");
    }

    // ── keybindings ───────────────────────────────────────────────────────────

    #[test]
    fn keybindings_default_values_match_spec() {
        let kb = KeybindingsConfig::default();
        assert_eq!(kb.save, "Ctrl+S");
        assert_eq!(kb.new_note, "Ctrl+N");
        assert_eq!(kb.command_palette, "Ctrl+P");
        assert_eq!(kb.full_search, "Ctrl+Shift+F");
        assert_eq!(kb.view_mode_toggle, "Ctrl+E");
        assert_eq!(kb.sidebar_toggle, "Ctrl+B");
        assert_eq!(kb.settings, "Ctrl+,");
    }

    #[test]
    fn keybindings_empty_toml_yields_defaults() {
        let c: Config = toml::from_str("").unwrap();
        assert_eq!(c.keybindings, KeybindingsConfig::default());
    }

    #[test]
    fn keybindings_parsed_from_toml() {
        let toml = "[keybindings]\nsave = \"Ctrl+Alt+S\"\nnew_note = \"Ctrl+N\"";
        let c: Config = toml::from_str(toml).unwrap();
        assert_eq!(c.keybindings.save, "Ctrl+Alt+S");
        assert_eq!(c.keybindings.new_note, "Ctrl+N");
        // unspecified keys fall back to defaults
        assert_eq!(c.keybindings.command_palette, "Ctrl+P");
    }

    #[test]
    fn is_valid_key_spec_accepts_valid_specs() {
        assert!(is_valid_key_spec("Ctrl+S"));
        assert!(is_valid_key_spec("Ctrl+Shift+F"));
        assert!(is_valid_key_spec("Ctrl+,"));
        assert!(is_valid_key_spec("Ctrl+N"));
        assert!(is_valid_key_spec("Alt+F4"));
        assert!(is_valid_key_spec("Ctrl+Alt+Delete"));
    }

    #[test]
    fn is_valid_key_spec_rejects_invalid_specs() {
        assert!(!is_valid_key_spec("S"));           // no modifier
        assert!(!is_valid_key_spec("Ctrl+"));       // empty key
        assert!(!is_valid_key_spec(""));            // empty string
        assert!(!is_valid_key_spec("Ctrl"));        // modifier only, no key part
        assert!(!is_valid_key_spec("Bad+S"));       // unknown modifier
    }

    #[test]
    fn duplicate_key_specs_detects_dups() {
        let mut kb = KeybindingsConfig::default();
        kb.new_note = "Ctrl+S".to_owned(); // duplicate of save
        let dups = duplicate_key_specs(&kb);
        assert!(dups.iter().any(|d| d.eq_ignore_ascii_case("Ctrl+S")));
    }

    #[test]
    fn duplicate_key_specs_none_for_defaults() {
        let kb = KeybindingsConfig::default();
        assert!(duplicate_key_specs(&kb).is_empty());
    }

    #[test]
    fn save_and_reload_keybindings_roundtrip() {
        use std::io::Write as _;
        // Build a config, serialize to a temp file, reload, verify
        let mut config = Config::default();
        config.keybindings.save = "Ctrl+Alt+S".to_owned();

        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        let content = toml::to_string_pretty(&config).unwrap();
        write!(tmp, "{content}").unwrap();

        let reloaded = load_config_from_path(tmp.path());
        assert_eq!(reloaded.keybindings.save, "Ctrl+Alt+S");
        assert_eq!(reloaded.keybindings.new_note, "Ctrl+N");
    }
}
