//! prv-cache: PRV six-tier cache wiring over `thesix`.
//!
//! Application code never selects tiers — every operation goes through
//! [`NsCache`], which delegates routing to the `thesix` policy engine +
//! `Cachelito` control plane. This crate adds two read-heavy companions
//! backed by [`dashmap::DashMap`] (lock-free sharded reads, no
//! `RwLock<Mutex<…>>` layering):
//!
//! - [`CacheStats`]: per-namespace operation counters.
//! - [`KeyIndex`]: key → subject index for pattern-scoped lookups.
//!
//! Lock audit (2026-09-25): first-party `crates/` hold zero `RwLock` / `Mutex`
//! in code; enforced by `scripts/check_no_coarse_locks.sh` in CI.

pub mod index;
pub mod keys;
pub mod manager;
pub mod stats;

pub use index::KeyIndex;
pub use keys::{
    ekf_state_key, historical_data_key, namespace_of, policy_decision_key, simulation_results_key,
};
pub use manager::NsCache;
pub use stats::CacheStats;
