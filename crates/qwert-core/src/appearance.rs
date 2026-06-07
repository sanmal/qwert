use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AppearanceConfig {
    pub text: TextConfig,
    pub color: ColorConfig,
    pub highlight: HighlightConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TextConfig {
    pub font_size: u32,
    pub font_family: String,
    pub line_height: f32,
    pub letter_spacing: f32,
    pub word_spacing: f32,
    pub editor_max_width: u32,
}

impl Default for TextConfig {
    fn default() -> Self {
        Self {
            font_size: 16,
            font_family: "system-ui, sans-serif".to_owned(),
            line_height: 1.6,
            letter_spacing: 0.0,
            word_spacing: 0.0,
            editor_max_width: 72,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ColorConfig {
    pub preset: Option<String>,
    pub fg: Option<String>,
    pub bg: Option<String>,
    pub advanced: AdvancedColorConfig,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AdvancedColorConfig {
    #[serde(rename = "cm-keyword")]
    pub cm_keyword: Option<String>,
    #[serde(rename = "cm-string")]
    pub cm_string: Option<String>,
    #[serde(rename = "cm-comment")]
    pub cm_comment: Option<String>,
    #[serde(rename = "cm-heading")]
    pub cm_heading: Option<String>,
    #[serde(rename = "cm-link")]
    pub cm_link: Option<String>,
    pub cursor: Option<String>,
    #[serde(rename = "selection-bg")]
    pub selection_bg: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct HighlightConfig {
    pub enabled: bool,
}

impl Default for HighlightConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

pub const ALLOWED_PRESETS: &[&str] = &["default", "high-contrast", "dark", "dark-high-contrast"];

const DANGEROUS_PATTERNS: &[&str] = &[
    "url(",
    "expression(",
    "javascript:",
    "import",
    "@",
    "<",
    ">",
    "{",
    "}",
];

fn is_dangerous(value: &str) -> bool {
    let lower = value.to_lowercase();
    DANGEROUS_PATTERNS.iter().any(|p| lower.contains(p))
}

fn is_valid_color(value: &str) -> bool {
    if is_dangerous(value) {
        return false;
    }
    let v = value.trim();
    // #rgb or #rrggbb
    if let Some(hex) = v.strip_prefix('#') {
        return (hex.len() == 3 || hex.len() == 6) && hex.chars().all(|c| c.is_ascii_hexdigit());
    }
    // rgb(...) or hsl(...)
    if (v.starts_with("rgb(") || v.starts_with("hsl(")) && v.ends_with(')') {
        return true;
    }
    false
}

fn is_valid_dimension(value: &str) -> bool {
    if is_dangerous(value) {
        return false;
    }
    let v = value.trim();
    let units = ["px", "em", "rem", "ch", "%"];
    for unit in units {
        if let Some(num) = v.strip_suffix(unit) {
            return num.parse::<f64>().is_ok();
        }
    }
    // unitless number (e.g. line-height)
    v.parse::<f64>().is_ok()
}

fn is_valid_font_family(value: &str) -> bool {
    if is_dangerous(value) {
        return false;
    }
    value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == ' ' || c == '-' || c == ',')
}

pub fn load_global_appearance() -> crate::Result<AppearanceConfig> {
    let dirs = directories::ProjectDirs::from("", "", "qwert");
    let path = match dirs {
        Some(d) => d.config_dir().join("appearance.toml"),
        None => return Ok(AppearanceConfig::default()),
    };
    if !path.exists() {
        return Ok(AppearanceConfig::default());
    }
    let content = std::fs::read_to_string(&path)?;
    let config: AppearanceConfig = toml::from_str(&content)?;
    Ok(config)
}

pub fn load_vault_appearance(vault_root: &Path) -> crate::Result<Option<AppearanceConfig>> {
    let path = vault_root.join(".qwert").join("appearance.toml");
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)?;
    let config: AppearanceConfig = toml::from_str(&content)?;
    Ok(Some(config))
}

pub fn to_css_vars(config: &AppearanceConfig) -> crate::Result<HashMap<String, String>> {
    let mut map = HashMap::new();

    // preset and fg/bg are mutually exclusive
    let has_preset = config.color.preset.is_some();
    let has_custom_color = config.color.fg.is_some() || config.color.bg.is_some();
    if has_preset && has_custom_color {
        return Err(crate::CoreError::AppearanceConflict(
            "preset and fg/bg cannot be set at the same time".to_owned(),
        ));
    }

    // preset → data-theme special key
    if let Some(preset) = &config.color.preset {
        if ALLOWED_PRESETS.contains(&preset.as_str()) {
            map.insert("data-theme".to_owned(), preset.clone());
        }
    } else {
        // fg/bg custom colors
        if let Some(fg) = &config.color.fg
            && is_valid_color(fg)
        {
            map.insert("--qw-fg".to_owned(), fg.clone());
        }
        if let Some(bg) = &config.color.bg
            && is_valid_color(bg)
        {
            map.insert("--qw-bg".to_owned(), bg.clone());
        }
    }

    // advanced colors
    let adv = &config.color.advanced;
    let color_entries = [
        ("--qw-cm-keyword", &adv.cm_keyword),
        ("--qw-cm-string", &adv.cm_string),
        ("--qw-cm-comment", &adv.cm_comment),
        ("--qw-cm-heading", &adv.cm_heading),
        ("--qw-cm-link", &adv.cm_link),
        ("--qw-cursor", &adv.cursor),
        ("--qw-selection-bg", &adv.selection_bg),
    ];
    for (key, val) in &color_entries {
        if let Some(v) = val
            && is_valid_color(v)
        {
            map.insert((*key).to_owned(), v.clone());
        }
    }

    // text config
    let text = &config.text;
    let font_size_str = format!("{}px", text.font_size);
    if is_valid_dimension(&font_size_str) {
        map.insert("--qw-font-size".to_owned(), font_size_str);
    }
    if is_valid_font_family(&text.font_family) {
        map.insert("--qw-font-family".to_owned(), text.font_family.clone());
    }
    let line_height_str = format!("{}", text.line_height);
    if is_valid_dimension(&line_height_str) {
        map.insert("--qw-line-height".to_owned(), line_height_str);
    }
    let letter_spacing_str = format!("{}em", text.letter_spacing);
    if is_valid_dimension(&letter_spacing_str) {
        map.insert("--qw-letter-spacing".to_owned(), letter_spacing_str);
    }
    let word_spacing_str = format!("{}em", text.word_spacing);
    if is_valid_dimension(&word_spacing_str) {
        map.insert("--qw-word-spacing".to_owned(), word_spacing_str);
    }
    let max_width_str = format!("{}ch", text.editor_max_width);
    if is_valid_dimension(&max_width_str) {
        map.insert("--qw-editor-max-width".to_owned(), max_width_str);
    }

    Ok(map)
}

// ─── WCAG contrast calculation ────────────────────────────────────────────────

/// Parse `#rgb` or `#rrggbb` into byte components.
fn parse_hex_color(hex: &str) -> Option<(u8, u8, u8)> {
    let h = hex.trim().strip_prefix('#')?;
    match h.len() {
        3 => {
            let r = u8::from_str_radix(&h[0..1].repeat(2), 16).ok()?;
            let g = u8::from_str_radix(&h[1..2].repeat(2), 16).ok()?;
            let b = u8::from_str_radix(&h[2..3].repeat(2), 16).ok()?;
            Some((r, g, b))
        }
        6 => {
            let r = u8::from_str_radix(&h[0..2], 16).ok()?;
            let g = u8::from_str_radix(&h[2..4], 16).ok()?;
            let b = u8::from_str_radix(&h[4..6], 16).ok()?;
            Some((r, g, b))
        }
        _ => None,
    }
}

fn channel_lin(c: u8) -> f64 {
    let v = c as f64 / 255.0;
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

fn relative_luminance(r: u8, g: u8, b: u8) -> f64 {
    0.2126 * channel_lin(r) + 0.7152 * channel_lin(g) + 0.0722 * channel_lin(b)
}

/// WCAG 2.x contrast ratio between two hex colors (`#rgb` or `#rrggbb`).
///
/// Returns `CoreError::AppearanceValidation` for unparseable colors.
pub fn contrast_ratio(fg_hex: &str, bg_hex: &str) -> crate::Result<f64> {
    let (r1, g1, b1) = parse_hex_color(fg_hex).ok_or_else(|| {
        crate::CoreError::AppearanceValidation(format!("invalid hex color for fg: {fg_hex}"))
    })?;
    let (r2, g2, b2) = parse_hex_color(bg_hex).ok_or_else(|| {
        crate::CoreError::AppearanceValidation(format!("invalid hex color for bg: {bg_hex}"))
    })?;
    let l1 = relative_luminance(r1, g1, b1);
    let l2 = relative_luminance(r2, g2, b2);
    let (lighter, darker) = if l1 >= l2 { (l1, l2) } else { (l2, l1) };
    Ok((lighter + 0.05) / (darker + 0.05))
}

// ─── Persistence ──────────────────────────────────────────────────────────────

/// Absolute path to the global `appearance.toml` (may not exist yet).
pub fn global_config_path() -> Option<std::path::PathBuf> {
    directories::ProjectDirs::from("", "", "qwert").map(|d| d.config_dir().join("appearance.toml"))
}

/// Persist `config` to the global `appearance.toml`, creating the directory if needed.
pub fn save_global_appearance(config: &AppearanceConfig) -> crate::Result<()> {
    let dirs = directories::ProjectDirs::from("", "", "qwert")
        .ok_or_else(|| crate::CoreError::NotFound("config directory not found".to_owned()))?;
    let config_dir = dirs.config_dir();
    std::fs::create_dir_all(config_dir)?;
    let path = config_dir.join("appearance.toml");
    let content = toml::to_string_pretty(config)?;
    std::fs::write(path, content)?;
    Ok(())
}

/// TOML template for `appearance.toml` (no AI protocol — Phase 3).
pub const APPEARANCE_TEMPLATE: &str = r##"# qwert appearance.toml (global scope)
# Apply a preset:   qwert appearance set --preset <name>
# Custom colors:    qwert appearance set --fg '#1a1a1a' --bg '#ffffff'
#
# Available presets: default, high-contrast, dark, dark-high-contrast

[text]
font_size = 16
font_family = "system-ui, sans-serif"
line_height = 1.6
letter_spacing = 0.0
word_spacing = 0.0
editor_max_width = 72

[color]
# Choose one of:
#   preset = "default"  # default | high-contrast | dark | dark-high-contrast
# or both custom hex colors (fg and bg must be specified together – F24):
#   fg = "#1a1a1a"
#   bg = "#ffffff"

[highlight]
enabled = true
"##;

#[cfg(test)]
mod tests {
    use super::*;

    fn config_with_preset(preset: &str) -> AppearanceConfig {
        AppearanceConfig {
            color: ColorConfig {
                preset: Some(preset.to_owned()),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[test]
    fn dangerous_value_is_skipped() {
        let mut config = AppearanceConfig::default();
        config.color.fg = Some("url(http://evil.com)".to_owned());
        config.color.bg = Some("#ffffff".to_owned());
        let vars = to_css_vars(&config).unwrap();
        assert!(!vars.contains_key("--qw-fg"), "url() must be rejected");
        assert!(vars.contains_key("--qw-bg"));
    }

    #[test]
    fn javascript_in_color_is_skipped() {
        let mut config = AppearanceConfig::default();
        config.color.fg = Some("javascript:alert(1)".to_owned());
        let vars = to_css_vars(&config).unwrap();
        assert!(!vars.contains_key("--qw-fg"));
    }

    #[test]
    fn expression_in_color_is_skipped() {
        let mut config = AppearanceConfig::default();
        config.color.fg = Some("expression(alert(1))".to_owned());
        let vars = to_css_vars(&config).unwrap();
        assert!(!vars.contains_key("--qw-fg"));
    }

    #[test]
    fn invalid_font_family_is_skipped() {
        let mut config = AppearanceConfig::default();
        config.text.font_family = "url(evil)".to_owned();
        let vars = to_css_vars(&config).unwrap();
        assert!(!vars.contains_key("--qw-font-family"));
    }

    #[test]
    fn valid_hex_color_passes() {
        let mut config = AppearanceConfig::default();
        config.color.fg = Some("#1a1a1a".to_owned());
        config.color.bg = Some("#ffffff".to_owned());
        let vars = to_css_vars(&config).unwrap();
        assert_eq!(vars.get("--qw-fg").map(|s| s.as_str()), Some("#1a1a1a"));
    }

    #[test]
    fn preset_dark_returns_data_theme_key() {
        let config = config_with_preset("dark");
        let vars = to_css_vars(&config).unwrap();
        assert_eq!(vars.get("data-theme").map(|s| s.as_str()), Some("dark"));
        assert!(!vars.contains_key("--qw-preset"));
    }

    #[test]
    fn invalid_preset_is_not_emitted() {
        let config = config_with_preset("neon");
        let vars = to_css_vars(&config).unwrap();
        assert!(!vars.contains_key("data-theme"));
        assert!(!vars.contains_key("--qw-preset"));
    }

    #[test]
    fn preset_and_fg_conflict_returns_error() {
        let config = AppearanceConfig {
            color: ColorConfig {
                preset: Some("dark".to_owned()),
                fg: Some("#000000".to_owned()),
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(to_css_vars(&config).is_err());
    }

    #[test]
    fn default_text_config_values() {
        let t = TextConfig::default();
        assert_eq!(t.font_size, 16);
        assert_eq!(t.editor_max_width, 72);
        assert!((t.line_height - 1.6).abs() < f32::EPSILON);
        assert_eq!(t.font_family, "system-ui, sans-serif");
    }

    #[test]
    fn default_highlight_enabled() {
        assert!(HighlightConfig::default().enabled);
    }

    #[test]
    fn font_size_emitted_as_px() {
        let mut config = AppearanceConfig::default();
        config.text.font_size = 18;
        let vars = to_css_vars(&config).unwrap();
        assert_eq!(vars.get("--qw-font-size").map(|s| s.as_str()), Some("18px"));
    }

    #[test]
    fn all_presets_accepted() {
        for preset in ALLOWED_PRESETS {
            let config = config_with_preset(preset);
            let vars = to_css_vars(&config).unwrap();
            assert_eq!(vars.get("data-theme").map(|s| s.as_str()), Some(*preset));
        }
    }

    // ─── contrast_ratio ───────────────────────────────────────────────────────

    #[test]
    fn contrast_black_white_is_21() {
        let r = contrast_ratio("#000000", "#ffffff").unwrap();
        assert!((r - 21.0).abs() < 0.01, "expected 21:1, got {r:.4}");
    }

    #[test]
    fn contrast_white_black_is_same() {
        let a = contrast_ratio("#000000", "#ffffff").unwrap();
        let b = contrast_ratio("#ffffff", "#000000").unwrap();
        assert!((a - b).abs() < f64::EPSILON);
    }

    #[test]
    fn contrast_same_color_is_1() {
        let r = contrast_ratio("#888888", "#888888").unwrap();
        assert!((r - 1.0).abs() < 0.0001, "expected 1:1, got {r:.4}");
    }

    #[test]
    fn contrast_shorthand_hex_works() {
        let a = contrast_ratio("#000", "#fff").unwrap();
        assert!((a - 21.0).abs() < 0.01);
    }

    #[test]
    fn contrast_invalid_fg_returns_err() {
        assert!(contrast_ratio("notacolor", "#ffffff").is_err());
    }

    #[test]
    fn contrast_invalid_bg_rgb_syntax_returns_err() {
        // Only hex is supported in contrast_ratio; rgb() is rejected
        assert!(contrast_ratio("#000000", "rgb(0,0,0)").is_err());
    }

    #[test]
    fn preset_default_meets_aaa() {
        // theme-default.css: fg=#1a1a1a bg=#ffffff
        let r = contrast_ratio("#1a1a1a", "#ffffff").unwrap();
        assert!(r >= 7.0, "default preset should be AAA, got {r:.2}");
    }

    #[test]
    fn preset_high_contrast_is_21() {
        // theme-high-contrast.css: fg=#000000 bg=#ffffff
        let r = contrast_ratio("#000000", "#ffffff").unwrap();
        assert!((r - 21.0).abs() < 0.01);
    }

    #[test]
    fn preset_dark_high_contrast_is_21() {
        // theme-dark-high-contrast.css: fg=#ffffff bg=#000000
        let r = contrast_ratio("#ffffff", "#000000").unwrap();
        assert!((r - 21.0).abs() < 0.01);
    }

    #[test]
    fn preset_dark_meets_aa() {
        // theme-dark.css: fg=#e5e7eb bg=#1f2937
        let r = contrast_ratio("#e5e7eb", "#1f2937").unwrap();
        assert!(r >= 4.5, "dark preset should be AA, got {r:.2}");
    }
}
