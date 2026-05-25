#!/usr/bin/env bash
# check-conductor-cutoff-staleness.sh -- warn when the conductor read
# cutoff has not advanced recently.

set -euo pipefail

REPO="${ARGO_REPO_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
HEARTBEAT="$REPO/docs/skycot/agents/heartbeats/conductor.json"
NOW_UTC="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
THRESHOLD_SECONDS=7200

usage() {
  cat <<'EOF'
Usage: scripts/check-conductor-cutoff-staleness.sh [--heartbeat PATH] [--now UTC-ISO] [--threshold-seconds N]

Exit codes:
  0  conductor cutoff is fresh enough
  1  conductor cutoff is older than the threshold
  2  heartbeat is missing, malformed, or timestamp parsing failed
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --heartbeat)
      HEARTBEAT="${2:-}"
      shift 2
      ;;
    --now)
      NOW_UTC="${2:-}"
      shift 2
      ;;
    --threshold-seconds)
      THRESHOLD_SECONDS="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      usage >&2
      exit 2
      ;;
  esac
done

python3 - "$HEARTBEAT" "$NOW_UTC" "$THRESHOLD_SECONDS" <<'PY'
from __future__ import annotations

from datetime import datetime, timezone
import json
import sys

heartbeat_path, now_raw, threshold_raw = sys.argv[1:4]


def parse_iso(value: str) -> datetime:
    value = value.strip()
    if not value:
        raise ValueError("empty timestamp")
    parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    if parsed.tzinfo is None:
        parsed = parsed.replace(tzinfo=timezone.utc)
    return parsed.astimezone(timezone.utc)


def age_label(seconds: int) -> str:
    minutes = max(0, seconds) // 60
    hours, mins = divmod(minutes, 60)
    if hours:
        return f"{hours}h{mins:02d}m"
    return f"{mins}m"


try:
    threshold = int(threshold_raw)
    if threshold < 1:
        raise ValueError("threshold must be positive")
except Exception as exc:
    print(f"FAIL: invalid threshold: {exc}", file=sys.stderr)
    sys.exit(2)

try:
    with open(heartbeat_path, "r", encoding="utf-8") as f:
        last_raw = str(json.load(f).get("last_run_utc", ""))
except Exception as exc:
    print(f"FAIL: read conductor heartbeat: {exc}", file=sys.stderr)
    sys.exit(2)

try:
    last = parse_iso(last_raw)
    now = parse_iso(now_raw)
except Exception as exc:
    print(f"FAIL: parse conductor cutoff timestamp: {exc}", file=sys.stderr)
    sys.exit(2)

age_seconds = max(0, int((now - last).total_seconds()))
if age_seconds > threshold:
    print(
        "⚠️ conductor cutoff staleness: "
        f"last refresh {age_label(age_seconds)} ago — "
        "review PROTOCOL_LOOP_PROMPT.md Step 4.6 compliance"
    )
    sys.exit(1)

print(
    "OK: conductor cutoff fresh: "
    f"last refresh {age_label(age_seconds)} ago "
    f"(last_run_utc={last_raw})"
)
PY
