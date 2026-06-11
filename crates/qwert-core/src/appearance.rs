use notify::{EventKind, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

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

/// Path to a vault's scoped appearance file (`<vault>/.qwert/appearance.toml`).
/// Single source of truth shared by the loader (C1) and the watcher (C2).
pub fn vault_appearance_path(vault_root: &Path) -> PathBuf {
    vault_root.join(".qwert").join("appearance.toml")
}

pub fn load_vault_appearance(vault_root: &Path) -> crate::Result<Option<AppearanceConfig>> {
    let path = vault_appearance_path(vault_root);
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)?;
    let config: AppearanceConfig = toml::from_str(&content)?;
    Ok(Some(config))
}

/// The scope from which the effective appearance config was loaded.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AppearanceScope {
    /// Global config (`<config_dir>/appearance.toml`).
    #[default]
    Global,
    /// Vault-scoped config (`<vault>/.qwert/appearance.toml`).
    Vault,
}

impl AppearanceScope {
    pub fn as_str(self) -> &'static str {
        match self {
            AppearanceScope::Global => "global",
            AppearanceScope::Vault => "vault",
        }
    }
}

/// Deterministic scope resolution: if a vault-scoped config is present it is
/// adopted **exclusively** (no merge with global); otherwise the global config
/// is used. This two-choice rule keeps appearance resolution predictable —
/// values from the two scopes are never combined.
pub fn resolve_appearance_scope(
    vault: Option<AppearanceConfig>,
    global: AppearanceConfig,
) -> (AppearanceConfig, AppearanceScope) {
    match vault {
        Some(vault_cfg) => (vault_cfg, AppearanceScope::Vault),
        None => (global, AppearanceScope::Global),
    }
}

/// Resolve the effective appearance config and the scope it came from.
///
/// When `vault_root` is `Some` and `<vault>/.qwert/appearance.toml` exists, the
/// vault config is returned with [`AppearanceScope::Vault`] and the global
/// config is **not** read or merged. Otherwise the global config is returned
/// with [`AppearanceScope::Global`].
pub fn resolve_appearance(
    vault_root: Option<&Path>,
) -> crate::Result<(AppearanceConfig, AppearanceScope)> {
    if let Some(root) = vault_root
        && let Some(vault_cfg) = load_vault_appearance(root)?
    {
        return Ok((vault_cfg, AppearanceScope::Vault));
    }
    Ok((load_global_appearance()?, AppearanceScope::Global))
}

// ─── Hot reload (C2) ──────────────────────────────────────────────────────────

/// Debounce window for appearance hot-reload: a burst of file events is
/// coalesced and processed once after this much quiet time.
pub const APPEARANCE_DEBOUNCE: Duration = Duration::from_millis(300);

/// Trailing-edge debounce, expressed as a pure function for testing.
///
/// Given event arrival times in milliseconds (ascending) and a debounce
/// `window_ms`, return the times at which a fire would occur. Consecutive
/// events no more than `window_ms` apart belong to the same burst and produce
/// a single fire `window_ms` after the burst's last event. This mirrors the
/// runtime behaviour of [`watch_vault_appearance`] (which uses
/// `recv_timeout(window)` instead of explicit timestamps).
pub fn debounce_trailing(events_ms: &[u64], window_ms: u64) -> Vec<u64> {
    let mut fires = Vec::new();
    for (i, &t) in events_ms.iter().enumerate() {
        let burst_ends_here = match events_ms.get(i + 1) {
            // Next event arrives after the window → this event ends a burst.
            Some(&next) => next.saturating_sub(t) > window_ms,
            // Last event always ends a burst.
            None => true,
        };
        if burst_ends_here {
            fires.push(t + window_ms);
        }
    }
    fires
}

// ─── C3: error classification & fallback ─────────────────────────────────────

/// Human-readable warning from a failed appearance load/validation.
/// Extracts line/column from TOML syntax errors and key names from
/// mutual-exclusion violations so the UI can show actionable hints.
pub fn format_appearance_warning(err: &crate::CoreError) -> String {
    match err {
        // toml::de::Error Display already contains "line X, column Y".
        crate::CoreError::Toml(e) => format!("appearance.toml parse error: {e}"),
        crate::CoreError::AppearanceConflict(_) => {
            "appearance.toml conflict: conflicting keys preset, fg/bg — remove one".to_owned()
        }
        other => format!("appearance error: {other}"),
    }
}

/// Outcome of resolving the effective appearance at startup.
pub struct AppearanceResolution {
    pub config: AppearanceConfig,
    pub scope: AppearanceScope,
    /// Set when the preferred scope failed (parse / mutual-exclusion error) and
    /// the global config was used as a fallback. Surface this in the UI.
    pub warning: Option<String>,
}

/// Infallible startup resolver. On vault-config error (syntax or mutual-exclusion)
/// falls back to the global config and records a warning. The app always starts.
pub fn resolve_appearance_with_fallback(vault_root: Option<&Path>) -> AppearanceResolution {
    if let Some(root) = vault_root {
        match load_vault_appearance(root) {
            Ok(Some(vault_cfg)) => {
                // Also check mutual-exclusion (preset + fg/bg).
                if let Err(e) = validate_appearance_config(&vault_cfg) {
                    let warning = format_appearance_warning(&e);
                    return AppearanceResolution {
                        config: load_global_appearance().unwrap_or_default(),
                        scope: AppearanceScope::Global,
                        warning: Some(warning),
                    };
                }
                return AppearanceResolution {
                    config: vault_cfg,
                    scope: AppearanceScope::Vault,
                    warning: None,
                };
            }
            Ok(None) => {} // No vault file → fall through to global.
            Err(e) => {
                let warning = format_appearance_warning(&e);
                return AppearanceResolution {
                    config: load_global_appearance().unwrap_or_default(),
                    scope: AppearanceScope::Global,
                    warning: Some(warning),
                };
            }
        }
    }
    AppearanceResolution {
        config: load_global_appearance().unwrap_or_default(),
        scope: AppearanceScope::Global,
        warning: None,
    }
}

/// Validate mutual-exclusion constraint (preset and fg/bg cannot coexist).
/// Returns an error with the conflicting key names; otherwise `Ok(())`.
fn validate_appearance_config(config: &AppearanceConfig) -> crate::Result<()> {
    if config.color.preset.is_some() && (config.color.fg.is_some() || config.color.bg.is_some()) {
        return Err(crate::CoreError::AppearanceConflict(
            "preset and fg/bg cannot be set at the same time".to_owned(),
        ));
    }
    Ok(())
}

/// C3: result of a single hot-reload attempt after debounce.
pub enum AppearanceUpdate {
    /// Parse and validation succeeded — apply this config.
    Changed(Box<AppearanceConfig>),
    /// Parse or validation failed — keep the previous config and show this
    /// warning. The caller must NOT clear the previous visual state.
    Error(String),
}

/// Handle for a vault appearance watcher; dropping it stops the watch.
pub struct AppearanceWatchGuard {
    _watcher: notify::RecommendedWatcher,
}

/// Parse + validate one attempt. Used by the retry loop and in tests.
pub(crate) fn try_read_and_validate(path: &Path) -> std::result::Result<AppearanceConfig, String> {
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("cannot read appearance.toml: {e}"))?;
    let config: AppearanceConfig = toml::from_str(&content)
        .map_err(|e| format_appearance_warning(&crate::CoreError::Toml(e)))?;
    validate_appearance_config(&config).map_err(|e| format_appearance_warning(&e))?;
    Ok(config)
}

/// Retry up to 5×20 ms to tolerate mid-write partial files (direct edits).
/// After all retries returns the last error message so C3 can emit a warning.
fn read_appearance_with_retry(path: &Path) -> std::result::Result<AppearanceConfig, String> {
    const RETRIES: u32 = 5;
    const DELAY: Duration = Duration::from_millis(20);
    let mut last_err = String::from("appearance.toml not found");
    for attempt in 0..RETRIES {
        match try_read_and_validate(path) {
            Ok(c) => return Ok(c),
            Err(e) => last_err = e,
        }
        if attempt + 1 < RETRIES {
            std::thread::sleep(DELAY);
        }
    }
    Err(last_err)
}

/// Watch a vault's `.qwert/appearance.toml` for direct edits and invoke
/// `callback` with an [`AppearanceUpdate`], debounced by [`APPEARANCE_DEBOUNCE`].
///
/// - `Changed(config)` → apply the new config.
/// - `Error(warning)` → keep the previous config; show the warning transiently.
///
/// The parent `.qwert` directory is watched **non-recursively** (not the whole
/// vault) so that atomic writes (tmp → rename, which swap the file's inode) are
/// still observed, while large vaults incur no traversal cost. The callback runs
/// on a background thread (`Send + 'static`). Drop the returned guard to stop.
pub fn watch_vault_appearance<F>(
    vault_root: &Path,
    callback: F,
) -> crate::Result<AppearanceWatchGuard>
where
    F: Fn(AppearanceUpdate) + Send + 'static,
{
    let file_path = vault_appearance_path(vault_root);
    let watch_dir = file_path
        .parent()
        .ok_or_else(|| crate::CoreError::NotFound("appearance.toml parent".to_owned()))?
        .to_path_buf();
    // The directory must exist to watch it; it is qwert's own config dir.
    std::fs::create_dir_all(&watch_dir)?;

    let (tx, rx) = mpsc::channel::<()>();
    let target = file_path.clone();
    let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        if let Ok(event) = res
            && matches!(
                event.kind,
                EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_)
            )
            && event.paths.iter().any(|p| p == &target)
        {
            let _ = tx.send(());
        }
    })
    .map_err(|e| crate::CoreError::Io(std::io::Error::other(e)))?;

    watcher
        .watch(&watch_dir, RecursiveMode::NonRecursive)
        .map_err(|e| crate::CoreError::Io(std::io::Error::other(e)))?;

    std::thread::spawn(move || {
        // Trailing-edge debounce: after the first event, keep resetting the
        // timer while events keep arriving; fire once the file goes quiet.
        while rx.recv().is_ok() {
            loop {
                match rx.recv_timeout(APPEARANCE_DEBOUNCE) {
                    Ok(()) => continue,                            // still bursting → keep waiting
                    Err(mpsc::RecvTimeoutError::Timeout) => break, // quiet → fire
                    Err(mpsc::RecvTimeoutError::Disconnected) => return,
                }
            }
            // C3: propagate both success and error to the caller.
            let update = match read_appearance_with_retry(&file_path) {
                Ok(config) => AppearanceUpdate::Changed(Box::new(config)),
                Err(warning) => AppearanceUpdate::Error(warning),
            };
            callback(update);
        }
    });

    Ok(AppearanceWatchGuard { _watcher: watcher })
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

/// WCAG 2.x conformance level for a contrast ratio.
///
/// - normal text: `≥7.0`→`"AAA"`, `≥4.5`→`"AA"`, else `"fail"`.
/// - large text:  `≥4.5`→`"AAA"`, `≥3.0`→`"AA"`, else `"fail"`.
pub fn wcag_level(ratio: f64, large: bool) -> &'static str {
    if large {
        if ratio >= 4.5 {
            "AAA"
        } else if ratio >= 3.0 {
            "AA"
        } else {
            "fail"
        }
    } else if ratio >= 7.0 {
        "AAA"
    } else if ratio >= 4.5 {
        "AA"
    } else {
        "fail"
    }
}

// ─── C5: appearance status ───────────────────────────────────────────────────

/// Representative fg/bg hex colors for each built-in preset.
/// Used to compute WCAG contrast when the active config names a preset rather
/// than storing explicit hex values. Kept in core so CLI and MCP share one table.
pub fn preset_fg_bg(preset: &str) -> Option<(&'static str, &'static str)> {
    match preset {
        "default" => Some(("#1a1a1a", "#ffffff")),
        "high-contrast" => Some(("#000000", "#ffffff")),
        "dark" => Some(("#e5e7eb", "#1f2937")),
        "dark-high-contrast" => Some(("#ffffff", "#000000")),
        _ => None,
    }
}

/// Full appearance status report: effective configuration plus WCAG contrast metrics.
#[derive(Debug, Serialize)]
pub struct AppearanceStatus {
    pub schema_version: String,
    pub kind: String,
    pub scope: AppearanceScope,
    /// Active preset name (null when using custom fg/bg).
    pub preset: Option<String>,
    /// Effective foreground hex — custom value, or preset-representative if a preset
    /// is active (null when neither is set).
    pub fg: Option<String>,
    /// Effective background hex (same source as `fg`).
    pub bg: Option<String>,
    /// WCAG 2.x contrast ratio for normal text rounded to 2 decimal places
    /// (null when fg/bg cannot be determined).
    pub contrast_ratio: Option<f64>,
    /// WCAG level for normal text ("AAA" / "AA" / "fail"; null when no fg/bg).
    pub level: Option<String>,
    /// Resolved text configuration.
    pub text: TextConfig,
    /// Resolved highlight configuration.
    pub highlight: HighlightConfig,
}

/// Compute the full appearance status. Always succeeds (falls back to global on
/// vault config error, matching startup behaviour). When a preset is active, its
/// representative colors from [`preset_fg_bg`] are used for contrast calculation.
pub fn compute_appearance_status(vault_root: Option<&Path>) -> AppearanceStatus {
    let resolution = resolve_appearance_with_fallback(vault_root);
    let config = &resolution.config;

    let (fg, bg) = if let Some(ref preset) = config.color.preset {
        preset_fg_bg(preset)
            .map(|(f, b)| (Some(f.to_owned()), Some(b.to_owned())))
            .unwrap_or((None, None))
    } else {
        (config.color.fg.clone(), config.color.bg.clone())
    };

    let (contrast, level) = match (&fg, &bg) {
        (Some(f), Some(b)) => match contrast_ratio(f, b) {
            Ok(r) => {
                let r2 = (r * 100.0).round() / 100.0;
                (Some(r2), Some(wcag_level(r, false).to_owned()))
            }
            Err(_) => (None, None),
        },
        _ => (None, None),
    };

    AppearanceStatus {
        schema_version: "v1".to_owned(),
        kind: "appearance_status".to_owned(),
        scope: resolution.scope,
        preset: config.color.preset.clone(),
        fg,
        bg,
        contrast_ratio: contrast,
        level,
        text: config.text.clone(),
        highlight: config.highlight.clone(),
    }
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

/// Persist `config` to the vault-scoped `<vault>/.qwert/appearance.toml`.
/// Creates the directory if needed. Uses an atomic write (tmp → rename) so
/// the C2 watcher always reads a complete file, never a partial write.
pub fn save_vault_appearance(vault_root: &Path, config: &AppearanceConfig) -> crate::Result<()> {
    let dir = vault_root.join(".qwert");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("appearance.toml");
    let content = toml::to_string_pretty(config)?;
    // Atomic: write to a tmp file in the same directory, then rename.
    // Same-directory placement keeps the rename on one filesystem.
    let mut tmp = tempfile::NamedTempFile::new_in(&dir)?;
    use std::io::Write as _;
    tmp.write_all(content.as_bytes())?;
    tmp.persist(&path)
        .map_err(|e| crate::CoreError::Io(e.error))?;
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

    // ─── scope resolution (C1) ────────────────────────────────────────────────

    fn config_with_fg_bg(fg: &str, bg: &str) -> AppearanceConfig {
        AppearanceConfig {
            color: ColorConfig {
                fg: Some(fg.to_owned()),
                bg: Some(bg.to_owned()),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[test]
    fn vault_scope_is_used_exclusively_no_merge() {
        // vault: preset = "dark"; global: custom fg/bg.
        let vault = config_with_preset("dark");
        let global = config_with_fg_bg("#111111", "#eeeeee");

        let (resolved, scope) = resolve_appearance_scope(Some(vault), global);

        assert_eq!(scope, AppearanceScope::Vault);
        assert_eq!(resolved.color.preset.as_deref(), Some("dark"));
        // Global fg/bg must NOT leak in — no merge.
        assert_eq!(resolved.color.fg, None);
        assert_eq!(resolved.color.bg, None);
    }

    #[test]
    fn global_scope_used_when_no_vault() {
        let global = config_with_fg_bg("#111111", "#eeeeee");
        let (resolved, scope) = resolve_appearance_scope(None, global);

        assert_eq!(scope, AppearanceScope::Global);
        assert_eq!(resolved.color.fg.as_deref(), Some("#111111"));
        assert_eq!(resolved.color.bg.as_deref(), Some("#eeeeee"));
    }

    #[test]
    fn resolve_appearance_reads_vault_file_and_reports_scope() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join(".qwert")).unwrap();
        std::fs::write(
            root.join(".qwert").join("appearance.toml"),
            "[color]\npreset = \"dark\"\n",
        )
        .unwrap();

        let (resolved, scope) = resolve_appearance(Some(root)).unwrap();
        assert_eq!(scope, AppearanceScope::Vault);
        assert_eq!(resolved.color.preset.as_deref(), Some("dark"));
    }

    #[test]
    fn resolve_appearance_falls_back_to_global_scope_without_vault_file() {
        // Empty vault (no .qwert/appearance.toml) → Global scope regardless of
        // whatever the global config happens to contain in this environment.
        let dir = tempfile::tempdir().unwrap();
        let (_resolved, scope) = resolve_appearance(Some(dir.path())).unwrap();
        assert_eq!(scope, AppearanceScope::Global);
    }

    #[test]
    fn appearance_scope_as_str() {
        assert_eq!(AppearanceScope::Global.as_str(), "global");
        assert_eq!(AppearanceScope::Vault.as_str(), "vault");
        assert_eq!(AppearanceScope::default(), AppearanceScope::Global);
    }

    // ─── hot reload debounce (C2) ─────────────────────────────────────────────

    #[test]
    fn vault_appearance_path_is_dot_qwert() {
        let p = vault_appearance_path(Path::new("/vault"));
        assert!(p.ends_with(".qwert/appearance.toml"), "{p:?}");
    }

    #[test]
    fn debounce_coalesces_a_burst_into_one_fire() {
        // 5 events within the 300ms window collapse to a single trailing fire.
        let events = [0, 50, 120, 200, 280];
        let fires = debounce_trailing(&events, 300);
        assert_eq!(fires, vec![280 + 300], "burst must coalesce to one fire");
    }

    #[test]
    fn debounce_separates_bursts_beyond_the_window() {
        // Gap of >300ms between the two clusters → two fires.
        let events = [0, 100, 700, 750];
        let fires = debounce_trailing(&events, 300);
        assert_eq!(fires, vec![100 + 300, 750 + 300]);
    }

    #[test]
    fn debounce_single_event_fires_once() {
        assert_eq!(debounce_trailing(&[42], 300), vec![342]);
    }

    #[test]
    fn debounce_no_events_no_fire() {
        assert!(debounce_trailing(&[], 300).is_empty());
    }

    #[test]
    fn debounce_gap_exactly_window_stays_in_burst() {
        // next - t == window is NOT > window, so it remains one burst.
        let events = [0, 300, 600];
        let fires = debounce_trailing(&events, 300);
        assert_eq!(fires, vec![600 + 300]);
    }

    // ─── C3: invalid TOML fallback / warning ─────────────────────────────────

    #[test]
    fn syntax_error_toml_fallback_to_global_on_startup() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join(".qwert")).unwrap();
        // Write intentionally broken TOML.
        std::fs::write(
            root.join(".qwert").join("appearance.toml"),
            "this is not = {valid",
        )
        .unwrap();

        let res = resolve_appearance_with_fallback(Some(root));
        assert_eq!(
            res.scope,
            AppearanceScope::Global,
            "must fall back to global"
        );
        assert!(res.warning.is_some(), "warning must be set");
        let w = res.warning.unwrap();
        // toml error messages include line/column info.
        assert!(
            w.contains("parse error"),
            "warning should mention parse error: {w}"
        );
    }

    #[test]
    fn syntax_error_warning_contains_toml_line_info() {
        // toml::de::Error Display format includes "line X" when a position is
        // available. We verify the warning text is non-empty and actionable.
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join(".qwert")).unwrap();
        std::fs::write(
            root.join(".qwert").join("appearance.toml"),
            "[color\nbad line here",
        )
        .unwrap();

        let res = resolve_appearance_with_fallback(Some(root));
        let w = res.warning.expect("warning must be set");
        assert!(w.len() > 10, "warning should be descriptive: {w}");
        assert!(
            w.contains("appearance.toml"),
            "should reference the file: {w}"
        );
    }

    #[test]
    fn mutual_exclusion_violation_falls_back_to_global() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join(".qwert")).unwrap();
        // preset and fg/bg together — mutual-exclusion violation.
        std::fs::write(
            root.join(".qwert").join("appearance.toml"),
            "[color]\npreset = \"dark\"\nfg = \"#ffffff\"\n",
        )
        .unwrap();

        let res = resolve_appearance_with_fallback(Some(root));
        assert_eq!(res.scope, AppearanceScope::Global);
        let w = res.warning.expect("warning must be set for conflict");
        // Warning must name the conflicting keys.
        assert!(w.contains("preset"), "must mention 'preset': {w}");
        assert!(
            w.contains("fg") || w.contains("bg"),
            "must mention 'fg/bg': {w}"
        );
    }

    #[test]
    fn valid_vault_config_produces_no_warning() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join(".qwert")).unwrap();
        std::fs::write(
            root.join(".qwert").join("appearance.toml"),
            "[color]\npreset = \"dark\"\n",
        )
        .unwrap();

        let res = resolve_appearance_with_fallback(Some(root));
        assert_eq!(res.scope, AppearanceScope::Vault);
        assert!(res.warning.is_none(), "no warning for valid config");
    }

    #[test]
    fn no_vault_file_uses_global_no_warning() {
        let dir = tempfile::tempdir().unwrap();
        let res = resolve_appearance_with_fallback(Some(dir.path()));
        assert_eq!(res.scope, AppearanceScope::Global);
        assert!(res.warning.is_none());
    }

    #[test]
    fn hotreload_error_is_error_variant_not_changed() {
        // Simulate the hot-reload decision: a parse failure must produce
        // AppearanceUpdate::Error (keep-previous), NOT Changed (apply new state).
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join(".qwert")).unwrap();
        let bad_path = root.join(".qwert").join("appearance.toml");
        std::fs::write(&bad_path, "bad toml {{").unwrap();

        // Directly test the internal read-and-validate path (same logic the
        // watcher uses) — infallible wrapper confirms the Error branch.
        let result = try_read_and_validate(&bad_path);
        assert!(
            result.is_err(),
            "bad TOML must be Err (→ AppearanceUpdate::Error)"
        );
        let msg = result.unwrap_err();
        assert!(!msg.is_empty(), "error message must be non-empty");
    }

    #[test]
    fn hotreload_success_is_changed_variant() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join(".qwert")).unwrap();
        let path = root.join(".qwert").join("appearance.toml");
        std::fs::write(&path, "[color]\npreset = \"dark\"\n").unwrap();

        let result = try_read_and_validate(&path);
        assert!(
            result.is_ok(),
            "valid TOML must be Ok (→ AppearanceUpdate::Changed)"
        );
    }

    #[test]
    fn format_warning_names_conflicting_keys() {
        let err = crate::CoreError::AppearanceConflict(
            "preset and fg/bg cannot be set at the same time".to_owned(),
        );
        let w = format_appearance_warning(&err);
        assert!(w.contains("preset"), "{w}");
        assert!(w.contains("fg") || w.contains("bg"), "{w}");
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

    // ── wcag_level ────────────────────────────────────────────────────────────

    #[test]
    fn wcag_level_normal_boundaries() {
        assert_eq!(wcag_level(7.0, false), "AAA");
        assert_eq!(wcag_level(6.99, false), "AA");
        assert_eq!(wcag_level(4.5, false), "AA");
        assert_eq!(wcag_level(4.49, false), "fail");
        assert_eq!(wcag_level(3.0, false), "fail");
        assert_eq!(wcag_level(2.99, false), "fail");
    }

    #[test]
    fn wcag_level_large_boundaries() {
        assert_eq!(wcag_level(7.0, true), "AAA");
        assert_eq!(wcag_level(6.99, true), "AAA");
        assert_eq!(wcag_level(4.5, true), "AAA");
        assert_eq!(wcag_level(4.49, true), "AA");
        assert_eq!(wcag_level(3.0, true), "AA");
        assert_eq!(wcag_level(2.99, true), "fail");
    }

    // ─── C5: preset_fg_bg / compute_appearance_status ────────────────────────

    #[test]
    fn preset_fg_bg_returns_correct_colors() {
        assert_eq!(preset_fg_bg("default"), Some(("#1a1a1a", "#ffffff")));
        assert_eq!(preset_fg_bg("high-contrast"), Some(("#000000", "#ffffff")));
        assert_eq!(preset_fg_bg("dark"), Some(("#e5e7eb", "#1f2937")));
        assert_eq!(
            preset_fg_bg("dark-high-contrast"),
            Some(("#ffffff", "#000000"))
        );
    }

    #[test]
    fn preset_fg_bg_returns_none_for_unknown() {
        assert_eq!(preset_fg_bg("neon"), None);
        assert_eq!(preset_fg_bg(""), None);
    }

    #[test]
    fn status_custom_fg_bg_gives_aaa() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join(".qwert")).unwrap();
        std::fs::write(
            root.join(".qwert").join("appearance.toml"),
            "[color]\nfg = \"#1a1a1a\"\nbg = \"#ffffff\"\n",
        )
        .unwrap();

        let s = compute_appearance_status(Some(root));
        assert_eq!(s.scope, AppearanceScope::Vault);
        assert_eq!(s.fg.as_deref(), Some("#1a1a1a"));
        assert_eq!(s.bg.as_deref(), Some("#ffffff"));
        let ratio = s
            .contrast_ratio
            .expect("ratio must be Some for known fg/bg");
        assert!(ratio >= 7.0, "default fg/bg is AAA, got {ratio:.2}");
        assert_eq!(s.level.as_deref(), Some("AAA"));
    }

    #[test]
    fn status_preset_dark_has_ratio_and_level() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join(".qwert")).unwrap();
        std::fs::write(
            root.join(".qwert").join("appearance.toml"),
            "[color]\npreset = \"dark\"\n",
        )
        .unwrap();

        let s = compute_appearance_status(Some(root));
        assert_eq!(s.scope, AppearanceScope::Vault);
        assert_eq!(s.preset.as_deref(), Some("dark"));
        let ratio = s.contrast_ratio.expect("preset must yield a ratio");
        assert!(
            ratio >= 4.5,
            "dark preset should be at least AA, got {ratio:.2}"
        );
        assert!(s.level.is_some());
    }

    #[test]
    fn status_no_color_config_has_null_ratio() {
        // A vault config with no [color] section → preset/fg/bg all None.
        // Uses vault scope so the ambient global config doesn't interfere.
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join(".qwert")).unwrap();
        std::fs::write(
            root.join(".qwert").join("appearance.toml"),
            "[text]\nfont_size = 16\n",
        )
        .unwrap();

        let s = compute_appearance_status(Some(root));
        assert_eq!(s.scope, AppearanceScope::Vault);
        assert!(s.contrast_ratio.is_none(), "no-color config has no ratio");
        assert!(s.level.is_none(), "no-color config has no level");
    }

    #[test]
    fn status_schema_envelope_fields() {
        let dir = tempfile::tempdir().unwrap();
        let s = compute_appearance_status(Some(dir.path()));
        assert_eq!(s.schema_version, "v1");
        assert_eq!(s.kind, "appearance_status");
    }
}
