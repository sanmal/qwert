use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum OutputFormat {
    Json,
    Path,
    Text,
    Raw,
    /// Unified diff output (only meaningful for `note revision --dry-run`)
    Diff,
    /// Tab-separated values; tabular commands output one row per result.
    /// Non-tabular commands (render, revision, status) fall back to text.
    Tsv,
}

/// Merge `schema_version` + `kind` with payload fields at the top level.
/// The payload must be a JSON object; its fields are merged directly (no `data` wrapper).
pub fn make_envelope(kind: &str, payload: Value) -> Value {
    let mut base = serde_json::json!({
        "schema_version": "v1",
        "kind": kind,
    });
    if let (Value::Object(ref mut b), Value::Object(p)) = (&mut base, payload) {
        b.extend(p);
    }
    base
}

pub fn to_json_string(v: &Value) -> String {
    serde_json::to_string_pretty(v).unwrap_or_else(|e| format!("{{\"error\":\"{e}\"}}"))
}

/// Escape a single TSV field value.
///
/// `\`, `\t`, `\n`, `\r` are replaced by two-character backslash sequences so
/// values never break column or row boundaries.
pub fn tsv_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str(r"\\"),
            '\t' => out.push_str(r"\t"),
            '\n' => out.push_str(r"\n"),
            '\r' => out.push_str(r"\r"),
            other => out.push(other),
        }
    }
    out
}

/// Join escaped fields with `\t` and return a single TSV row (no trailing newline).
pub fn tsv_row(fields: impl IntoIterator<Item = impl AsRef<str>>) -> String {
    fields
        .into_iter()
        .map(|f| tsv_escape(f.as_ref()))
        .collect::<Vec<_>>()
        .join("\t")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_has_schema_version_and_kind() {
        let v = make_envelope("file_list", serde_json::json!({"paths": ["a.md"]}));
        assert_eq!(v["schema_version"], "v1");
        assert_eq!(v["kind"], "file_list");
    }

    #[test]
    fn no_data_wrapper() {
        let v = make_envelope("file_list", serde_json::json!({"paths": ["a.md"]}));
        assert!(v.get("data").is_none(), "data wrapper must not exist: {v}");
    }

    #[test]
    fn payload_fields_at_top_level() {
        let v = make_envelope("test", serde_json::json!({"paths": ["x.md"], "count": 1}));
        assert!(v["paths"].is_array());
        assert_eq!(v["count"], 1);
        assert!(v.get("data").is_none());
    }

    #[test]
    fn format_variants_are_distinct() {
        assert_ne!(OutputFormat::Json, OutputFormat::Path);
        assert_ne!(OutputFormat::Raw, OutputFormat::Text);
        assert_ne!(OutputFormat::Json, OutputFormat::Raw);
        assert_ne!(OutputFormat::Tsv, OutputFormat::Json);
    }

    // ── tsv_escape ────────────────────────────────────────────────────────────

    #[test]
    fn tsv_escape_plain_string_unchanged() {
        assert_eq!(tsv_escape("hello world"), "hello world");
    }

    #[test]
    fn tsv_escape_empty_string() {
        assert_eq!(tsv_escape(""), "");
    }

    #[test]
    fn tsv_escape_tab_becomes_backslash_t() {
        assert_eq!(tsv_escape("a\tb"), r"a\tb");
    }

    #[test]
    fn tsv_escape_newline_becomes_backslash_n() {
        assert_eq!(tsv_escape("a\nb"), r"a\nb");
    }

    #[test]
    fn tsv_escape_carriage_return_becomes_backslash_r() {
        assert_eq!(tsv_escape("a\rb"), r"a\rb");
    }

    #[test]
    fn tsv_escape_backslash_is_doubled() {
        assert_eq!(tsv_escape(r"a\b"), r"a\\b");
    }

    #[test]
    fn tsv_escape_all_special_chars() {
        // backslash + tab + newline + cr combined
        assert_eq!(tsv_escape("\\\t\n\r"), r"\\\t\n\r");
    }

    // ── tsv_row ───────────────────────────────────────────────────────────────

    #[test]
    fn tsv_row_single_field() {
        assert_eq!(tsv_row(["path"]), "path");
    }

    #[test]
    fn tsv_row_multiple_fields_joined_with_tab() {
        assert_eq!(tsv_row(["path", "line", "snippet"]), "path\tline\tsnippet");
    }

    #[test]
    fn tsv_row_escapes_embedded_tab_in_field() {
        // field value "a\tb" must not split the column
        assert_eq!(tsv_row(["a\tb"]), r"a\tb");
    }

    #[test]
    fn tsv_row_accepts_owned_strings() {
        let fields = vec!["alpha".to_owned(), "beta".to_owned()];
        assert_eq!(tsv_row(fields), "alpha\tbeta");
    }
}
