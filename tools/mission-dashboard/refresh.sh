#!/usr/bin/env bash
# Refresh the mission dashboard. Reads the source markdown files +
# git log; never modifies them.
#
# Usage:
#   ./mission-dashboard/refresh.sh                      # regenerate once
#   ./mission-dashboard/refresh.sh --open               # regenerate once + open in browser
#   ./mission-dashboard/refresh.sh --serve              # one-click refresh server
#   ./mission-dashboard/refresh.sh --serve --open       # serve + open browser
#   ./mission-dashboard/refresh.sh --partial 0.75       # custom partial weight
#
# Works from ANY current directory — uses absolute path to the script.
set -euo pipefail

# Resolve the directory of THIS script — works no matter where you run it from.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PY="${SCRIPT_DIR}/refresh.py"

OPEN=0
SERVE=0
ARGS=()
while [[ $# -gt 0 ]]; do
  case "$1" in
    --open)            OPEN=1; shift ;;
    --serve)           SERVE=1; shift ;;
    --partial)         ARGS+=(--partial-weight "$2"); shift 2 ;;
    --partial=*)       ARGS+=(--partial-weight "${1#*=}"); shift ;;
    --port)            ARGS+=(--port "$2"); shift 2 ;;
    *)                 ARGS+=("$1"); shift ;;
  esac
done

if [[ "$SERVE" == "1" ]]; then
  EXTRA=(--serve)
  [[ "$OPEN" == "1" ]] && EXTRA+=(--open)
  exec python3 "$PY" "${EXTRA[@]}" "${ARGS[@]+"${ARGS[@]}"}"
fi

python3 "$PY" "${ARGS[@]+"${ARGS[@]}"}"
if [[ "$OPEN" == "1" ]]; then
  open "${SCRIPT_DIR}/dashboard.html"
fi
