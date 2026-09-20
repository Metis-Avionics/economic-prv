# prv-data

CSV/JSON data loading, preprocessing, and `DataFrame` abstractions for the `prv` workspace.

## Overview

`prv-data` handles ingestion and representation of economic time-series data:

- `DataLoader` — load data from CSV or JSON sources
- `DataFrame` — tabular data abstraction aligned with `State` vectors
- `DataError` — error type for data access failures

The crate bridges raw observational data with `prv-core` state representations.

## Required Series

CSV files must include the following columns (case-insensitive):

| Category | Columns |
|----------|---------|
| Required (10) | `gdp_growth`, `inflation`, `unemployment`, `interest_rate`, `fiscal_balance`, `current_account`, `housing_price_index`, `consumer_confidence`, `investment_flow`, `exchange_rate` |
| Geopolitical (3) | `geopolitical_tension_index`, `sanctions_exposure`, `alliance_stability` |

## Usage

```rust
use prv_data::DataLoader;

let loader = DataLoader::new();
let df = loader.load_historical("examples/faux_data.csv")?;
println!("Loaded {} rows x {} columns", df.row_count, df.columns.len());

let observations = loader.to_observations(&df)?;
println!("Generated {} observations", observations.len());
```

## Preprocessing

`DataLoader::preprocess` applies:
- Normalization (z-score)
- Outlier marking (>3σ → `NaN`)
- Missing value handling

## Dependencies

- `prv-core` — state type mapping
- `csv` — CSV parsing
- `serde` / `serde_json` — JSON serialization
- `chrono` — time handling
- `thiserror` — error definitions

## License

MIT © Metis Avionics
