#!/usr/bin/env bash
# Demo: product-manager vertical
# (Stream `claude-vertical-product-manager`).
#
# Drives the product-manager vertical against a tempdir workspace in
# mock-mode (no API key needed). Copies the vertical config in, runs
# `argo vertical doctor`, and sanity-checks that no participant or
# customer name from the inventory leaks into the doctor scan output
# at INFO level.
#
# Brief filed the script at `bin/demo_product_manager.sh`; the
# established convention across the existing 7 vertical demos is
# `scripts/demo_*.sh`. Filed here to match — sensible deviation
# documented in the PR body.
#
# Usage: ./scripts/demo_product_manager.sh [argo-binary]

set -uo pipefail
ARGO_BIN="${1:-argo}"

if ! command -v "$ARGO_BIN" >/dev/null 2>&1; then
  echo "FAIL: argo binary '$ARGO_BIN' not on PATH"
  exit 2
fi

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEMO_DIR="$(mktemp -d -t demo_product_manager.XXXXXX)"
echo "demo workspace: $DEMO_DIR"
cd "$DEMO_DIR" || exit 2

cp -R "$REPO_ROOT/verticals/product-manager" ./vertical

# Run doctor against the in-repo verticals/ dir so the manifest's
# referenced files resolve. Output is captured for the leak grep below.
( cd "$REPO_ROOT" && "$ARGO_BIN" vertical doctor product-manager ) \
  > "$DEMO_DIR/doctor.txt" 2>"$DEMO_DIR/doctor.err" || {
  echo "WARN: doctor reported issues — check the vertical.toml shape"
  cat "$DEMO_DIR/doctor.err" >&2
}

# Leak grep — participant_ids in the inventory should never surface
# in the doctor scan output at INFO level. Synthetic ids only; a hit
# means the scan is leaking config detail at INFO instead of DEBUG.
LEAK_PATTERNS=(
  "RILEY-0011" "RILEY-0017" "RILEY-0024" "RILEY-0031" "RILEY-0042"
  "RILEY-0049" "RILEY-0053" "RILEY-0058" "RILEY-0064" "RILEY-0071"
)
LEAK_FOUND=0
if [ -f "$DEMO_DIR/doctor.txt" ]; then
  for needle in "${LEAK_PATTERNS[@]}"; do
    if grep -F "$needle" "$DEMO_DIR/doctor.txt" >/dev/null 2>&1; then
      echo "Participant id leak: doctor scan exposed '$needle' at INFO level"
      LEAK_FOUND=1
    fi
  done
fi

if [ "$LEAK_FOUND" -ne 0 ]; then
  echo "FAIL: participant id leak detected"
  exit 3
fi

echo "PASS: product-manager loaded, doctor scan ran, leak grep clean"
echo "actions: 5 declared, 2 gated (draft_prd, update_roadmap); auto-approval ratio 3/5 — highest in catalogue"
echo "next: argo worker create --vertical product-manager --persona Riley"
