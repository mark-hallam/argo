#!/usr/bin/env bash
# Check active BUILD_STATUS.md rows for missing stream brief files.

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
LEDGER="$REPO_ROOT/docs/build-plan/BUILD_STATUS.md"
STREAMS_DIR="$REPO_ROOT/docs/build-plan/streams"
QUIET=0

usage() {
  cat <<EOF
usage: scripts/check-brief-files-exist.sh [--ledger=PATH] [--streams-dir=PATH] [--quiet]

Scans BUILD_STATUS.md for active rows that cite Brief: streams/<id>.md and
fails when the referenced docs/build-plan/streams/<id>.md file is missing.
EOF
}

trim() {
  local s="$1"
  s="${s#"${s%%[![:space:]]*}"}"
  s="${s%"${s##*[![:space:]]}"}"
  printf '%s' "$s"
}

is_inactive_status() {
  case "$1" in
    BLOCKED|MERGED|DROPPED|SUPERSEDED|ARCHIVED) return 0 ;;
  esac
  return 1
}

for arg in "$@"; do
  case "$arg" in
    -h|--help) usage; exit 0 ;;
    --ledger=*) LEDGER="${arg#--ledger=}" ;;
    --streams-dir=*) STREAMS_DIR="${arg#--streams-dir=}" ;;
    --quiet) QUIET=1 ;;
    --*) echo "error: unknown flag: $arg" >&2; usage >&2; exit 2 ;;
    *) echo "error: unexpected argument: $arg" >&2; usage >&2; exit 2 ;;
  esac
done

if [[ ! -f "$LEDGER" ]]; then
  echo "error: ledger not found: $LEDGER" >&2
  exit 2
fi
if [[ ! -d "$STREAMS_DIR" ]]; then
  echo "error: streams dir not found: $STREAMS_DIR" >&2
  exit 2
fi

failures=0

while IFS= read -r line; do
  [[ "$line" == \|* ]] || continue

  IFS='|' read -r _ brief owner status pr notes <<<"$line"
  brief="$(trim "${brief:-}")"
  status="$(trim "${status:-}")"
  notes="${notes:-}"

  [[ -z "$brief" || "$brief" == "Brief" || "$brief" == "---"* ]] && continue
  is_inactive_status "$status" && continue

  while IFS= read -r cited_path; do
    [[ -z "$cited_path" ]] && continue
    [[ "$cited_path" == streams/archive/* ]] && continue

    target="$STREAMS_DIR/${cited_path#streams/}"
    if [[ ! -f "$target" ]]; then
      echo "[brief-files-exist] missing brief file: brief=${brief} status=${status} path=${cited_path}" >&2
      failures=$((failures + 1))
    fi
  done < <(grep -oE 'Brief:[[:space:]]*streams/[^` ,)|]+\.md' <<<"$notes" \
    | sed -E 's/^Brief:[[:space:]]*//')
done <"$LEDGER"

if (( failures > 0 )); then
  echo "[brief-files-exist] ${failures} missing active brief file reference(s)" >&2
  exit 1
fi

if [[ "$QUIET" -eq 0 ]]; then
  echo "[brief-files-exist] ok"
fi
exit 0
