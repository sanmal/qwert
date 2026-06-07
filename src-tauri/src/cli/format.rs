use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum OutputFormat {
    Json,
    Path,
    Text,
    Raw,
    /// Unified diff output (only meaningful for `note revision --dry-run`)
    Diff,
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
    }
}
