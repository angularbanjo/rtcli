use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("SCGI error: {0}")]
    Scgi(String),

    #[error("RPC error {code}: {message}")]
    Rpc { code: i64, message: String },

    #[error("No torrent matching '{0}'")]
    NoMatch(String),

    #[error("Ambiguous hash prefix '{0}' — matches multiple torrents")]
    AmbiguousMatch(String),
}

pub type Result<T> = std::result::Result<T, Error>;
