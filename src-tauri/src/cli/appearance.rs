use qwert_core::appearance::{
    contrast_ratio, global_config_path, load_global_appearance, save_global_appearance,
    ALLOWED_PRESETS, APPEARANCE_TEMPLATE,
};
use qwert_core::error::ActionableError;

use crate::cli::{emit_core_error, exit_code::ExitCode, format::OutputFormat};

// ─── appearance contrast ──────────────────────────────────────────────────────

pub struct ContrastArgs {
    pub fg: String,
    pub bg: String,
    pub assert_aa: bool,
    pub assert_aaa: bool,
    pub format: OutputFormat,
}

pub fn execute_contrast(args: ContrastArgs) -> i32 {
    let ratio = match contrast_ratio(&args.fg, &args.bg) {
        Ok(r) => r,
        Err(e) => return emit_core_error(&e),
    };

    let aa = ratio >= 4.5;
    let aaa = ratio >= 7.0;

    // Assertions are checked before output; --assert-aaa takes precedence
    if args.assert_aaa && !aaa {
        let err = ActionableError::new(
            "validation",
            ExitCode::Validation as u8,
            format!("contrast ratio {ratio:.2}:1 is below WCAG AAA threshold (7.0)"),
        )
        .with_next_step("Adjust fg/bg colors to achieve contrast ratio ≥7.0");
        eprintln!("{}", serde_json::to_string_pretty(&err).unwrap_or_default());
        return ExitCode::Validation.as_i32();
    }
    if args.assert_aa && !aa {
        let err = ActionableError::new(
            "validation",
            ExitCode::Validation as u8,
            format!("contrast ratio {ratio:.2}:1 is below WCAG AA threshold (4.5)"),
        )
        .with_next_step("Adjust fg/bg colors to achieve contrast ratio ≥4.5");
        eprintln!("{}", serde_json::to_string_pretty(&err).unwrap_or_default());
        return ExitCode::Validation.as_i32();
    }

    match args.format {
        OutputFormat::Json => {
            let obj = serde_json::json!({
                "schema_version": "v1",
                "kind": "contrast_result",
                "fg": args.fg,
                "bg": args.bg,
                "ratio": round2(ratio),
                "aa": aa,
                "aaa": aaa,
            });
            println!("{}", serde_json::to_string_pretty(&obj).unwrap_or_default());
        }
        _ => {
            println!("contrast-ratio: {:.2}", ratio);
            println!("WCAG AA:  {} (≥4.5)", if aa { "pass" } else { "fail" });
            println!("WCAG AAA: {} (≥7.0)", if aaa { "pass" } else { "fail" });
        }
    }

    ExitCode::Success.as_i32()
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

// ─── appearance set ───────────────────────────────────────────────────────────

pub struct SetArgs {
    pub preset: Option<String>,
    pub fg: Option<String>,
    pub bg: Option<String>,
    pub require_aa: bool,
    pub scope: String,
    pub format: OutputFormat,
}

pub fn execute_set(args: SetArgs) -> i32 {
    if args.scope != "global" {
        let err = ActionableError::new(
            "validation",
            ExitCode::Validation as u8,
            format!(
                "--scope '{}' is not supported; only 'global' is available (vault scope is Phase 3)",
                args.scope
            ),
        )
        .with_next_step("Use --scope global or omit --scope");
        eprintln!("{}", serde_json::to_string_pretty(&err).unwrap_or_default());
        return ExitCode::Validation.as_i32();
    }

    let has_preset = args.preset.is_some();
    let has_fg = args.fg.is_some();
    let has_bg = args.bg.is_some();

    // Preset and custom colors are mutually exclusive
    if has_preset && (has_fg || has_bg) {
        let err = ActionableError::new(
            "validation",
            ExitCode::Validation as u8,
            "preset and fg/bg are mutually exclusive",
        )
        .with_next_step("Use either --preset or --fg/--bg, not both");
        eprintln!("{}", serde_json::to_string_pretty(&err).unwrap_or_default());
        return ExitCode::Validation.as_i32();
    }

    // F24: fg and bg must both be present or both absent
    if has_fg != has_bg {
        let err = ActionableError::new(
            "validation",
            ExitCode::Validation as u8,
            "fg and bg must both be specified together (F24)",
        )
        .with_next_step("Provide both --fg and --bg");
        eprintln!("{}", serde_json::to_string_pretty(&err).unwrap_or_default());
        return ExitCode::Validation.as_i32();
    }

    // Validate preset name against the allowed list
    if let Some(p) = &args.preset {
        if !ALLOWED_PRESETS.contains(&p.as_str()) {
            let err = ActionableError::new(
                "validation",
                ExitCode::Validation as u8,
                format!("unknown preset: '{p}'"),
            )
            .with_next_step(format!("Valid presets: {}", ALLOWED_PRESETS.join(", ")));
            eprintln!("{}", serde_json::to_string_pretty(&err).unwrap_or_default());
            return ExitCode::Validation.as_i32();
        }
    }

    // --require-aa: check contrast when custom fg/bg are provided
    if args.require_aa && has_fg && has_bg {
        let fg = args.fg.as_deref().unwrap_or("");
        let bg = args.bg.as_deref().unwrap_or("");
        match contrast_ratio(fg, bg) {
            Err(e) => return emit_core_error(&e),
            Ok(ratio) if ratio < 4.5 => {
                let err = ActionableError::new(
                    "validation",
                    ExitCode::Validation as u8,
                    format!("contrast ratio {ratio:.2}:1 does not meet WCAG AA (≥4.5)"),
                )
                .with_next_step("Adjust fg/bg colors to achieve contrast ratio ≥4.5");
                eprintln!("{}", serde_json::to_string_pretty(&err).unwrap_or_default());
                return ExitCode::Validation.as_i32();
            }
            Ok(_) => {}
        }
    }

    let mut config = match load_global_appearance() {
        Ok(c) => c,
        Err(e) => return emit_core_error(&e),
    };

    if let Some(ref preset) = args.preset {
        config.color.preset = Some(preset.clone());
        config.color.fg = None;
        config.color.bg = None;
    } else if has_fg && has_bg {
        config.color.preset = None;
        config.color.fg = args.fg.clone();
        config.color.bg = args.bg.clone();
    }

    if let Err(e) = save_global_appearance(&config) {
        return emit_core_error(&e);
    }

    let path = global_config_path()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "<unknown>".to_owned());

    match args.format {
        OutputFormat::Json => {
            let changes = if let Some(ref p) = args.preset {
                serde_json::json!({ "preset": p })
            } else if has_fg {
                serde_json::json!({ "fg": args.fg, "bg": args.bg })
            } else {
                serde_json::json!({})
            };
            let obj = serde_json::json!({
                "schema_version": "v1",
                "kind": "appearance_set",
                "path": path,
                "changes": changes,
            });
            println!("{}", serde_json::to_string_pretty(&obj).unwrap_or_default());
        }
        _ => {
            if let Some(ref p) = args.preset {
                println!("appearance set: preset={p} saved to {path}");
            } else if has_fg {
                println!(
                    "appearance set: fg={} bg={} saved to {path}",
                    args.fg.as_deref().unwrap_or(""),
                    args.bg.as_deref().unwrap_or("")
                );
            } else {
                println!("appearance set: no changes, config saved to {path}");
            }
        }
    }

    ExitCode::Success.as_i32()
}

// ─── appearance template ──────────────────────────────────────────────────────

pub fn execute_template(format: OutputFormat) -> i32 {
    match format {
        OutputFormat::Json => {
            let obj = serde_json::json!({
                "schema_version": "v1",
                "kind": "appearance_template",
                "content": APPEARANCE_TEMPLATE,
            });
            println!("{}", serde_json::to_string_pretty(&obj).unwrap_or_default());
        }
        _ => {
            print!("{}", APPEARANCE_TEMPLATE);
        }
    }
    ExitCode::Success.as_i32()
}
