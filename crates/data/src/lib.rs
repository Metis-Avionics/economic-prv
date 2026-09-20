#![forbid(unsafe_code)]
#![warn(clippy::pedantic, clippy::nursery)]

pub mod error;
pub mod loader;

pub use error::DataError;
pub use loader::{DataFrame, DataLoader};

pub use prv_core::State;
