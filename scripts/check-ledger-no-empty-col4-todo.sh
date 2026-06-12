#!/usr/bin/env bash
# Reject BUILD_STATUS.md TODO rows whose col-4 (PR/commit) is EMPTY.
#
# An empty col-4 on a TODO row (`| <id> | <owner> | TODO | | <notes> |`)
# is a one-character ledger-format slip that, before the tolerant-regex fix
# in scripts/claim-with-retry.sh, made claim-with-retry's apply_edit regex
# reject the row → claim rc=2 → because workers claim the first eligible
# TODO by document order, ONE such malformed first row fast-failed EVERY
# worker and silently starved the whole swarm (root-caused 2026-06-12 —
# swarm at 0 loops for hours while 14 claimable TODOs waited).
#
# The claim regex is now tolerant of empty col-4, so this is the belt to
# that fix's suspenders: keep the malformed format out of the ledger in the
# first place. Unstarted TODO rows use a `—` (em-dash) placeholder in col-4,
# never an empty cell. Wired into scripts/lint-brief.sh (which the brief
# pre-commit hook runs) and runnable standalone in the conductor tick.
#
# Exit 0 = clean; 1 = empty-col4 TODO row(s) found; 2 = usage / no ledger.

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
LEDGER="$REPO_ROOT/docs/build-plan/BUILD_STATUS.md"
QUIET=0

usage() {
  cat <<EOF
usage: scripts/check-ledger-no-empty-col4-todo.sh [--ledger=PATH] [--quiet]

Scans BUILD_STATUS.md for TODO rows whose col-4 (PR/commit) cell is empty
and fails when any are found. An empty col-4 on a TODO row starves the
worker swarm (see header comment). Use a '—' placeholder, never an empty
cell, for an unstarted TODO.

Exit codes:
  0  no empty-col4 TODO rows
  1  one or more empty-col4 TODO rows found
  2  usage error / ledger not found
EOF
}

trim() {
  local s="$1"
  s="${s#"${s%%[![:space:]]*}"}"
  s="${s%"${s##*[![:space:]]}"}"
  printf '%s' "$s"
}

for arg in "$@"; do
  case "$arg" in
    -h|--help) usage; exit 0 ;;
    --ledger=*) LEDGER="${arg#--ledger=}" ;;
    --quiet) QUIET=1 ;;
    --*) echo "error: unknown flag: $arg" >&2; usage >&2; exit 2 ;;
    *) echo "error: unexpected argument: $arg" >&2; usage >&2; exit 2 ;;
  esac
done

if [[ ! -f "$LEDGER" ]]; then
  echo "error: ledger not found: $LEDGER" >&2
  exit 2
fi

failures=0
lineno=0

while IFS= read -r line; do
  lineno=$((lineno + 1))
  [[ "$line" == \|* ]] || continue

  # Fields: _ (pre-pipe) | brief | owner | status | pr | notes...
  IFS='|' read -r _ brief _owner status pr _rest <<<"$line"
  brief="$(trim "${brief:-}")"
  status="$(trim "${status:-}")"
  pr="$(trim "${pr:-}")"

  [[ -z "$brief" || "$brief" == "Brief" || "$brief" == "---"* ]] && continue
  [[ "$status" == "TODO" ]] || continue

  if [[ -z "$pr" ]]; then
    echo "[ledger-empty-col4-todo] line ${lineno}: TODO row '${brief}' has an EMPTY col-4 (PR) — use a '—' placeholder, not '| TODO | |'. An empty col-4 starves the worker swarm. See scripts/check-ledger-no-empty-col4-todo.sh header." >&2
    failures=$((failures + 1))
  fi
done <"$LEDGER"

if (( failures > 0 )); then
  echo "[ledger-empty-col4-todo] ${failures} empty-col4 TODO row(s) — fix before commit (see docs/build-plan/streams/fix-claim-regex-empty-col4-starves-queue.md)" >&2
  exit 1
fi

if [[ "$QUIET" -eq 0 ]]; then
  echo "[ledger-empty-col4-todo] ok"
fi
exit 0
