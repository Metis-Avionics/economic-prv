# Plan: Zero Clippy Warnings Across the PRV Workspace

## Goal
Eliminate all 47 clippy warnings so `cargo clippy --workspace --all-targets`
produces zero warnings and zero errors. The TETANUS checker already reports
0 errors (16 density warnings, out of scope for this plan).

## Current State (from /tmp/clippy.log, 823 lines)

| Crate | File | Warnings | Categories |
|-------|------|----------|------------|
| prv-cache | `crates/cache/src/manager.rs` | 8 | must_use_candidate(4), expect_used(1), missing_errors_doc(1), shadow_reuse(2) |
| prv-data | `crates/data/src/cache.rs` | 2 | let_underscore_must_use(2) |
| prv-data | `crates/data/src/loader.rs` | 3 | doc_markdown(1), cast_precision_loss(1), shadow_reuse(1) |
| prv-filter | `crates/filter/src/transition.rs` | 1 | manual_assert_eq(1) |
| prv-monte_carlo | `crates/monte_carlo/src/simulator.rs` | 9 | single_match_else(1), unused_self(3), option_if_let_else(1), cast_precision_loss(2), cast_possible_truncation(1), cast_sign_loss(1) |
| prv-policy | `crates/policy/src/engine.rs` | 11 | suboptimal_flops(7), struct_field_names(1) |
| prv-evaluation | `crates/evaluation/src/metrics.rs` | 1 | shadow_reuse(1) |
| prv-evaluation | `crates/evaluation/src/backtest.rs` | 11 | cast_lossless(5), cast_possible_wrap(5) |
| prv-cli | `crates/cli/src/main.rs` | 19 | let_underscore_must_use(5), cognitive_complexity(1), shadow_reuse(4), too_many_lines(1), single_match_else(3), option_if_let_else(3) |

## Formatting (already fixed)
`cargo fmt --all` was run and resolved the `#![allow(...)]` line-wrap in
`crates/cli/src/main.rs:1`. No further formatting issues exist.

---

## Fixes by File

### 1. `crates/cache/src/manager.rs` (8 warnings)

**a. `must_use_candidate` on `new()` (line 46)**
Add `#[must_use]` before `pub fn new()`.
Note: `Default` impl already delegates to `new()`.

**b. `must_use_candidate` on `with_tiers()` (line 64)**
Add `#[must_use]` before `pub fn with_tiers()`.

**c. `expect_used` on `MemoryPool::new(1024).expect(...)` (line 70)**
Add `#[allow(clippy::expect_used)]` to the `with_tiers` method.
The doc comment already says "Panics if the MemoryPool allocation fails."

**d. `must_use_candidate` on `ctx()` (line 80)**
Add `#[must_use]` before `pub fn ctx()`.

**e. `must_use_candidate` on `anonymous_ctx()` (line 89)**
Add `#[must_use]` before `pub fn anonymous_ctx()`.

**f. `missing_errors_doc` on `get()` (line 94)**
Add a `# Errors` doc section before the `pub async fn get` line:
```rust
/// # Errors
///
/// Returns `CacheError` if the underlying cache lookup fails.
```

**g. `shadow_reuse` — `ctx` variable in `warm_many` (lines 211/216)**
In `warm_many`, rename the inner `let ctx = ctx.clone();` to `let ctx_clone = ctx.clone();`
and update the `spawn` closure: `set.spawn(async move { this.set(&key, val, &ctx_clone).await });`

**h. `shadow_reuse` — `ctx` variable in `invalidate_many` (lines 235/240)**
In `invalidate_many`, rename `let ctx = ctx.clone();` to `let ctx_clone = ctx.clone();`
and update the `spawn` closure: `set.spawn(async move { this.invalidate(&key, &ctx_clone).await });`

---

### 2. `crates/data/src/cache.rs` (2 warnings)

**a. `let_underscore_must_use` on `set()` (line 32)**
Add `#[allow(clippy::let_underscore_must_use)]` on the `set` method.
The `L0Stub::set` returns `Result<(), CacheError>` which is intentionally ignored
(fire-and-forget cache write).

**b. `let_underscore_must_use` on `clear()` (line 37)**
Add `#[allow(clippy::let_underscore_must_use)]` on the `clear` method.
Same rationale — fire-and-forget cache clear.

---

### 3. `crates/data/src/loader.rs` (3 warnings)

**a. `doc_markdown` on `NsCache` (line 93)**
Change doc comment:
`/// Loads historical data from a CSV file with NsCache backing.`
→
`/// Loads historical data from a CSV file with \`NsCache\` backing.`

**b. `cast_precision_loss` in test `preprocess_normalizes_data` (line 245)**
The test computes `df.data.len() as f64`. Add
`#[allow(clippy::cast_precision_loss)]` to the test function.

**c. `shadow_reuse` on `df` in test `dataframe_source_attribution` (lines 323/325)**
```rust
let df = DataFrame::new(vec!["a".to_string()], vec![vec![1.0]]);
assert!(df.source().is_none());
let df = df.with_source(Some("test.csv".to_string()));
```
→ Rename the second binding:
```rust
let df = DataFrame::new(vec!["a".to_string()], vec![vec![1.0]]);
assert!(df.source().is_none());
let df_with_source = df.with_source(Some("test.csv".to_string()));
assert_eq!(df_with_source.source(), Some("test.csv"));
```

---

### 4. `crates/filter/src/transition.rs` (1 warning)

**a. `manual_assert_eq` (line 57)**
```rust
assert!(next.as_vector() != state.as_vector());
```
→
```rust
assert_ne!(next.as_vector(), state.as_vector());
```
Note: `clippy::allow-expect-in-tests` and `allow-panic-in-tests` in clippy.toml
already permit test assertions.

---

### 5. `crates/monte_carlo/src/simulator.rs` (9 warnings)

**a. `single_match_else` in `sample_state` (line 57)**
Refactor `sample_state` to call `self.prepare_cholesky(covariance)?` instead of
duplicating the cholesky logic with a `match`. Replace lines 57-70:
```rust
let chol = match covariance.cholesky() {
    Some(c) => c,
    None => { ... }
};
```
with:
```rust
let chol = self.prepare_cholesky(covariance)?;
```
This eliminates the duplicated logic and the `single_match_else` warning.

**b. `unused_self` on `prepare_cholesky` (line 135)**
Add `#[allow(clippy::unused_self)]` to the `prepare_cholesky` method.

**c. `option_if_let_else` in `prepare_cholesky` (line 138)**
Change the `if let Some(c) = covariance.cholesky() { Ok(c) } else { ... }` pattern
to `map_or_else`:
```rust
covariance.cholesky().map_or_else(
    || {
        let mut cov = *covariance;
        let eigen = cov.symmetric_eigenvalues();
        let min_eig = eigen.min();
        if min_eig < 1e-10 {
            for i in 0..8 {
                cov[(i, i)] += (1e-10 - min_eig).max(0.0);
            }
        }
        cov.cholesky().ok_or(PrvError::NonPsdCovariance)
    },
    Ok,
)
```

**d. `unused_self` on `run_single_path` (line 184)**
Add `#[allow(clippy::unused_self)]` to the `run_single_path` method.

**e. `cast_precision_loss` on `compute_regime_stats` (lines 327, 335)**
Add `#[allow(clippy::unused_self, clippy::cast_precision_loss)]` to the
`compute_regime_stats` method.

**f. `cast_possible_truncation` and `cast_sign_loss` on `compute_tail_risk` (line 355)**
Update existing `#[allow(clippy::unused_self, clippy::cast_precision_loss)]`
on `compute_tail_risk` to include `clippy::cast_possible_truncation` and
`clippy::cast_sign_loss`:
```rust
#[allow(
    clippy::unused_self,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
```

---

### 6. `crates/policy/src/engine.rs` (11 warnings)

**a. `suboptimal_flops` on `interest_score` (lines 116-118)**
```rust
let interest_score = base_interest
    + self.weights.inflation * signals.inflation_signal
    + self.weights.systemic_risk * signals.systemic_signal;
```
→ Use `mul_add` for both multiply-add pairs:
```rust
let interest_score = self.weights.systemic_risk.mul_add(
    signals.systemic_signal,
    self.weights.inflation.mul_add(signals.inflation_signal, base_interest),
);
```

**b. `suboptimal_flops` on `compute_expected` (lines 262-271)**
```rust
State::new(
    state.capacity() + 0.01 * scores.fiscal,
    state.investment() + 0.02 * scores.swf,
    state.labour_absorption() + 0.01 * scores.infra,
    state.fiscal_capacity() - 0.01 * scores.tax,
    state.demand_pressure() - 0.01 * scores.interest,
    state.housing_pressure() - 0.005 * scores.interest,
    state.geopolitical_load() - 0.01 * scores.migration,
    state.migration_pressure() - 0.01 * scores.migration,
)
```
→ Replace each `+ c * x` with `c.mul_add(x, base)` and each `- c * x` with
`c.mul_add(-x, base)`:
```rust
State::new(
    0.01f64.mul_add(scores.fiscal, state.capacity()),
    0.02f64.mul_add(scores.swf, state.investment()),
    0.01f64.mul_add(scores.infra, state.labour_absorption()),
    0.01f64.mul_add(-scores.tax, state.fiscal_capacity()),
    0.01f64.mul_add(-scores.interest, state.demand_pressure()),
    0.005f64.mul_add(-scores.interest, state.housing_pressure()),
    0.01f64.mul_add(-scores.migration, state.geopolitical_load()),
    0.01f64.mul_add(-scores.migration, state.migration_pressure()),
)
```

**c. `struct_field_names` on `StateSignals` (line 324)**
Add `#[allow(clippy::struct_field_names)]` before the `struct StateSignals` definition.
All three fields (`inflation_signal`, `debt_signal`, `systemic_signal`) share
the `_signal` postfix, which is intentional for clarity.

---

### 7. `crates/evaluation/src/metrics.rs` (1 warning)

**a. `shadow_reuse` on `p` in `log_loss` (line 65)**
```rust
for (&p, &o) in probabilities.iter().zip(outcomes.iter()) {
    let p = p.clamp(1e-10, 1.0 - 1e-10);
    sum += if o { -p.ln() } else { -(1.0 - p).ln() };
}
```
→ Rename the clamped binding:
```rust
for (&p, &o) in probabilities.iter().zip(outcomes.iter()) {
    let p_clamped = p.clamp(1e-10, 1.0 - 1e-10);
    sum += if o { -p_clamped.ln() } else { -(1.0 - p_clamped).ln() };
}
```

---

### 8. `crates/evaluation/src/backtest.rs` (11 warnings, all in tests)

All warnings are in the `#[cfg(test)] mod tests` block, on `cast_lossless`
(integer-to-f64 via `i as f64`) and `cast_possible_wrap` (usize-to-i64).

**a. `cast_lossless` (5 instances at lines 479, 519, 537, 554, 571)**
5 test functions use `(0..20).map(|i| State::new(i as f64, ...))`.
Change `i as f64` → `f64::from(i)` in all 5 locations.

**b. `cast_possible_wrap` (5 instances at lines 484, 524, 559, 576)**
Same 5 test functions use `.enumerate().map(|(i, _)| chrono::Utc::now() +
chrono::Duration::days(i as i64))` where `i` is `usize`.

**Recommended approach:** Add `#[allow(clippy::cast_lossless, clippy::cast_possible_wrap)]`
to the `#[cfg(test)] mod tests` block. These are test-only warnings with small
fixed-range integers (0..20) that cannot overflow. This is the least invasive
fix across 5 nearly-identical test functions.

Alternatively, fix `cast_lossless` with `f64::from(i)` (5 edits) and add
`#[allow(clippy::cast_possible_wrap)]` to each test function. The module-level
allow is cleaner.

---

### 9. `crates/cli/src/main.rs` (19 warnings)

**a. Crate-level `let_underscore_must_use` (5 instances)**
The file has `let _ = fs::create_dir_all(...)` and `let _ = fs::write(...)`
in `update_living`, `session_handover`, and `print_status`.

Add `clippy::let_underscore_must_use` to the crate-level `#![allow(...)]`
on line 1:
```rust
#![allow(
    clippy::needless_raw_string_hashes,
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::let_underscore_must_use
)]
```

This also covers the identical warning in `crates/cli/src/cache.rs:30`
which is the same crate.

**b. `cognitive_complexity` on `validate_spec` (line 75)**
Add `clippy::cognitive_complexity` to the existing `#[allow(clippy::too_many_lines)]`
attribute:
```rust
#[allow(clippy::too_many_lines, clippy::cognitive_complexity)]
```
The function already has this attribute; just add the second lint name.

**c. `shadow_reuse` on `project`, `workspace`, `quality` in `validate_spec` (lines 134, 151, 158)**
Add `clippy::shadow_reuse` to the same `#[allow(...)]` on `validate_spec`:
```rust
#[allow(clippy::too_many_lines, clippy::cognitive_complexity, clippy::shadow_reuse)]
```

**d. `too_many_lines` on `update_living` (line 282)**
Add `#[allow(clippy::too_many_lines)]` before `fn update_living`.

**e. `shadow_reuse` on `line` in `update_living` (line 339)**
Add `#[allow(clippy::shadow_reuse)]` to `update_living` (alongside the
`too_many_lines` allow).

**f. `single_match_else` + `option_if_let_else` × 3 (lines 294, 407, 532)**
Three locations use the same pattern:
```rust
let x = match cache.get("key") {
    Some(cached) => cached,
    None => {
        let content = fs::read_to_string(path).unwrap_or_default();
        cache.set("key", content.clone());
        content
    }
};
```
Convert all three to `unwrap_or_else`:
```rust
let x = cache.get("key").unwrap_or_else(|| {
    let content = fs::read_to_string(path).unwrap_or_default();
    cache.set("key", content.clone());
    content
});
```
This fixes both `single_match_else` and `option_if_let_else` at each location.

Locations:
- Line 294 in `update_living` (key: `"cli:spec.toml"`, path: `&spec_path`)
- Line 407 in `session_handover` (key: `"cli:living.toml"`, path: `living_path`)
- Line 532 in `print_status` (key: `"cli:living.toml"`, path: `&living_path`)

---

## Validation Steps (for the implementing agent)

1. `cargo fmt --all -- --check` — should pass (0 diffs)
2. `cargo clippy --workspace --all-targets 2>/tmp/clippy.log` — should produce 0 warnings
3. `cargo test --workspace` — all tests still pass
4. `python3 scripts/tetanus-check.py` — 0 errors (warnings are pre-existing density metrics, out of scope)
5. `bash scripts/check_schema.sh` — all labels/edges present (already verified)

## Risks / Notes

- The `mul_add` rewrites in `policy/src/engine.rs` change floating-point
  evaluation order. `mul_add` uses a single FMA instruction when available,
  which may produce slightly different results. Tests should be checked
  for tolerance (they use `>= 0.0`, `<= 1.0`, `is_finite` assertions, so
  no risk).
- The `sample_state` refactor in `simulator.rs` reduces code duplication
  by reusing `prepare_cholesky`. Behavior is identical.
- The `unwrap_or_else` conversions in `main.rs` are semantically identical
  to the `match` patterns.
