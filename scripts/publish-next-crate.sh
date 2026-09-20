#!/bin/bash
set -euo pipefail
cd /home/leo/prv

STATE_FILE="/home/leo/prv/.publish-state"
CRATES=("prv-geometry" "prv-policy" "prv-evaluation" "prv-cli")

if [ ! -f "$STATE_FILE" ]; then
  echo 0 > "$STATE_FILE"
fi

INDEX=$(cat "$STATE_FILE")
if [ "$INDEX" -ge "${#CRATES[@]}" ]; then
  echo "All crates published."
  systemctl disable --now prv-publish.timer || true
  exit 0
fi

CRATE="${CRATES[$INDEX]}"
echo "Publishing $CRATE..."
cargo publish -p "$CRATE"

INDEX=$((INDEX + 1))
echo "$INDEX" > "$STATE_FILE"

if [ "$INDEX" -ge "${#CRATES[@]}" ]; then
  echo "All crates published. Disabling timer."
  systemctl disable --now prv-publish.timer || true
fi
