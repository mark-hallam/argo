#!/usr/bin/env bash
# Classify unread non-conductor SHARED.md handoffs into conductor actions.
#
# This is a read-only companion to scripts/check-unread-handoffs.sh. It
# uses the same cutoff source: --since when supplied, otherwise
# docs/skycot/agents/heartbeats/conductor.json:last_run_utc.

set -euo pipefail

usage() {
  cat <<'EOF'
Usage: scripts/classify-unread-handoffs.sh [--since <UTC-ISO>]

Reads docs/skycot/agents/SHARED.md entries newer than the conductor
cutoff and classifies each non-conductor handoff as:
  already-landed | queue-needed | operator-only | informational

Read-only: does not edit SHARED.md, BUILD_STATUS.md, PRs, labels, or
branches.
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

SINCE=""
if [[ $# -gt 0 ]]; then
  if [[ "$1" != "--since" || -z "${2:-}" || $# -ne 2 ]]; then
    usage >&2
    exit 2
  fi
  SINCE="$2"
fi

REPO="${ARGO_REPO_ROOT:-$(pwd)}"
SHARED="$REPO/docs/skycot/agents/SHARED.md"
HEARTBEAT="$REPO/docs/skycot/agents/heartbeats/conductor.json"
LEDGER="$REPO/docs/build-plan/BUILD_STATUS.md"

python3 - "$SINCE" "$SHARED" "$HEARTBEAT" "$LEDGER" <<'PY'
from __future__ import annotations

from collections import Counter
from datetime import datetime, timezone
import json
import re
import sys

since_arg, shared_path, heartbeat_path, ledger_path = sys.argv[1:5]


def parse_iso(value: str) -> datetime:
    value = value.strip()
    if not value:
        raise ValueError("empty timestamp")
    return datetime.fromisoformat(value.replace("Z", "+00:00")).astimezone(timezone.utc)


def cutoff() -> tuple[str, datetime]:
    if since_arg:
        return since_arg, parse_iso(since_arg)
    try:
        with open(heartbeat_path, "r", encoding="utf-8") as f:
            raw = str(json.load(f).get("last_run_utc", ""))
    except FileNotFoundError:
        raw = ""
    except Exception as exc:
        print(f"FAIL: read conductor heartbeat: {exc}", file=sys.stderr)
        sys.exit(2)
    if not raw:
        return "<none>", datetime.fromtimestamp(0, timezone.utc)
    return raw, parse_iso(raw)


def ledger_index() -> tuple[dict[str, str], dict[str, str]]:
    by_brief: dict[str, str] = {}
    by_pr: dict[str, str] = {}
    try:
        with open(ledger_path, "r", encoding="utf-8") as f:
            lines = f.readlines()
    except FileNotFoundError:
        return by_brief, by_pr
    for line in lines:
        if not line.startswith("| "):
            continue
        cells = [cell.strip() for cell in line.strip().strip("|").split("|")]
        if len(cells) < 4 or cells[0] in {"Brief", "---"}:
            continue
        brief, _owner, status, pr = cells[:4]
        by_brief[brief] = status
        for match in re.findall(r"#(\d+)", pr + " " + line):
            by_pr[match] = status
    return by_brief, by_pr


def unread_entries(cutoff_dt: datetime) -> list[tuple[str, str, str]]:
    try:
        with open(shared_path, "r", encoding="utf-8") as f:
            lines = f.readlines()
    except FileNotFoundError:
        return []
    out: list[tuple[str, str, str]] = []
    pattern = re.compile(
        r"^(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2})Z?(?:\+\d{4})? \[([a-z-]+)\] (.*)$"
    )
    for line in lines:
        match = pattern.match(line.rstrip("\n"))
        if not match:
            continue
        ts_raw, agent, body = match.groups()
        if agent == "conductor":
            continue
        try:
            ts = parse_iso(f"{ts_raw}Z")
        except ValueError:
            continue
        if ts > cutoff_dt:
            out.append((f"{ts_raw}Z", agent, body))
    return out


def cited_briefs(text: str) -> set[str]:
    found = set(re.findall(r"(?:brief|row|stream|streams/)\s*[:`'\"]?([A-Za-z0-9][A-Za-z0-9_.-]+)", text))
    found.update(re.findall(r"`([A-Za-z0-9][A-Za-z0-9_.-]{5,})`", text))
    return {item.removesuffix(".md") for item in found}


def classify(text: str, by_brief: dict[str, str], by_pr: dict[str, str]) -> tuple[str, str]:
    lowered = text.lower()
    if any(
        marker in lowered
        for marker in (
            "operator-only",
            "outside repo",
            "outside the repo",
            "operator must",
            "operator should",
            "macos permission",
            "record demo",
            "system settings",
        )
    ):
        return "operator-only", "handoff points at operator/outside-repo work"

    for pr in re.findall(r"#(\d+)", text):
        status = by_pr.get(pr)
        if status in {"MERGED", "IN_REVIEW"}:
            return "already-landed", f"PR #{pr} is {status}"

    for brief in cited_briefs(text):
        status = by_brief.get(brief)
        if status in {"MERGED", "IN_REVIEW"}:
            return "already-landed", f"{brief} is {status}"

    plausible = {
        brief
        for brief in cited_briefs(text)
        if any(ch in brief for ch in ("-", "_")) and brief not in by_brief
    }
    if plausible:
        return "queue-needed", "no BUILD_STATUS.md row for " + ", ".join(sorted(plausible))

    return "informational", "no direct ledger action inferred"


try:
    cutoff_label, cutoff_dt = cutoff()
except ValueError as exc:
    print(f"FAIL: cutoff parse error: {exc}", file=sys.stderr)
    sys.exit(2)

by_brief, by_pr = ledger_index()
entries = unread_entries(cutoff_dt)

print(f"cutoff: {cutoff_label}")
if not entries:
    print("OK: no unread non-conductor handoffs to classify.")
    sys.exit(0)

counts: Counter[str] = Counter()
for ts, agent, body in entries:
    category, reason = classify(body, by_brief, by_pr)
    counts[category] += 1
    print(f"{category}\t{ts}\t[{agent}]\t{reason}\t{body}")

summary = ", ".join(f"{name}={counts[name]}" for name in sorted(counts))
print(f"summary: {summary}")
PY
