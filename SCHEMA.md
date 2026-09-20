# PRV HelixDB Schema

This document describes the HelixDB v0.0.5 schema for the PRV workspace.
The schema is validated by `scripts/check_schema.sh` in CI.

## Labels

| Label | Description |
|---|---|
| `SimulationCase` | A Monte Carlo simulation run with seed, horizon, and path count |
| `StateVector` | An 8-dimensional state vector at a point in time |
| `ShockSpec` | A shock specification with type, amplitude, persistence, autocorrelation |
| `MonteCarloPath` | A single simulated path through time |
| `PolicyDecision` | A policy instrument recommendation with score |
| `EvaluationResult` | A backtesting evaluation result with metrics |

## Edges

| Edge | From | To | Description |
|---|---|---|---|
| `HAS_STATE` | `SimulationCase` | `StateVector` | Links a case to its initial state |
| `SHOCKED_WITH` | `MonteCarloPath` | `ShockSpec` | Links a path to the shocks applied |
| `SIMULATED` | `SimulationCase` | `MonteCarloPath` | Links a case to its simulated paths |
| `POLICY_APPLIED` | `EvaluationResult` | `PolicyDecision` | Links an evaluation to its policy decisions |
| `EVALUATED_AS` | `MonteCarloPath` | `EvaluationResult` | Links a path to its evaluation result |

## Example Query

See `examples/helix-query.json` for a sample read query.
