#!/usr/bin/env bash
# check-conductor-singleton.sh -- preflight guard for conductor mutations.
#
# Prints one machine-readable reason to stdout:
#   missing  heartbeat file absent; safe to proceed
#   stale    heartbeat exists but is older than cadence + 25% grace
#   alive    heartbeat is still within cadence + 25% grace
#   newer    heartbeat timestamp is newer than this candidate tick
#   invalid  JSON, timestamp, or cadence cannot be parsed
#
# Exit code:
#   0  missing or stale
#   1  alive, newer, or invalid

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
HEARTBEAT="$ROOT/docs/skycot/agents/heartbeats/conductor.json"
CANDIDATE_NOW=""

usage() {
  cat <<'EOF'
Usage: scripts/check-conductor-singleton.sh [--heartbeat PATH] [--candidate-now ISO]

Defaults to docs/skycot/agents/heartbeats/conductor.json and current UTC.
EOF
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --heartbeat)
      if [ -z "${2:-}" ]; then
        echo "invalid"
        exit 1
      fi
      HEARTBEAT="$2"
      shift 2
      ;;
    --candidate-now)
      if [ -z "${2:-}" ]; then
        echo "invalid"
        exit 1
      fi
      CANDIDATE_NOW="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "invalid"
      exit 1
      ;;
  esac
done

python3 - "$HEARTBEAT" "$CANDIDATE_NOW" <<'PY'
import datetime as dt
import json
import os
import sys

heartbeat_path, candidate_arg = sys.argv[1:3]


def finish(reason: str, code: int) -> None:
    print(reason)
    raise SystemExit(code)


def parse_utc_iso(value: object) -> dt.datetime:
    if not isinstance(value, str) or not value.strip():
        raise ValueError("empty timestamp")
    raw = value.strip()
    if raw.endswith("Z"):
        raw = raw[:-1] + "+00:00"
    parsed = dt.datetime.fromisoformat(raw)
    if parsed.tzinfo is None:
        raise ValueError("timestamp must include timezone")
    return parsed.astimezone(dt.timezone.utc)


if not os.path.exists(heartbeat_path):
    finish("missing", 0)

try:
    with open(heartbeat_path, encoding="utf-8") as handle:
        heartbeat = json.load(handle)
except Exception:
    finish("invalid", 1)

try:
    last_run = parse_utc_iso(heartbeat.get("last_run_utc"))
    cadence_s = int(heartbeat.get("expected_cadence_seconds"))
    if cadence_s <= 0:
        raise ValueError("cadence must be positive")
    if candidate_arg:
        candidate_now = parse_utc_iso(candidate_arg)
    else:
        candidate_now = dt.datetime.now(dt.timezone.utc)
except Exception:
    finish("invalid", 1)

if last_run > candidate_now:
    finish("newer", 1)

age_s = (candidate_now - last_run).total_seconds()
alive_window_s = cadence_s * 1.25
if age_s <= alive_window_s:
    finish("alive", 1)

finish("stale", 0)
PY
