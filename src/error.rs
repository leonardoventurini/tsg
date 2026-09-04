use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("storage error: {0}")]
    Storage(String),

    #[error("store is locked by another writer: {0}")]
    WriterLocked(String),

    #[error("operation requires a writable store")]
    ReadOnly,

    #[error("unsupported schema version {found}; maximum supported version is {supported}")]
    UnsupportedSchema { found: u32, supported: u32 },

    #[error("vector accelerator is unavailable: {0}")]
    AcceleratorUnavailable(String),

    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}
