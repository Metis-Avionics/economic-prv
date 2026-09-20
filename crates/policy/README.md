# prv-policy

Policy engine, instrument weights, and bias distributions for the `prv` workspace.

## Overview

`prv-policy` implements the policy layer of the economic simulation framework, translating model outputs into actionable policy instruments:

- `PolicyEngine` — main policy decision engine
- `PolicyInstrument` — individual policy tool definition
- `PolicyWeights` — weighting across policy instruments
- `PolicyDistribution` — distribution over policy actions
- `PolicyBias` — bias correction for policy outputs

The engine consumes `MonteCarloResults` and `Regime` classifications from upstream crates.

## Policy Instruments

| Instrument | Description |
|------------|-------------|
| `InterestRate` | Monetary policy rate adjustment |
| `QuantitativeEasing` | QE deployment scale |
| `SovereignWealthFundDeployment` | SWF liquidity injection |
| `FiscalSpending` | Government spending level |
| `Taxation` | Tax rate adjustment |
| `InfrastructureInvestment` | Infrastructure spending |
| `MigrationCapacity` | Migration intake adjustment |

## Regime Responses

| Regime | Bias |
|--------|------|
| `Expansion` | Neutral |
| `Saturation` | Contraction |
| `Contraction` | Stimulus |
| `Recovery` | Neutral |
| `StructuralShock` | Emergency |

## Usage

```rust
use prv_policy::{PolicyEngine, PolicyWeights, Regime};
use prv_monte_carlo::Simulator;

let engine = PolicyEngine::new(PolicyWeights::default());
let policy = engine.evaluate(&mc_results, &Regime::Expansion);

println!("Recommended actions: {:?}", policy.recommended_action_distribution);
println!("Constraint violations: {:?}", policy.constraint_violations);
```

## Constraints

- `QuantitativeEasing` — flagged when `inflation_signal > 2.0` and QE score > 0.5
- `SovereignWealthFundDeployment` — flagged when SWF score > 0.7 and debt signal < 0.1

## Dependencies

- `prv-core` — regime and state types
- `prv-monte-carlo` — simulation results
- `prv-filter` — EKF state estimates
- `nalgebra` — numerical types
- `serde` / `thiserror` — serialization, errors

## License

MIT © Metis Avionics
