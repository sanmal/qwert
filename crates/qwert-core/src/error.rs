use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("TOML parse error: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("TOML serialize error: {0}")]
    TomlSer(#[from] toml::ser::Error),
    #[error("Path traversal detected: {0}")]
    PathTraversal(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Appearance config conflict: {0}")]
    AppearanceConflict(String),
    #[error("Invalid pattern: {0}")]
    InvalidPattern(String),
    #[error("Appearance validation error: {0}")]
    AppearanceValidation(String),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Invalid UTF-8 at byte offset {byte_offset}")]
    InvalidUtf8 { byte_offset: usize },
}

pub type Result<T> = std::result::Result<T, CoreError>;

/// A candidate item for disambiguation (e.g. multiple wikilink matches).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candidate {
    pub value: String,
    pub label: String,
}

/// Structured error envelope for CLI/MCP output.
/// Carries actionable hints so agents can decide the next step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionableError {
    pub schema_version: String,
    pub kind: String,
    pub category: String,
    pub exit_code: u8,
    pub message: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub candidates: Vec<Candidate>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub required_args: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_step: Option<String>,
}

impl ActionableError {
    pub fn new(category: impl Into<String>, exit_code: u8, message: impl Into<String>) -> Self {
        Self {
            schema_version: "v1".to_owned(),
            kind: "error".to_owned(),
            category: category.into(),
            exit_code,
            message: message.into(),
            candidates: Vec::new(),
            required_args: Vec::new(),
            next_step: None,
        }
    }

    pub fn with_next_step(mut self, next_step: impl Into<String>) -> Self {
        self.next_step = Some(next_step.into());
        self
    }

    pub fn with_candidates(mut self, candidates: Vec<Candidate>) -> Self {
        self.candidates = candidates;
        self
    }

    pub fn with_required_args(mut self, args: Vec<String>) -> Self {
        self.required_args = args;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actionable_error_serializes_correctly() {
        let err = ActionableError::new("not_found", 3, "file not found")
            .with_next_step("Run qwert with --vault <PATH>");
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("\"schema_version\":\"v1\""));
        assert!(json.contains("\"kind\":\"error\""));
        assert!(json.contains("\"exit_code\":3"));
        assert!(json.contains("\"next_step\""));
    }

    #[test]
    fn empty_vecs_are_omitted() {
        let err = ActionableError::new("general", 1, "oops");
        let json = serde_json::to_string(&err).unwrap();
        assert!(!json.contains("candidates"));
        assert!(!json.contains("required_args"));
    }
}
