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
#
# ---------------------------------------------------------------------------
# Why this file uses `grep` and not `rg`, and why there is a preflight
# ---------------------------------------------------------------------------
# The first version of this gate used `rg`, and it was never actually
# exercised: the GitHub runner has no ripgrep. That produced two failures in
# OPPOSITE directions, which is worse than either alone.
#
#   * The coarse-lock check ran `rg … || true`. A missing `rg` exits 127, `|| true`
#     swallowed it, the result was empty, and the gate reported the tree CLEAN.
#     The security-relevant check — "does any first-party crate use Mutex or
#     RwLock" — was silently passing because it could not search. It would have
#     passed a real violation too.
#
#   * The DashMap regression check ran `if ! rg -q …`. A missing `rg` fails, `!`
#     inverts that to true, and the gate reported a DashMap regression that had
#     not happened.
#
# The net exit code was 1 either way, so CI looked like it was policing
# something. It was not: half the gate was failing open and the other half was
# reporting a phantom. A gate that cannot distinguish "clean" from "could not
# check" is not a gate.
#
# So: `grep` is used instead of `rg`, because `grep` is in POSIX and is present
# everywhere this runs. And the preflight below treats an unavailable search
# tool as a hard failure with its own message, so a future tool swap degrades
# loudly rather than silently.
#
# Exit codes:
#   0  every check ran and passed
#   1  a real policy violation
#   2  a check could not run (missing tool, missing site) — NOT the same as clean
set -uo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT" || { echo "coarse-lock check: cannot cd to $ROOT" >&2; exit 2; }

# --- preflight: refuse to report a result we could not actually compute ------
if ! command -v grep >/dev/null 2>&1; then
  echo "coarse-lock check: CANNOT VERIFY — 'grep' is not on PATH." >&2
  echo "  Refusing to report 'clean': a gate that cannot search is not a passing gate." >&2
  exit 2
fi

failures=0

# grep over first-party Rust sources. `--include` replaces `rg --type rust`, so
# no external tool is required to scope the search.
search_rust() {
  # $@ = grep pattern(s); scans crates/ for *.rs, returns grep's exit status.
  grep -rnE --include='*.rs' "$@" crates/
}

# 1. Negative check: coarse lock primitives in code (full-line `//` comments
#    excluded, so doc prose such as "no `RwLock<Mutex<…>>` layering" does not
#    trip the gate).
code_hits="$(search_rust -e 'RwLock' -e 'Mutex' -e 'parking_lot::' || true)"
code_hits="$(printf '%s' "$code_hits" | grep -vE ':[0-9]+:[[:space:]]*//' || true)"
if [ -n "$code_hits" ]; then
  echo "coarse-lock check: forbidden lock primitive in first-party code:" >&2
  printf '%s\n' "$code_hits" >&2
  echo "Use dashmap::DashMap for shared maps; Arc alone (no lock) is fine." >&2
  failures=$((failures + 1))
fi

# 2. Positive check: shared cache maps still go through DashMap.
for site in "crates/cache/src/stats.rs" "crates/cache/src/index.rs"; do
  if [ ! -f "$site" ]; then
    # A renamed or moved file is not a DashMap regression, and must not be
    # reported as one. It is a check that could not run.
    echo "coarse-lock check: CANNOT VERIFY — $site does not exist (renamed or moved?)." >&2
    echo "  Update this gate to point at the file's new location." >&2
    exit 2
  fi
  if ! grep -q 'dashmap::DashMap' "$site"; then
    echo "coarse-lock check: $site no longer uses dashmap::DashMap" >&2
    failures=$((failures + 1))
  fi
done

if [ "$failures" -gt 0 ]; then
  echo "coarse-lock check: $failures failing check(s)" >&2
  exit 1
fi
echo "coarse-lock check: no RwLock/Mutex/parking_lot in crates/ code; DashMap intact"
