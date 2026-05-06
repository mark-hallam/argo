#!/usr/bin/env bash
# Demo: marketing-intern vertical
# (Stream `claude-demo-multi-vertical`).

set -uo pipefail
ARGO_BIN="${1:-argo}"

if ! command -v "$ARGO_BIN" >/dev/null 2>&1; then
  echo "FAIL: argo binary '$ARGO_BIN' not on PATH"
  exit 2
fi

DEMO_DIR="$(mktemp -d -t demo_marketing_intern.XXXXXX)"
echo "demo workspace: $DEMO_DIR"
cd "$DEMO_DIR" || exit 2

cp -R "$(dirname "$0")/../verticals/marketing-intern" ./vertical
"$ARGO_BIN" vertical doctor --vertical-id marketing-intern || {
  echo "WARN: doctor reported issues — check the vertical.toml shape"
}

echo "PASS: marketing-intern vertical loaded + doctor scan ran"
echo "next steps:"
echo "  1) argo worker create --vertical marketing-intern --persona Sam --charter '...'"
echo "  2) configure competitor domains via argo memory write competitors ..."
echo "  3) argo worker actions list --vertical marketing-intern"
