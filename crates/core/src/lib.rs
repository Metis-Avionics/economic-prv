#![forbid(unsafe_code)]
#![warn(clippy::pedantic, clippy::nursery)]

pub mod error;
pub mod observation;
pub mod prv;
pub mod regime;
pub mod state;
pub mod time;

pub use error::PrvError;
pub use observation::Observation;
pub use prv::PressureReleaseValve;
pub use regime::Regime;
pub use state::State;
pub use time::TimeSeries;

pub use nalgebra::SVector;
pub type Control = SVector<f64, 8>;
