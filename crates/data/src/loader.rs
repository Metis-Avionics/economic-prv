use crate::DataError;
use csv::ReaderBuilder;
use serde::{Deserialize, Serialize};
use std::fs::File;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DataFrame {
    pub columns: Vec<String>,
    pub data: Vec<Vec<f64>>,
    pub row_count: usize,
}

impl DataFrame {
    #[must_use]
    pub const fn new(columns: Vec<String>, data: Vec<Vec<f64>>) -> Self {
        let row_count = data.len();
        Self {
            columns,
            data,
            row_count,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct DataLoader;

impl DataLoader {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Loads historical data from a CSV file.
    ///
    /// # Errors
    ///
    /// Returns `DataError::IoError` if the file cannot be opened or read.
    /// Returns `DataError::ParseError` if a value cannot be parsed as f64.
    pub fn load_historical(&self, path: &str) -> Result<DataFrame, DataError> {
        let file = File::open(path).map_err(|e| DataError::IoError(e.to_string()))?;
        let mut reader = ReaderBuilder::new().from_reader(file);

        let headers = reader
            .headers()
            .map_err(|e| DataError::IoError(e.to_string()))?
            .iter()
            .map(std::string::ToString::to_string)
            .collect::<Vec<_>>();

        let mut data = Vec::new();
        for result in reader.records() {
            let record = result.map_err(|e| DataError::IoError(e.to_string()))?;
            let row: Vec<f64> = record
                .iter()
                .map(|s| {
                    s.parse::<f64>()
                        .map_err(|e| DataError::ParseError(e.to_string()))
                })
                .collect::<Result<Vec<_>, _>>()?;
            data.push(row);
        }

        Ok(DataFrame::new(headers, data))
    }

    /// Preprocesses the data frame.
    ///
    /// # Errors
    ///
    /// Returns `DataError` if preprocessing fails.
    pub const fn preprocess(&self, _data: &mut DataFrame) -> Result<(), DataError> {
        Ok(())
    }

    /// Converts the data frame to observation vectors.
    ///
    /// # Errors
    ///
    /// Returns `DataError` if conversion fails.
    pub const fn to_observations(
        &self,
        _data: &DataFrame,
    ) -> Result<Vec<prv_core::Observation<10>>, DataError> {
        Ok(Vec::new())
    }
}
