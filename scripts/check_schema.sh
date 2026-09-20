#!/usr/bin/env bash
# check_schema.sh — verify SCHEMA.md labels and edges are reflected in code/docs.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SCHEMA="$ROOT/SCHEMA.md"
EXPECTED_LABELS=("SimulationCase" "StateVector" "ShockSpec" "MonteCarloPath" "PolicyDecision" "EvaluationResult")
EXPECTED_EDGES=("HAS_STATE" "SHOCKED_WITH" "SIMULATED" "POLICY_APPLIED" "EVALUATED_AS")
missing=0
for label in "${EXPECTED_LABELS[@]}"; do
  if ! grep -q "$label" "$SCHEMA"; then
    echo "SCHEMA.md missing label: $label" >&2
    missing=$((missing + 1))
  fi
done
for edge in "${EXPECTED_EDGES[@]}"; do
  if ! grep -q "$edge" "$SCHEMA"; then
    echo "SCHEMA.md missing edge: $edge" >&2
    missing=$((missing + 1))
  fi
done
if [ "$missing" -gt 0 ]; then
  echo "schema check: $missing missing entries" >&2
  exit 1
fi
echo "schema check: all ${#EXPECTED_LABELS[@]} labels + ${#EXPECTED_EDGES[@]} edges present"
