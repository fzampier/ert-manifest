use thiserror::Error;

/// Application error type
#[derive(Debug, Error)]
#[allow(dead_code)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("CSV error: {0}")]
    Csv(#[from] csv::Error),

    #[error("Excel error: {0}")]
    Excel(#[from] calamine::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Unsupported file format: {0}")]
    UnsupportedFormat(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Privacy violation: {0}")]
    PrivacyViolation(String),

    #[cfg(feature = "formats-readstat")]
    #[error("ReadStat error: {0}")]
    ReadStat(String),
}
