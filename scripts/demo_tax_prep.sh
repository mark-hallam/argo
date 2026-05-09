#!/usr/bin/env bash
# Demo: tax-prep vertical
# (Stream `claude-vertical-tax-prep`).
#
# Drives the tax-prep vertical against a tempdir workspace in
# mock-mode (no API key needed). Copies the vertical config in,
# runs `argo vertical doctor` to confirm the manifest parses, and
# prints the recommended next steps for an operator.
#
# Usage: ./scripts/demo_tax_prep.sh [argo-binary]

set -uo pipefail
ARGO_BIN="${1:-argo}"

if ! command -v "$ARGO_BIN" >/dev/null 2>&1; then
  echo "FAIL: argo binary '$ARGO_BIN' not on PATH"
  exit 2
fi

DEMO_DIR="$(mktemp -d -t demo_tax_prep.XXXXXX)"
echo "demo workspace: $DEMO_DIR"
cd "$DEMO_DIR" || exit 2

cp -R "$(dirname "$0")/../verticals/tax-prep" ./vertical
"$ARGO_BIN" vertical doctor --vertical-id tax-prep || {
  echo "WARN: doctor reported issues — check the vertical.toml shape"
}

echo "PASS: tax-prep vertical loaded + doctor scan ran"
echo "next steps:"
echo "  1) argo worker create --vertical tax-prep --persona Dakota --charter 'Seasonal assistant for Pat the Preparer, EA — 1040 prep + e-file routing'"
echo "  2) seed W-2 inventory: cp ./vertical/inventory/sample_w2s.json .skycot/tax-prep/w2s.json"
echo "  3) argo worker actions list --vertical tax-prep"
echo "  4) auto-approval gates review_w2 + schedule_review; flag_deduction_eligibility, draft_return, and file_return all surface to the operator"
echo "  5) file_return is mocked — V2 does NOT call the IRS e-file API"
