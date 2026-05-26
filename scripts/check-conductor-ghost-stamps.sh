#!/usr/bin/env bash
# Detect conductor SHARED.md stamps without a nearby committed heartbeat refresh.

set -euo pipefail

GIT_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || true)"
if [[ -n "$GIT_ROOT" ]]; then
  REPO="$GIT_ROOT"
else
  REPO="${ARGO_REPO_ROOT:-$(pwd)}"
fi

SHARED="${CONDUCTOR_GHOST_SHARED:-$REPO/docs/skycot/agents/SHARED.md}"
HEARTBEAT="${CONDUCTOR_GHOST_HEARTBEAT:-$REPO/docs/skycot/agents/heartbeats/conductor.json}"
LIMIT="${CONDUCTOR_GHOST_LIMIT:-10}"
WINDOW_SECONDS="${CONDUCTOR_GHOST_WINDOW_SECONDS:-300}"

if [[ ! "$LIMIT" =~ ^[0-9]+$ ]] || [[ "$LIMIT" -lt 1 ]]; then
  echo "check-conductor-ghost-stamps: CONDUCTOR_GHOST_LIMIT must be a positive integer" >&2
  exit 2
fi
if [[ ! "$WINDOW_SECONDS" =~ ^[0-9]+$ ]] || [[ "$WINDOW_SECONDS" -lt 1 ]]; then
  echo "check-conductor-ghost-stamps: CONDUCTOR_GHOST_WINDOW_SECONDS must be a positive integer" >&2
  exit 2
fi
if [[ ! -f "$SHARED" ]]; then
  echo "check-conductor-ghost-stamps: no SHARED.md at $SHARED" >&2
  exit 2
fi
if [[ ! -f "$HEARTBEAT" ]]; then
  echo "check-conductor-ghost-stamps: no conductor heartbeat at $HEARTBEAT" >&2
  exit 2
fi

cd "$REPO"

python3 - "$REPO" "$SHARED" "$HEARTBEAT" "$LIMIT" "$WINDOW_SECONDS" <<'PY'
from datetime import datetime
import os
import re
import subprocess
import sys

repo, shared, heartbeat, limit_raw, window_raw = sys.argv[1:]
limit = int(limit_raw)
window = int(window_raw)
stamp_re = re.compile(
    r"^([0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2})"
    r"Z?(?:\+[0-9]{4})? \[conductor[-a-z]*\]"
)

def epoch(iso: str) -> int:
    return int(datetime.fromisoformat(iso.replace("Z", "+00:00")).timestamp())

heartbeat_rel = os.path.relpath(heartbeat, repo)
history = subprocess.run(
    ["git", "log", "--date=iso-strict", "--format=%cI", "--", heartbeat_rel],
    check=False,
    stdout=subprocess.PIPE,
    text=True,
)
commits = [(epoch(line.strip()), line.strip()) for line in history.stdout.splitlines() if line.strip()]
if not commits:
    print(f"check-conductor-ghost-stamps: no git history for {heartbeat_rel}", file=sys.stderr)
    sys.exit(2)

with open(shared, encoding="utf-8") as handle:
    entries = [line.rstrip("\n") for line in handle if stamp_re.match(line)]

recent = sorted(entries)[-limit:]
if not recent:
    print("✅ No recent conductor SHARED.md entries found.")
    sys.exit(0)

ghost_count = 0
for line in recent:
    match = stamp_re.match(line)
    if not match:
        continue
    stamp = f"{match.group(1)}Z"
    stamp_epoch = epoch(stamp)
    best_epoch, best_iso = min(commits, key=lambda item: abs(stamp_epoch - item[0]))
    delta = abs(stamp_epoch - best_epoch)
    if delta > window:
        ghost_count += 1
        print(f"GHOST STAMP: {line}")
        print(
            "  nearest conductor.json commit: "
            f"{best_iso} ({delta}s away; window={window}s)"
        )

if ghost_count:
    print(f"⚠️ ghost-stamped conductor entries detected ({ghost_count})")
    sys.exit(1)

print(f"✅ No ghost-stamped conductor entries in last {limit} conductor stamps (window ±{window}s).")
PY
