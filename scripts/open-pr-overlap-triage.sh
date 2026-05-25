#!/usr/bin/env bash
# Read-only helper for conductor PR review: report open PRs that touch
# the same non-doc file path, so merge order and rebases are explicit.

set -euo pipefail

GH_BIN="${OPEN_PR_OVERLAP_GH_BIN:-gh}"

usage() {
  cat <<'EOF'
Usage: scripts/open-pr-overlap-triage.sh

Lists open GitHub pull requests, fetches their changed files, and
prints non-doc file paths shared by two or more PRs. The script is
read-only: it does not comment, label, merge, or mutate git state.
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi
if [[ $# -ne 0 ]]; then
  usage >&2
  exit 2
fi

if ! command -v "$GH_BIN" >/dev/null 2>&1; then
  echo "FAIL: cannot find gh binary '$GH_BIN'" >&2
  exit 2
fi
if ! command -v python3 >/dev/null 2>&1; then
  echo "FAIL: python3 is required" >&2
  exit 2
fi

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

"$GH_BIN" pr list --state open --json number,title,headRefName >"$tmp/prs.json"

python3 - "$tmp/prs.json" >"$tmp/prs.tsv" <<'PY'
import json
import sys

with open(sys.argv[1], "r", encoding="utf-8") as f:
    rows = json.load(f)

for row in rows:
    number = str(row.get("number", "")).strip()
    if not number:
        continue
    title = str(row.get("title", "")).replace("\t", " ").replace("\n", " ")
    branch = str(row.get("headRefName", "")).replace("\t", " ").replace("\n", " ")
    print(f"{number}\t{title}\t{branch}")
PY

if [[ ! -s "$tmp/prs.tsv" ]]; then
  echo "OK: no open PRs to compare."
  exit 0
fi

while IFS=$'\t' read -r number _title _branch; do
  "$GH_BIN" pr view "$number" --json files >"$tmp/pr-$number.json"
done <"$tmp/prs.tsv"

python3 - "$tmp" <<'PY'
from collections import defaultdict
import json
import os
import sys

tmp = sys.argv[1]
prs = {}
with open(os.path.join(tmp, "prs.tsv"), "r", encoding="utf-8") as f:
    for line in f:
        number, title, branch = line.rstrip("\n").split("\t", 2)
        prs[number] = {"title": title, "branch": branch}

paths = defaultdict(set)
for number in prs:
    with open(os.path.join(tmp, f"pr-{number}.json"), "r", encoding="utf-8") as f:
        data = json.load(f)
    for item in data.get("files", []):
        path = item if isinstance(item, str) else item.get("path")
        if path:
            paths[path].add(number)

all_overlaps = {path: nums for path, nums in paths.items() if len(nums) >= 2}
code_overlaps = {
    path: nums
    for path, nums in all_overlaps.items()
    if not path.startswith("docs/")
}

def pr_list(nums):
    def key(value):
        try:
            return int(value)
        except ValueError:
            return value
    return ", ".join(f"#{n}" for n in sorted(nums, key=key))

if not code_overlaps:
    suffix = ""
    if all_overlaps:
        suffix = f" Ignored {len(all_overlaps)} docs-only overlap(s)."
    print(f"OK: no code file overlaps across {len(prs)} open PR(s).{suffix}")
    sys.exit(0)

print(f"Found {len(code_overlaps)} overlapping code path(s) across open PRs:")
for path in sorted(code_overlaps):
    nums = code_overlaps[path]
    print(f"OVERLAP {path}: {pr_list(nums)}")
    for number in sorted(nums, key=lambda n: int(n) if n.isdigit() else n):
        meta = prs[number]
        print(f"  #{number} {meta['branch']} - {meta['title']}")
    print(
        "RECOMMENDATION "
        f"{path}: merge one PR first, then rebase/recheck the remaining PR(s)."
    )
PY
