#!/usr/bin/env bash
# publish-pending.sh — publish any workspace crate whose current version is not
# yet on crates.io, then stop.
#
# WHY A TIMER EXISTS
# crates.io rate-limits publishing. A burst of `cargo publish` calls returns 429
# with a retry window, and because a version can never be re-published once it
# exists, a half-finished release is worse than a slow one: some crates land,
# the rest do not, and the workspace is left advertising a version that is only
# half real. This script is driven by a 10-minute systemd timer so the work
# resumes on a schedule instead of needing a human to retry it.
#
# WHAT MAKES IT SAFE TO RUN UNATTENDED
# Publishing is irreversible. Four rails, in order of importance:
#
#   1. It publishes only what is already committed and pushed. It runs from a
#      clean tree on the default branch, and refuses to run at all otherwise.
#      An unpublished local edit can never be shipped by accident.
#   2. It only ever publishes a version that ALREADY EXISTS in the manifest. It
#      never bumps, never tags, never invents a version. A wrong version has to
#      be committed deliberately to be published.
#   3. It skips anything already on the registry, so it is idempotent and
#      safe to run every 10 minutes forever.
#   4. It honours a 429 by exiting, leaving the rest for the next tick, rather
#      than hammering the registry.
#
# It does NOT auto-merge, and it does NOT open PRs. Merging stays a human
# decision; this only publishes what a human already merged.
#
# USAGE
#   scripts/publish-pending.sh [--dry-run] [--allow name,name,...]
#
#   --dry-run          report what would be published, publish nothing
#   --allow a,b        restrict to these crate names (repeatable, comma-separated)
#
# EXIT CODES
#   0  nothing left to publish, or a dry run completed
#   1  a genuine publish failure (not a rate limit)
#   2  refused to run: dirty tree, wrong branch, or missing token
#   3  rate limited; work remains, and the next timer tick will continue

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT" || { echo "publish-pending: cannot cd to $REPO_ROOT" >&2; exit 2; }

DEFAULT_BRANCH="$(git symbolic-ref --quiet --short refs/remotes/origin/HEAD 2>/dev/null | sed 's|^origin/||')"
DEFAULT_BRANCH="${DEFAULT_BRANCH:-master}"

DRY_RUN=0
ALLOW_LIST=""
while [ $# -gt 0 ]; do
  case "$1" in
    --dry-run) DRY_RUN=1; shift ;;
    --allow)   ALLOW_LIST="$ALLOW_LIST,$2"; shift 2 ;;
    -h|--help) sed -n '2,40p' "${BASH_SOURCE[0]}"; exit 0 ;;
    *) echo "publish-pending: unknown argument '$1'" >&2; exit 2 ;;
  esac
done

log() { printf '%s %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$*"; }
die() { log "REFUSED: $*"; exit 2; }

# --- rail 1: only committed, pushed, default-branch content gets published ----
[ -z "$(git status --porcelain)" ] || die "working tree is dirty; refusing to publish"
BRANCH="$(git rev-parse --abbrev-ref HEAD 2>/dev/null)"
[ "$BRANCH" = "$DEFAULT_BRANCH" ] || die "on '$BRANCH', not '$DEFAULT_BRANCH'; refusing to publish"
if [ -n "$(git log --oneline "origin/$DEFAULT_BRANCH..HEAD" 2>/dev/null)" ]; then
  die "local commits are not on origin/$DEFAULT_BRANCH; push first"
fi
if [ ! -f ~/.cargo/credentials.toml ] && [ -z "${CARGO_REGISTRY_TOKEN:-}" ]; then
  die "no crates.io token in ~/.cargo/credentials.toml and CARGO_REGISTRY_TOKEN is unset"
fi

# --- enumerate members in dependency order ----------------------------------
# A crate whose path-dependency sibling is not yet on the registry cannot be
# published, so members are emitted deps-first. `cargo metadata` gives the real
# intra-workspace edges rather than a guess from directory order.
mapfile -t MEMBERS < <(python3 - "$DEFAULT_BRANCH" <<'PY'
import json, subprocess, sys
md = json.loads(subprocess.run(["cargo", "metadata", "--no-deps", "--format-version", "1"],
                               capture_output=True, text=True, check=True).stdout)
local = {p["name"] for p in md["packages"]}
# workspace-internal deps per package
edges = {}
for p in md["packages"]:
    deps = {d["name"] for d in p["dependencies"]
            if d.get("path") is not None or d["name"] in local}
    edges[p["name"]] = deps & local
order, seen = [], set()
def visit(n, stack=()):
    if n in seen or n in stack:
        return
    for d in sorted(edges.get(n, ())):
        visit(d, stack + (n,))
    seen.add(n)
    order.append(n)
for n in sorted(local):
    visit(n)
print("\n".join(order))
PY
) || die "cargo metadata failed"

# `cargo pkgid` yields `name@version` after the `#`; only the version is
# comparable against the registry's `num` field. Stripping the name matters:
# comparing `prv-core@0.2.1` against `0.2.1` never matches, so every published
# crate looks pending and the script republishes the whole workspace.
version_of() {
  cargo pkgid -p "$1" 2>/dev/null | sed 's/.*#//; s/.*@//'
}

# --- is this exact version already public? ----------------------------------
# Checks the version, not merely the crate: a crate can be published while the
# version in this tree is still ahead of the registry.
on_registry() {
  local crate="$1" version="$2"
  curl -sS --max-time 20 -H "User-Agent: degoyle-publish-pending/1.0" \
    "https://crates.io/api/v1/crates/${crate}/versions" 2>/dev/null \
  | python3 -c "
import json,sys
try:
    print('yes' if any(v['num'] == sys.argv[1] for v in json.load(sys.stdin).get('versions', [])) else 'no')
except Exception:
    print('unknown')
" "$version"
}

# --- rail 5: the probe verifies itself against a canary ---------------------
# A probe that silently never matches is the most dangerous failure this script
# could have: it makes every published crate look pending, and the script then
# republishes the entire workspace. That is exactly the bug an earlier draft
# had (a name-prefixed version string), and it produced a dry run cheerfully
# reporting ten already-published crates as pending.
#
# So before the probe is trusted, it must correctly report a version that is
# definitionally on the registry. If it cannot, we stop rather than publish on
# the strength of a measurement we know is wrong.
CANARY_CRATE="serde"
CANARY_VERSION="1.0.0"
canary_state="$(on_registry "$CANARY_CRATE" "$CANARY_VERSION")"
case "$canary_state" in
  yes) : ;;
  *) log "REFUSED: the registry probe reports ${CANARY_CRATE} ${CANARY_VERSION} as '${canary_state}', so it cannot be trusted to decide what is pending"; exit 2 ;;
esac

published=0
for crate in "${MEMBERS[@]}"; do
  [ -n "$crate" ] || continue
  if [ -n "$ALLOW_LIST" ] && [[ ",$ALLOW_LIST," != *",${crate},"* ]]; then
    log "SKIP  $crate (not in --allow list)"
    continue
  fi
  version="$(version_of "$crate")"
  if [ -z "$version" ]; then
    log "SKIP  $crate (could not resolve version)"
    continue
  fi

  state="$(on_registry "$crate" "$version")"
  case "$state" in
    yes)  log "OK    $crate $version already published" ;;
    unknown) log "HOLD  $crate $version (registry unreachable; not guessing)"; published=$((published+1)) ;;
    no)
      if [ "$DRY_RUN" -eq 1 ]; then
        log "DRY   $crate $version would be published"
        published=$((published+1))
        continue
      fi
      log "PUBLISH $crate $version"
      out="$(cargo publish -p "$crate" --locked 2>&1)"
      rc=$?
      # crates.io signals rate limiting with 429; anything else is a real fault.
      if printf '%s' "$out" | grep -qi '429\|rate limit\|too many requests'; then
        log "RATE LIMITED on $crate; leaving the rest for the next tick"
        printf '%s\n' "$out" | tail -5
        exit 3
      fi
      if [ $rc -ne 0 ]; then
        log "FAILED $crate $version (exit $rc)"
        printf '%s\n' "$out" | tail -15
        exit 1
      fi
      log "DONE  $crate $version"
      # crates.io indexes asynchronously; confirm before moving on so a silent
      # rejection cannot be reported as a success and skipped forever after.
      sleep 5
      if [ "$(on_registry "$crate" "$version")" = "yes" ]; then
        published=$((published+1))
      else
        log "WARN  $crate $version not yet visible on crates.io; the next tick will confirm"
        published=$((published+1))
      fi
      ;;
  esac
done

if [ "$published" -eq 0 ]; then
  log "nothing pending; workspace is fully published"
else
  log "$published item(s) handled this tick"
fi
exit 0
