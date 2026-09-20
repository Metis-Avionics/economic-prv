use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum DataError {
    #[error("IO error: {0}")]
    IoError(String),
    #[error("Parse error: {0}")]
    ParseError(String),
}

pub type DataResult<T> = Result<T, DataError>;
