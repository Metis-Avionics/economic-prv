# prv-data

CSV/JSON data loading and DataFrame abstractions for the `prv` workspace.

## Overview

`prv-data` handles ingestion and representation of economic time-series data:

- `DataLoader` — load data from CSV or JSON sources
- `DataFrame` — tabular data abstraction aligned with `State` vectors
- `DataError` — error type for data access failures

The crate bridges raw observational data with `prv-core` state representations.

## Dependencies

- `prv-core` — state type mapping
- `csv` — CSV parsing
- `serde` / `serde_json` — JSON serialization
- `chrono` — time handling
- `thiserror` / `tracing` — errors and logging

## License

MIT © Metis Avionics
