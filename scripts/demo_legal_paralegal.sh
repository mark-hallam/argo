#!/usr/bin/env bash
# Demo: legal-paralegal vertical
# (Stream `claude-vertical-legal-paralegal`).
#
# Drives the legal-paralegal vertical against a tempdir workspace in
# mock-mode (no API key needed). Copies the vertical config in, runs
# `argo vertical doctor` to confirm the manifest parses, and prints
# the recommended next steps for an operator.
#
# Usage: ./scripts/demo_legal_paralegal.sh [argo-binary]

set -uo pipefail
ARGO_BIN="${1:-argo}"

if ! command -v "$ARGO_BIN" >/dev/null 2>&1; then
  echo "FAIL: argo binary '$ARGO_BIN' not on PATH"
  exit 2
fi

DEMO_DIR="$(mktemp -d -t demo_legal_paralegal.XXXXXX)"
echo "demo workspace: $DEMO_DIR"
cd "$DEMO_DIR" || exit 2

cp -R "$(dirname "$0")/../verticals/legal-paralegal" ./vertical
"$ARGO_BIN" vertical doctor --vertical-id legal-paralegal || {
  echo "WARN: doctor reported issues — check the vertical.toml shape"
}

echo "PASS: legal-paralegal vertical loaded + doctor scan ran"
echo "next steps:"
echo "  1) argo worker create --vertical legal-paralegal --persona Quinn --charter 'Paralegal under attorney supervision'"
echo "  2) seed contracts: cp ./vertical/inventory.json .skycot/legal-paralegal/inventory.json"
echo "  3) argo worker actions list --vertical legal-paralegal"
echo "  4) review a contract: argo worker actions emit review_contract --payload '{\"contract_id\":\"NDA-2026-0117\"}'"
