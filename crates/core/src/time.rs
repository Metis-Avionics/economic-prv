use crate::PrvError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TimeSeries<T> {
    pub timestamps: Vec<DateTime<Utc>>,
    pub values: Vec<T>,
}

impl<T: Clone> Clone for TimeSeries<T> {
    fn clone(&self) -> Self {
        Self {
            timestamps: self.timestamps.clone(),
            values: self.values.clone(),
        }
    }
}

impl<T> TimeSeries<T> {
    /// Creates a new time series.
    ///
    /// # Errors
    ///
    /// Returns `PrvError::InvalidTimeSeries` if timestamps and values have different lengths.
    pub fn new(timestamps: Vec<DateTime<Utc>>, values: Vec<T>) -> Result<Self, PrvError> {
        if timestamps.len() != values.len() {
            return Err(PrvError::InvalidTimeSeries {
                timestamps: timestamps.len(),
                values: values.len(),
            });
        }
        Ok(Self { timestamps, values })
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.timestamps.len()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.timestamps.is_empty()
    }

    #[must_use]
    pub fn interpolate(&self, _timestamp: DateTime<Utc>) -> Option<&T> {
        self.values.first()
    }

    #[must_use]
    pub fn resample(&self, _frequency: &str) -> Self
    where
        T: Clone,
    {
        self.clone()
    }
}
