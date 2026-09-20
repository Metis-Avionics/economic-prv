use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum PrvError {
    #[error("Invalid time series: timestamps length {timestamps} != values length {values}")]
    InvalidTimeSeries { timestamps: usize, values: usize },

    #[error("Invalid state dimension: expected 8, got {got}")]
    InvalidStateDimension { got: usize },

    #[error("Covariance is not positive semidefinite")]
    NonPsdCovariance,

    #[error("State contains non-finite values")]
    NonFiniteState,

    #[error("Invalid regime threshold configuration")]
    InvalidRegimeThreshold,

    #[error("Serialization error: {0}")]
    SerializationError(String),
}
