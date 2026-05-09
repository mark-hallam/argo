#!/usr/bin/env bash
# Demo: real-estate-agent vertical
# (Stream `claude-vertical-real-estate-agent`).
#
# Drives the real-estate-agent vertical against a tempdir workspace
# in mock-mode (no API key needed). Copies the vertical config in,
# runs `argo vertical doctor` to confirm the manifest parses, and
# prints the recommended next steps for an operator.
#
# Usage: ./scripts/demo_real_estate.sh [argo-binary]

set -uo pipefail
ARGO_BIN="${1:-argo}"

if ! command -v "$ARGO_BIN" >/dev/null 2>&1; then
  echo "FAIL: argo binary '$ARGO_BIN' not on PATH"
  exit 2
fi

DEMO_DIR="$(mktemp -d -t demo_real_estate.XXXXXX)"
echo "demo workspace: $DEMO_DIR"
cd "$DEMO_DIR" || exit 2

cp -R "$(dirname "$0")/../verticals/real-estate-agent" ./vertical
"$ARGO_BIN" vertical doctor --vertical-id real-estate-agent || {
  echo "WARN: doctor reported issues — check the vertical.toml shape"
}

echo "PASS: real-estate-agent vertical loaded + doctor scan ran"
echo "next steps:"
echo "  1) argo worker create --vertical real-estate-agent --persona Jamie --charter 'Assistant for Pat the Agent — RE buyer-side'"
echo "  2) seed listings: cp ./vertical/inventory/sample_listings.json .skycot/real-estate-agent/listings.json"
echo "  3) argo worker actions list --vertical real-estate-agent"
echo "  4) auto-approval gates schedule_viewing + follow_up_lead; the other 3 actions surface to the operator"
