use crate::PrvError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TimeSeries<T> {
    pub timestamps: Vec<DateTime<Utc>>,
    pub values: Vec<T>,
    pub frequency: Option<String>,
}

impl<T: Clone> Clone for TimeSeries<T> {
    fn clone(&self) -> Self {
        Self {
            timestamps: self.timestamps.clone(),
            values: self.values.clone(),
            frequency: self.frequency.clone(),
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
        Ok(Self {
            timestamps,
            values,
            frequency: None,
        })
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
    pub fn with_frequency(mut self, frequency: impl Into<String>) -> Self {
        self.frequency = Some(frequency.into());
        self
    }

    #[must_use]
    pub fn frequency(&self) -> Option<&str> {
        debug_assert!(
            self.timestamps.is_empty() || self.timestamps.len() == self.values.len(),
            "time series lengths must match"
        );
        self.frequency.as_deref()
    }

    #[must_use]
    pub fn interpolate(&self, _timestamp: DateTime<Utc>) -> Option<&T> {
        debug_assert!(
            self.timestamps.is_empty() || self.timestamps.len() == self.values.len(),
            "time series lengths must match"
        );
        self.values.first()
    }

    /// Validates that timestamps are quarterly spaced.
    ///
    /// # Errors
    ///
    /// Returns `PrvError::InvalidFrequency` if the spacing between timestamps
    /// is not approximately quarterly (90/91 days).
    pub fn validate_quarterly(&self) -> Result<(), PrvError> {
        if self.timestamps.len() < 2 {
            return Ok(());
        }
        for window in self.timestamps.windows(2) {
            let delta = window[1] - window[0];
            let months = delta.num_days() / 30;
            if months != 3 && delta.num_days() != 90 && delta.num_days() != 91 {
                return Err(PrvError::InvalidFrequency {
                    got: format!("{delta:?}"),
                });
            }
        }
        Ok(())
    }

    #[must_use]
    pub fn resample(&self, frequency: &str) -> Self
    where
        T: Clone,
    {
        let mut resampled = self.clone();
        resampled.frequency = Some(frequency.to_string());
        resampled
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_series_new_validates_lengths() {
        let ts = TimeSeries::new(vec![Utc::now()], vec![1.0_f64]);
        assert!(ts.is_ok());
        let ts = TimeSeries::new(vec![Utc::now()], Vec::<f64>::new());
        assert!(ts.is_err());
    }

    #[test]
    fn time_series_with_frequency_sets_frequency() {
        let ts = TimeSeries::new(vec![Utc::now()], vec![1.0])
            .unwrap()
            .with_frequency("quarterly");
        assert_eq!(ts.frequency(), Some("quarterly"));
    }

    #[test]
    fn time_series_resample_updates_frequency() {
        let ts = TimeSeries::new(vec![Utc::now()], vec![1.0])
            .unwrap()
            .with_frequency("monthly");
        let resampled = ts.resample("quarterly");
        assert_eq!(resampled.frequency(), Some("quarterly"));
    }

    #[test]
    fn time_series_validate_quarterly_passes_for_quarterly_data() {
        let timestamps = vec![
            Utc::now(),
            Utc::now() + chrono::Duration::days(90),
            Utc::now() + chrono::Duration::days(180),
        ];
        let ts = TimeSeries::new(timestamps, vec![1.0, 2.0, 3.0]).unwrap();
        assert!(ts.validate_quarterly().is_ok());
    }

    #[test]
    fn time_series_validate_quarterly_fails_for_monthly_data() {
        let timestamps = vec![
            Utc::now(),
            Utc::now() + chrono::Duration::days(30),
            Utc::now() + chrono::Duration::days(60),
        ];
        let ts = TimeSeries::new(timestamps, vec![1.0, 2.0, 3.0]).unwrap();
        assert!(ts.validate_quarterly().is_err());
    }
}
