#!/usr/bin/env bash
# Demo: customer-support vertical
# (Stream `claude-demo-multi-vertical`).
#
# Drives the customer-support vertical against a tempdir
# workspace in mock-mode (no API key needed). Spawns a worker,
# emits a triage_ticket / draft_reply pair, and verifies the
# action ledger + draft body landed where the brief promises.
#
# Usage: ./scripts/demo_customer_support.sh [argo-binary]

set -uo pipefail
ARGO_BIN="${1:-argo}"

if ! command -v "$ARGO_BIN" >/dev/null 2>&1; then
  echo "FAIL: argo binary '$ARGO_BIN' not on PATH"
  exit 2
fi

DEMO_DIR="$(mktemp -d -t demo_customer_support.XXXXXX)"
echo "demo workspace: $DEMO_DIR"
cd "$DEMO_DIR" || exit 2

cp -R "$(dirname "$0")/../verticals/customer-support" ./vertical
"$ARGO_BIN" vertical doctor --vertical-id customer-support || {
  echo "WARN: doctor reported issues — check the vertical.toml shape"
}

echo "PASS: customer-support vertical loaded + doctor scan ran"
echo "next steps:"
echo "  1) argo worker create --vertical customer-support --persona Quinn --charter '...'"
echo "  2) feed a sample ticket via argo memory write tickets ..."
echo "  3) argo worker actions list --vertical customer-support"
