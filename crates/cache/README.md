# prv-cache

Six-tier cache wiring over `thesix` with read-heavy stats and key indexing for the `prv` workspace.

## Overview

`prv-cache` provides the caching layer for PRV pipeline operations. Application code never selects tiers directly — every operation goes through [`NsCache`], which delegates routing to the `thesix` policy engine + `Cachelito` control plane.

This crate adds two read-heavy companions backed by [`dashmap::DashMap`] (lock-free sharded reads, no `RwLock<Mutex<…>>` layering):

- [`CacheStats`] — per-namespace operation counters.
- [`KeyIndex`] — key → subject index for pattern-scoped lookups.

## Usage

```rust
use prv_cache::{NsCache, keys};

let cache = NsCache::new();
let ctx = NsCache::ctx("worker", vec!["solver".into()], "tenant-a");

let key = keys::simulation_results_key("case-42");
cache.set(&key, "result-data".into(), &ctx).await?;

let cached = cache.get(&key, &ctx).await?;
```

## Canonical Keys

| Namespace | Helper | Example |
|-----------|--------|---------|
| `simulation` | `simulation_results_key(case_id)` | `simulation:results:case-42` |
| `ekf` | `ekf_state_key(timestamp)` | `ekf:state:2024-01-01` |
| `data` | `historical_data_key(source)` | `data:historical:csv` |
| `policy` | `policy_decision_key(regime)` | `policy:decision:expansion` |

## Dependencies

- `thesix` — six-tier cache abstraction (L0..L5 stubs, Redis/Sled backends)
- `dashmap` — concurrent read-heavy stats and index
- `tokio` — async `JoinSet` for batch warm/invalidate
- `serde` / `serde_json` — snapshot serialization
- `tracing` — telemetry

## License

MIT © Metis Avionics
