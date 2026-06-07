use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub general: GeneralConfig,
    pub editor: EditorConfig,
    pub preview: PreviewConfig,
    pub revision: RevisionConfig,
    pub sanitize: SanitizeConfig,
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
}
