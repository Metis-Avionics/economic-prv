#!/bin/bash
set -euo pipefail
cd /home/leo/prv

STATE_FILE="/home/leo/prv/.publish-state"
CRATES=("prv-core" "prv-cache" "prv-filter" "prv-geometry" "prv-monte-carlo" "prv-policy" "prv-data" "prv-evaluation" "prv-cli")

if [ ! -f "$STATE_FILE" ]; then
  echo 0 > "$STATE_FILE"
fi

INDEX=$(cat "$STATE_FILE")
if [ "$INDEX" -ge "${#CRATES[@]}" ]; then
  echo "All crates published."
else
  CRATE="${CRATES[$INDEX]}"
  echo "Publishing $CRATE..."
  cargo publish -p "$CRATE"

  INDEX=$((INDEX + 1))
  echo "$INDEX" > "$STATE_FILE"

  if [ "$INDEX" -ge "${#CRATES[@]}" ]; then
    echo "All crates published."
  else
    echo "Next crate will be ${CRATES[$INDEX]} on next timer tick."
    exit 0
  fi
fi

echo "Creating GitHub release binary..."
VERSION=$(grep '^version' spec.toml | head -1 | sed 's/.*= *"//;s/".*//')
if [ -z "$VERSION" ]; then
  VERSION="0.1.0"
fi

TAG="v$VERSION"
if git rev-parse "$TAG" >/dev/null 2>&1; then
  echo "Tag $TAG already exists, skipping git tag."
else
  git tag "$TAG"
  git push origin "$TAG"
fi

cargo bloat --release -p prv-cli --bin prv-cli -o /tmp/prv-cli-size.txt 2>&1 || true
cargo build --release -p prv-cli --bin prv-cli

BINARY_PATH="/home/leo/prv/target/release/prv-cli"
if [ ! -f "$BINARY_PATH" ]; then
  echo "Binary not found at $BINARY_PATH"
  exit 1
fi

if gh release view "$TAG" >/dev/null 2>&1; then
  echo "Release $TAG already exists, uploading asset..."
  gh release upload "$TAG" "$BINARY_PATH" --clobber
else
  NOTES="PRV $VERSION release binary built with cargo bloat --release"
  gh release create "$TAG" "$BINARY_PATH" \
    --title "prv $VERSION" \
    --notes "$NOTES"
fi

echo "GitHub release $TAG created successfully."
systemctl disable --now prv-publish.timer || true
