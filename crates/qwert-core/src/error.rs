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
}

pub type Result<T> = std::result::Result<T, CoreError>;
