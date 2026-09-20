use crate::DataCache;
use crate::DataError;
use csv::ReaderBuilder;
use serde::{Deserialize, Serialize};
use std::fs::File;

use prv_cache::NsCache;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DataFrame {
    pub columns: Vec<String>,
    pub data: Vec<Vec<f64>>,
    pub row_count: usize,
    pub source: Option<String>,
}

impl DataFrame {
    #[must_use]
    pub const fn new(columns: Vec<String>, data: Vec<Vec<f64>>) -> Self {
        let row_count = data.len();
        Self {
            columns,
            data,
            row_count,
            source: None,
        }
    }

    #[must_use]
    pub fn source(&self) -> Option<&str> {
        self.source.as_deref()
    }

    #[must_use]
    pub fn with_source(mut self, source: Option<String>) -> Self {
        self.source = source;
        self
    }
}

#[derive(Clone, Debug, Default)]
pub struct DataLoader {
    cache: DataCache,
}

impl DataLoader {
    #[must_use]
    pub fn new() -> Self {
        Self {
            cache: DataCache::new(),
        }
    }

    /// Loads historical data from a CSV file.
    ///
    /// # Errors
    ///
    /// Returns `DataError::IoError` if the file cannot be opened or read.
    /// Returns `DataError::ParseError` if a value cannot be parsed as f64.
    pub fn load_historical(&self, path: &str) -> Result<DataFrame, DataError> {
        if let Some(cached) = self.cache.get(path) {
            return Ok(cached);
        }

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

        let frame = DataFrame::new(headers, data).with_source(Some(path.to_string()));
        self.cache.set(path, frame.clone());
        Ok(frame)
    }

    /// Loads historical data from a CSV file with `NsCache` backing.
    ///
    /// # Errors
    ///
    /// Returns `DataError::IoError` if the file cannot be opened or read.
    /// Returns `DataError::ParseError` if a value cannot be parsed as f64.
    /// Returns `DataError::SerializationError` if cache serialization fails.
    pub async fn load_historical_cached(
        &self,
        path: &str,
        cache: &NsCache,
        ctx: &thesix::CacheContext,
    ) -> Result<DataFrame, DataError> {
        let cache_key = format!("data:historical:{path}");
        match cache.get(&cache_key, ctx).await {
            Ok(Some(cached)) => {
                let frame: DataFrame = serde_json::from_str(&cached)
                    .map_err(|e| DataError::SerializationError(e.to_string()))?;
                return Ok(frame);
            }
            Ok(None) => {}
            Err(e) => return Err(DataError::SerializationError(e.to_string())),
        }

        let frame = self.load_historical(path)?;
        let json = serde_json::to_string(&frame)
            .map_err(|e| DataError::SerializationError(e.to_string()))?;
        cache
            .set(&cache_key, json, ctx)
            .await
            .map_err(|e| DataError::SerializationError(e.to_string()))?;
        Ok(frame)
    }

    /// Preprocesses the data frame.
    ///
    /// Applies normalization, outlier marking, and missing value handling.
    ///
    /// # Errors
    ///
    /// Returns `DataError` if preprocessing fails.
    pub fn preprocess(&self, data: &mut DataFrame) -> Result<(), DataError> {
        if data.data.is_empty() || data.columns.is_empty() {
            return Ok(());
        }

        let cols = data.columns.len();
        for col in 0..cols {
            let values: Vec<f64> = data
                .data
                .iter()
                .filter_map(|row| row.get(col).copied())
                .collect();
            let n = values.len();
            if n == 0 {
                continue;
            }

            #[allow(clippy::cast_precision_loss)]
            let mean: f64 = values.iter().sum::<f64>() / n as f64;
            #[allow(clippy::cast_precision_loss)]
            let variance: f64 = values.iter().map(|&v| (v - mean).powi(2)).sum::<f64>() / n as f64;
            let std = variance.sqrt().max(1e-10);

            for row in &mut data.data {
                if let Some(v) = row.get_mut(col) {
                    let z = (*v - mean) / std;
                    if z.abs() > 3.0 {
                        *v = f64::NAN;
                    } else {
                        *v = z;
                    }
                }
            }
        }

        Ok(())
    }

    /// Converts the data frame to observation vectors.
    ///
    /// # Errors
    ///
    /// Returns `DataError` if conversion fails.
    pub fn to_observations(
        &self,
        data: &DataFrame,
    ) -> Result<Vec<prv_core::Observation>, DataError> {
        if data.columns.len() < 10 {
            return Err(DataError::ParseError(format!(
                "DataFrame has {} columns, expected at least 10",
                data.columns.len()
            )));
        }

        let required = [
            "gdp_growth",
            "inflation",
            "unemployment",
            "interest_rate",
            "fiscal_balance",
            "current_account",
            "housing_price_index",
            "consumer_confidence",
            "investment_flow",
            "exchange_rate",
        ];
        let geopolitical = [
            "geopolitical_tension_index",
            "sanctions_exposure",
            "alliance_stability",
        ];
        let lower_columns: Vec<String> = data.columns.iter().map(|c| c.to_lowercase()).collect();
        for req in &required {
            if !lower_columns.contains(&req.to_string()) {
                return Err(DataError::ParseError(format!(
                    "Missing required series: {req}"
                )));
            }
        }
        for geo in &geopolitical {
            if !lower_columns.contains(&geo.to_string()) {
                return Err(DataError::ParseError(format!(
                    "Missing geopolitical series: {geo}"
                )));
            }
        }

        let mut observations = Vec::with_capacity(data.row_count);
        for row in &data.data {
            if row.len() < 10 {
                return Err(DataError::ParseError(
                    "Row has fewer than 10 elements".to_string(),
                ));
            }
            let values_vec = row[..10].to_vec();
            observations.push(prv_core::Observation::new(values_vec));
        }

        Ok(observations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::cast_precision_loss)]
    fn preprocess_normalizes_data() {
        let mut df = DataFrame::new(vec!["a".to_string()], vec![vec![1.0], vec![2.0], vec![3.0]]);
        let loader = DataLoader::new();
        loader.preprocess(&mut df).unwrap();
        let mean = df.data.iter().map(|row| row[0]).sum::<f64>() / df.data.len() as f64;
        assert!((mean - 0.0).abs() < 1e-10);
    }

    #[test]
    fn preprocess_marks_outliers_as_nan() {
        let mut data = vec![vec![1.0]; 100];
        data.push(vec![10000.0]);
        let mut df = DataFrame::new(vec!["a".to_string()], data);
        let loader = DataLoader::new();
        loader.preprocess(&mut df).unwrap();
        assert!(df.data[100][0].is_nan());
    }

    #[test]
    fn to_observations_requires_min_columns() {
        let df = DataFrame::new(vec!["a".to_string()], vec![vec![1.0]]);
        let loader = DataLoader::new();
        assert!(loader.to_observations(&df).is_err());
    }

    #[test]
    fn to_observations_requires_required_series() {
        let df = DataFrame::new(
            vec!["gdp_growth".to_string(), "inflation".to_string()],
            vec![vec![1.0, 2.0]],
        );
        let loader = DataLoader::new();
        assert!(loader.to_observations(&df).is_err());
    }

    #[test]
    fn to_observations_requires_geopolitical_series() {
        let df = DataFrame::new(
            vec![
                "gdp_growth".to_string(),
                "inflation".to_string(),
                "unemployment".to_string(),
                "interest_rate".to_string(),
                "fiscal_balance".to_string(),
                "current_account".to_string(),
                "housing_price_index".to_string(),
                "consumer_confidence".to_string(),
                "investment_flow".to_string(),
                "exchange_rate".to_string(),
            ],
            vec![vec![1.0; 10]],
        );
        let loader = DataLoader::new();
        assert!(loader.to_observations(&df).is_err());
    }

    #[test]
    fn to_observations_accepts_all_required_and_geopolitical_series() {
        let df = DataFrame::new(
            vec![
                "gdp_growth".to_string(),
                "inflation".to_string(),
                "unemployment".to_string(),
                "interest_rate".to_string(),
                "fiscal_balance".to_string(),
                "current_account".to_string(),
                "housing_price_index".to_string(),
                "consumer_confidence".to_string(),
                "investment_flow".to_string(),
                "exchange_rate".to_string(),
                "geopolitical_tension_index".to_string(),
                "sanctions_exposure".to_string(),
                "alliance_stability".to_string(),
            ],
            vec![vec![1.0; 13]],
        );
        let loader = DataLoader::new();
        assert!(loader.to_observations(&df).is_ok());
    }

    #[test]
    fn dataframe_source_attribution() {
        let df = DataFrame::new(vec!["a".to_string()], vec![vec![1.0]]);
        assert!(df.source().is_none());
        let df_with_source = df.with_source(Some("test.csv".to_string()));
        assert_eq!(df_with_source.source(), Some("test.csv"));
    }

    #[test]
    fn to_observations_returns_expected_dimension() {
        let df = DataFrame::new(
            vec![
                "gdp_growth".to_string(),
                "inflation".to_string(),
                "unemployment".to_string(),
                "interest_rate".to_string(),
                "fiscal_balance".to_string(),
                "current_account".to_string(),
                "housing_price_index".to_string(),
                "consumer_confidence".to_string(),
                "investment_flow".to_string(),
                "exchange_rate".to_string(),
                "geopolitical_tension_index".to_string(),
                "sanctions_exposure".to_string(),
                "alliance_stability".to_string(),
            ],
            vec![vec![1.0; 13]; 5],
        );
        let loader = DataLoader::new();
        let obs = loader.to_observations(&df).unwrap();
        assert_eq!(obs.len(), 5);
        assert_eq!(obs[0].as_vector().len(), 10);
    }
}
