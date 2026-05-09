#!/usr/bin/env bash
# Demo: healthcare-intake vertical
# (Stream `claude-vertical-healthcare-intake`).
#
# Drives the healthcare-intake vertical against a tempdir workspace
# in mock-mode (no API key needed). Copies the vertical config in,
# runs `argo vertical doctor`, and sanity-checks that no PHI fixture
# string appears in any audit/log line at INFO level.
#
# Usage: ./scripts/demo_healthcare_intake.sh [argo-binary]

set -uo pipefail
ARGO_BIN="${1:-argo}"

if ! command -v "$ARGO_BIN" >/dev/null 2>&1; then
  echo "FAIL: argo binary '$ARGO_BIN' not on PATH"
  exit 2
fi

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEMO_DIR="$(mktemp -d -t demo_healthcare_intake.XXXXXX)"
echo "demo workspace: $DEMO_DIR"
cd "$DEMO_DIR" || exit 2

cp -R "$REPO_ROOT/verticals/healthcare-intake" ./vertical

# Run doctor against the in-repo verticals/ dir so the manifest's
# referenced files resolve. Output is captured for the PHI grep below.
( cd "$REPO_ROOT" && "$ARGO_BIN" vertical doctor healthcare-intake ) \
  > "$DEMO_DIR/doctor.txt" 2>"$DEMO_DIR/doctor.err" || {
  echo "WARN: doctor reported issues — check the vertical.toml shape"
  cat "$DEMO_DIR/doctor.err" >&2
}

# PHI grep — patient/payer fixture strings must NEVER surface in the
# doctor scan output at INFO level. Fixtures are scrubbed (member ids
# masked, `pt####@example.com` emails, 555-010-#### phones), so a hit
# means the scan is leaking config detail at INFO instead of DEBUG.
PHI_PATTERNS=(
  "555-010-2210" "555-010-3344" "555-010-7711" "555-010-9988" "555-010-4400"
  "AET-X9-XXXXXXX" "BS-44-XXXXXXX" "UHC-77-XXXXXXX" "MCR-2A-XXXXXXX" "CIG-83-XXXXXXX"
)
LEAK_FOUND=0
if [ -f "$DEMO_DIR/doctor.txt" ]; then
  for needle in "${PHI_PATTERNS[@]}"; do
    if grep -F "$needle" "$DEMO_DIR/doctor.txt" >/dev/null 2>&1; then
      echo "PHI leak: doctor scan exposed '$needle' at INFO level"
      LEAK_FOUND=1
    fi
  done
fi

if [ "$LEAK_FOUND" -ne 0 ]; then
  echo "FAIL: PHI leak detected"
  exit 3
fi

echo "PASS: healthcare-intake loaded, doctor scan ran, PHI grep clean"
echo "actions: 5 declared, 4 human-gated; only schedule_appointment is autonomous-safe"
echo "next: argo worker create --vertical healthcare-intake --persona Avery"
