#!/usr/bin/env bash
# check_no_coarse_locks.sh — deny coarse-grained lock primitives in first-party crates.
#
# Policy (see crates/cache/README.md "Lock audit"): read-heavy shared maps must use
# `dashmap::DashMap` (lock-free sharded reads), never `RwLock<HashMap<…>>`,
# `Mutex<HashMap<…>>`, or the `RwLock<Mutex<…>>` nesting anti-pattern.
#
# What the gate checks under `crates/` (*.rs):
#   1. No code use of `RwLock`, `Mutex`, or `parking_lot::` outside `//` comments.
#      (`Arc<…>` is shared ownership, not a lock, and is always allowed.
#       Local `HashMap`/`BTreeMap` fields are allowed — only shared-mutable
#       state across threads falls under this policy.)
#   2. Positive check: `crates/cache` still routes shared maps through DashMap
#      (guards against regressing `CacheStats` / `KeyIndex` back to a locked map).
#
# Upstream `thesix` / `themql-cache` internals use `std::sync` locks; those live
# outside this repo and are documented as wont-fix, not gated here.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

failures=0

# 1. Negative check: coarse lock primitives in code (full-line `//` comments excluded,
#    so doc prose such as "no `RwLock<Mutex<…>>` layering" does not trip the gate).
code_hits="$(rg -n --type rust -e 'RwLock' -e 'Mutex' -e 'parking_lot::' crates/ || true)"
code_hits="$(printf '%s' "$code_hits" | grep -vE ':[0-9]+:[[:space:]]*//' || true)"
if [ -n "$code_hits" ]; then
  echo "coarse-lock check: forbidden lock primitive in first-party code:" >&2
  printf '%s\n' "$code_hits" >&2
  echo "Use dashmap::DashMap for shared maps; Arc alone (no lock) is fine." >&2
  failures=$((failures + 1))
fi

# 2. Positive check: shared cache maps still go through DashMap.
for site in "crates/cache/src/stats.rs" "crates/cache/src/index.rs"; do
  if ! rg -q 'dashmap::DashMap' "$site"; then
    echo "coarse-lock check: $site no longer uses dashmap::DashMap" >&2
    failures=$((failures + 1))
  fi
done

if [ "$failures" -gt 0 ]; then
  echo "coarse-lock check: $failures failing check(s)" >&2
  exit 1
fi
echo "coarse-lock check: no RwLock/Mutex/parking_lot in crates/ code; DashMap intact"
