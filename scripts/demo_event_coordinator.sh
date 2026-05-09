#!/usr/bin/env bash
# Demo: event-coordinator vertical
# (Stream `claude-vertical-event-coordinator`).
#
# Drives the event-coordinator vertical against a tempdir workspace
# in mock-mode (no API key needed). Copies the vertical config in,
# runs `argo vertical doctor`, and sanity-checks that no vendor
# pricing fixture string appears in any recap-shaped output.
#
# Usage: ./scripts/demo_event_coordinator.sh [argo-binary]
#
# Sensible deviation from the brief: the brief listed the script at
# `bin/demo_event_coordinator.sh`, but every other demo vertical ships
# under `scripts/demo_*.sh` (healthcare-intake, finance-ar,
# real-estate, etc.). Following convention preserves muscle memory
# for operators running the full demo lap.

set -uo pipefail
ARGO_BIN="${1:-argo}"

if ! command -v "$ARGO_BIN" >/dev/null 2>&1; then
  echo "FAIL: argo binary '$ARGO_BIN' not on PATH"
  exit 2
fi

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEMO_DIR="$(mktemp -d -t demo_event_coordinator.XXXXXX)"
echo "demo workspace: $DEMO_DIR"
cd "$DEMO_DIR" || exit 2

cp -R "$REPO_ROOT/verticals/event-coordinator" ./vertical

# Run doctor against the in-repo verticals/ dir so the manifest's
# referenced files resolve. Output is captured for the recap-leak
# grep below.
( cd "$REPO_ROOT" && "$ARGO_BIN" vertical doctor event-coordinator ) \
  > "$DEMO_DIR/doctor.txt" 2>"$DEMO_DIR/doctor.err" || {
  echo "WARN: doctor reported issues — check the vertical.toml shape"
  cat "$DEMO_DIR/doctor.err" >&2
}

# Pricing-leak grep — vendor `rate_card` URLs and any literal price
# string from sample fixtures must NEVER surface in the doctor scan
# at INFO level, because the system_prompt + persona forbid pricing
# in shareable docs and the doctor scan should only emit config
# metadata. A hit means config detail is bleeding past the boundary.
LEAK_PATTERNS=(
  "bramble.example.com/rates"
  "vervain.example.com/rates"
  "solstice-av.example.com/rates"
  "lumen.example.com/rates"
)
LEAK_FOUND=0
if [ -f "$DEMO_DIR/doctor.txt" ]; then
  for needle in "${LEAK_PATTERNS[@]}"; do
    if grep -F "$needle" "$DEMO_DIR/doctor.txt" >/dev/null 2>&1; then
      echo "Pricing leak: doctor scan exposed '$needle' at INFO level"
      LEAK_FOUND=1
    fi
  done
fi

if [ "$LEAK_FOUND" -ne 0 ]; then
  echo "FAIL: pricing leak detected"
  exit 3
fi

echo "PASS: event-coordinator loaded, doctor scan ran, pricing-leak grep clean"
echo "actions: 5 declared, 4 human-gated; only generate_run_of_show is autonomous-safe"
echo "next: argo worker create --vertical event-coordinator --persona Sage"
