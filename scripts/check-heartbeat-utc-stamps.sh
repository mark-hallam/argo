#!/usr/bin/env bash
# check-heartbeat-utc-stamps.sh -- lint heartbeat + SHARED.md UTC stamps.
#
# This is a write-side drift guard for the agent heartbeat surfaces. It
# accepts only fixed-width UTC stamps and rejects future-dated "fake Z"
# values, which are usually local-time strings written with a Z suffix.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
NOW=""

usage() {
    cat <<'USAGE' >&2
usage: scripts/check-heartbeat-utc-stamps.sh [--root <repo>] [--now <YYYY-MM-DDTHH:MM:SSZ>]

Checks:
  - docs/skycot/agents/heartbeats/*.json last_run_utc
  - docs/skycot/agents/SHARED.md appended entry timestamps
USAGE
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --root)
            [[ $# -ge 2 ]] || { usage; exit 2; }
            ROOT="$2"
            shift 2
            ;;
        --now)
            [[ $# -ge 2 ]] || { usage; exit 2; }
            NOW="$2"
            shift 2
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            usage
            exit 2
            ;;
    esac
done

python3 - "$ROOT" "$NOW" <<'PY'
import json
import re
import sys
from datetime import datetime, timezone, timedelta
from pathlib import Path

root = Path(sys.argv[1]).resolve()
now_arg = sys.argv[2]
strict_utc = re.compile(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$")
future_grace = timedelta(seconds=300)
failures = []


def rel(path: Path) -> str:
    try:
        return str(path.resolve().relative_to(root))
    except ValueError:
        return str(path)


def parse_strict_utc(value: str):
    if not isinstance(value, str) or not strict_utc.match(value):
        return None
    try:
        return datetime.strptime(value, "%Y-%m-%dT%H:%M:%SZ").replace(tzinfo=timezone.utc)
    except ValueError:
        return None


def key_line(path: Path, key: str) -> int:
    pattern = re.compile(rf'"{re.escape(key)}"\s*:')
    try:
        for idx, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            if pattern.search(line):
                return idx
    except OSError:
        pass
    return 1


def add(path: Path, line: int, message: str):
    failures.append(f"{rel(path)}:{line}: {message}")


if now_arg:
    now = parse_strict_utc(now_arg)
    if now is None:
        print("--now: invalid timestamp; expected YYYY-MM-DDTHH:MM:SSZ", file=sys.stderr)
        sys.exit(2)
else:
    now = datetime.now(timezone.utc).replace(microsecond=0)


def validate_stamp(path: Path, line: int, label: str, value):
    if not isinstance(value, str):
        add(path, line, f"{label} must be a string in strict UTC YYYY-MM-DDTHH:MM:SSZ format")
        return
    if not strict_utc.match(value):
        add(path, line, f"{label} must use strict UTC YYYY-MM-DDTHH:MM:SSZ format (got {value!r})")
        return
    parsed = parse_strict_utc(value)
    if parsed is None:
        add(path, line, f"{label} is not a valid UTC timestamp (got {value!r})")
        return
    if parsed - now > future_grace:
        delta = int((parsed - now).total_seconds())
        add(
            path,
            line,
            f"{label} is future-dated by {delta}s; possible local time stamped with Z, use date -u",
        )


heartbeat_dir = root / "docs/skycot/agents/heartbeats"
if heartbeat_dir.exists():
    for path in sorted(heartbeat_dir.glob("*.json")):
        try:
            data = json.loads(path.read_text(encoding="utf-8"))
        except json.JSONDecodeError as exc:
            add(path, exc.lineno, f"invalid json: {exc.msg}")
            continue
        except OSError as exc:
            add(path, 1, f"could not read heartbeat json: {exc}")
            continue

        if "last_run_utc" not in data:
            add(path, 1, "missing last_run_utc")
            continue
        validate_stamp(path, key_line(path, "last_run_utc"), "last_run_utc", data["last_run_utc"])

shared = root / "docs/skycot/agents/SHARED.md"
if shared.exists():
    try:
        lines = shared.read_text(encoding="utf-8").splitlines()
    except OSError as exc:
        add(shared, 1, f"could not read SHARED.md: {exc}")
    else:
        in_entries = False
        for idx, line in enumerate(lines, 1):
            stripped = line.strip()
            if not in_entries:
                if stripped == "---":
                    in_entries = True
                continue
            if not stripped:
                continue
            match = re.match(r"^(\S+)\s+\[[^\]]+\]\s+.+", stripped)
            if not match:
                add(shared, idx, "SHARED.md entry must start with '<UTC-ISO> [agent] ...'")
                continue
            validate_stamp(shared, idx, "SHARED.md entry timestamp", match.group(1))

if failures:
    for failure in failures:
        print(failure)
    sys.exit(1)

print("ok heartbeat UTC stamps")
PY
