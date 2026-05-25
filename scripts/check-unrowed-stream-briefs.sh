#!/usr/bin/env bash
# check-unrowed-stream-briefs.sh -- report stream briefs missing ledger rows.
#
# Exit codes:
#   0  every non-allowlisted stream brief has a BUILD_STATUS.md row
#   1  one or more stream briefs are missing from the ledger
#   2  usage or input-file error

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
STREAMS_DIR="$ROOT/docs/build-plan/streams"
LEDGER="$ROOT/docs/build-plan/BUILD_STATUS.md"

# Historical/non-queue docs can live beside operational briefs. Keep this
# exact and small: if a stream is meant to be claimable, add a ledger row.
ALLOWLIST=(
  "adr-frontend-design-language"
  "adr-postgres-support-path"
)

usage() {
  cat <<'EOF'
Usage: scripts/check-unrowed-stream-briefs.sh [--streams-dir PATH] [--ledger PATH]

Reports docs/build-plan/streams/*.md files whose stem is absent from
docs/build-plan/BUILD_STATUS.md, excluding the script's inline allowlist.
EOF
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --streams-dir)
      if [ -z "${2:-}" ]; then
        usage >&2
        exit 2
      fi
      STREAMS_DIR="$2"
      shift 2
      ;;
    --ledger)
      if [ -z "${2:-}" ]; then
        usage >&2
        exit 2
      fi
      LEDGER="$2"
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

python3 - "$STREAMS_DIR" "$LEDGER" "${ALLOWLIST[@]}" <<'PY'
from pathlib import Path
import sys

streams_dir = Path(sys.argv[1])
ledger_path = Path(sys.argv[2])
allowlist = set(sys.argv[3:])
statuses = {
    "TODO",
    "CLAIMED",
    "IN_REVIEW",
    "CHANGES-REQUESTED",
    "MERGED",
    "MERGED-FINDINGS-ONLY",
    "BLOCKED",
}

if not streams_dir.is_dir():
    print(f"error: streams dir not found: {streams_dir}", file=sys.stderr)
    raise SystemExit(2)
if not ledger_path.is_file():
    print(f"error: ledger not found: {ledger_path}", file=sys.stderr)
    raise SystemExit(2)

ledger_ids = set()
for line in ledger_path.read_text(encoding="utf-8").splitlines():
    if not line.startswith("| "):
        continue
    parts = [part.strip() for part in line.strip().strip("|").split("|")]
    if len(parts) >= 4 and parts[2] in statuses:
        ledger_ids.add(parts[0])

stream_ids = {
    path.stem
    for path in streams_dir.glob("*.md")
    if path.is_file() and not path.name.startswith(".")
}
missing = sorted(stream_ids - ledger_ids - allowlist)

if missing:
    print("unrowed-stream-briefs:")
    for brief_id in missing:
        print(brief_id)
    raise SystemExit(1)

print("unrowed-stream-briefs: ok")
PY
