#!/usr/bin/env bash
# Demo: finance-ar vertical
# (Stream `claude-vertical-finance-ar`).
#
# Drives the finance-ar vertical against a tempdir workspace in
# mock-mode (no API key needed). Copies the vertical config in,
# runs `argo vertical doctor` to confirm the manifest parses,
# and prints the recommended next steps for an operator.
#
# Usage: ./scripts/demo_finance_ar.sh [argo-binary]

set -uo pipefail
ARGO_BIN="${1:-argo}"

if ! command -v "$ARGO_BIN" >/dev/null 2>&1; then
  echo "FAIL: argo binary '$ARGO_BIN' not on PATH"
  exit 2
fi

DEMO_DIR="$(mktemp -d -t demo_finance_ar.XXXXXX)"
echo "demo workspace: $DEMO_DIR"
cd "$DEMO_DIR" || exit 2

cp -R "$(dirname "$0")/../verticals/finance-ar" ./vertical
"$ARGO_BIN" vertical doctor --vertical-id finance-ar || {
  echo "WARN: doctor reported issues — check the vertical.toml shape"
}

echo "PASS: finance-ar vertical loaded + doctor scan ran"
echo "next steps:"
echo "  1) argo worker create --vertical finance-ar --persona Riley --charter 'AR clerk for Acme Widgets'"
echo "  2) seed invoices: cp ./vertical/inventory.json .skycot/finance-ar/inventory.json"
echo "  3) argo worker actions list --vertical finance-ar"
echo "  4) the daily_review duty fires at 09:00 weekdays once duty-scheduler-gateway-wiring lands"
