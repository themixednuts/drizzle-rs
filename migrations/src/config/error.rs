//! Configuration errors

/// Configuration errors
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    IoError(String),
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Generation error: {0}")]
    GenerationError(String),
    #[error("Connection error: {0}")]
    ConnectionError(String),
}
