// Error type shared by all Clipsa modules.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClipsaError {
    #[error("Clipboard error: {0}")]
    Clipboard(String),

    #[error("Encryption error: {0}")]
    Encryption(String),

    #[error("Decryption error: {0}")]
    Decryption(String),

    #[error("Key management error: {0}")]
    KeyManagement(String),

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Invalid setting: {0}")]
    InvalidSetting(String),

    #[error("Platform error: {0}")]
    Platform(String),

    /// A refused request whose message is shown to the user verbatim.
    #[error("{0}")]
    Rejected(String),
    /// History commands before the first-run screen created the key.
    #[error("Clipsa isn't set up yet. Finish the welcome screen first.")]
    NotSetUp,
    #[error("Application startup failed: {0}")]
    ApplicationStartup(String),
}

pub type ClipsaResult<T> = Result<T, ClipsaError>;
