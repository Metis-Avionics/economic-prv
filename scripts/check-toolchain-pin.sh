#!/usr/bin/env bash
# Asserts rust-toolchain.toml and the CI workflow install the same toolchain.
#
# A pin that only one of the two knows about is not a pin. If the file says
# 1.98.1 and CI installs `stable`, then CI is not testing the pinned compiler
# and every "passes on CI" claim is about a toolchain nobody chose. That is the
# exact failure the pin exists to prevent, and it is invisible in review
# because both files look individually correct.
#
# This is the economic-prv counterpart of scripts/check-advisory-rationales.sh
# in theMQL: a fact that must stay true across two files gets a checker rather
# than a reviewer's memory.
#
# Exit 0 = the two agree. Exit 1 = drift, with both values printed.

set -uo pipefail

cd "$(dirname "$0")/.." || exit 1

status=0
report() { printf '%-8s %s\n' "$1" "$2"; }

file_ver=$(awk -F'"' '/^channel[[:space:]]*=/ { print $2; exit }' rust-toolchain.toml)
# Keep the raw value, not a number: "stable" must survive long enough to be
# reported as floating, rather than being stripped to "" and surfacing as a
# confusing "no toolchain found".
ci_raw=$(awk '
  /dtolnay\/rust-toolchain/ { in_action = 1; next }
  in_action && /^[[:space:]]*toolchain:/ {
    sub(/^[[:space:]]*toolchain:[[:space:]]*/, ""); gsub(/[[:space:]]/, ""); print; exit
  }
' .github/workflows/ci.yml)

echo "== toolchain pin consistency =="

if [ -z "${file_ver:-}" ]; then
  report "FAIL" "no channel found in rust-toolchain.toml"
  exit 1
fi
if [ -z "${ci_raw:-}" ]; then
  report "FAIL" "no 'toolchain:' key next to the dtolnay/rust-toolchain action in .github/workflows/ci.yml"
  exit 1
fi

report "INFO" "rust-toolchain.toml channel = $file_ver"
report "INFO" "ci.yml toolchain            = $ci_raw"

# Classify a value as floating or pinned before comparing, so a floating CI
# channel is reported as the specific problem it is.
is_floating() {
  case "$1" in
    stable | beta | nightly | master | "") return 0 ;;
    *) return 1 ;;
  esac
}

if is_floating "$ci_raw"; then
  report "DRIFT" "CI installs the floating channel '$ci_raw' while rust-toolchain.toml pins
       '$file_ver'. CI is therefore not testing the pinned compiler, so its
       result says nothing about the pin. Pin it to $file_ver."
  status=1
elif [ "$file_ver" != "$ci_raw" ]; then
  report "DRIFT" "rust-toolchain.toml pins '$file_ver' but CI installs '$ci_raw'.
       Make them match, then re-run this check."
  status=1
else
  report "OK" "the pin and CI agree on $file_ver"
fi

# A floating channel anywhere is the thing being prevented, so name it
# explicitly rather than relying on the comparison above to catch a rename.
# Comments are stripped first: these files *discuss* `channel = "stable"` in
# prose, and a grep that reads the prose is a grep that cries wolf.
for f in rust-toolchain.toml .github/workflows/ci.yml; do
  body=$(sed -e 's/#.*$//' -e '/^[[:space:]]*$/d' "$f")
  if printf '%s' "$body" | grep -Eq '^[[:space:]]*channel[[:space:]]*=[[:space:]]*"(stable|beta|nightly|master)"'; then
    report "DRIFT" "$f sets a floating channel. Pin an exact version."
    status=1
  fi
  if printf '%s' "$body" | grep -Eq '^[[:space:]]*toolchain:[[:space:]]*(stable|beta|nightly|master)[[:space:]]*$'; then
    report "DRIFT" "$f installs a floating channel. Pin an exact version."
    status=1
  fi
done

if [ "$status" -eq 0 ]; then
  echo "RESULT: toolchain is pinned and CI matches"
else
  echo "RESULT: toolchain pin has drifted"
fi

exit "$status"
