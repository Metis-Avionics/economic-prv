use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum DataError {
    #[error("IO error: {0}")]
    IoError(String),
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

pub type DataResult<T> = Result<T, DataError>;
