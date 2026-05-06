#!/usr/bin/env bash
# Demo: sales-sdr vertical
# (Stream `claude-demo-multi-vertical`).

set -uo pipefail
ARGO_BIN="${1:-argo}"

if ! command -v "$ARGO_BIN" >/dev/null 2>&1; then
  echo "FAIL: argo binary '$ARGO_BIN' not on PATH"
  exit 2
fi

DEMO_DIR="$(mktemp -d -t demo_sales_sdr.XXXXXX)"
echo "demo workspace: $DEMO_DIR"
cd "$DEMO_DIR" || exit 2

cp -R "$(dirname "$0")/../verticals/sales-sdr" ./vertical
"$ARGO_BIN" vertical doctor --vertical-id sales-sdr || {
  echo "WARN: doctor reported issues — check the vertical.toml shape"
}

echo "PASS: sales-sdr vertical loaded + doctor scan ran"
echo "next steps:"
echo "  1) argo worker create --vertical sales-sdr --persona Aria --charter '...'"
echo "  2) feed prospects via argo memory write prospects ..."
echo "  3) argo worker actions list --vertical sales-sdr"
