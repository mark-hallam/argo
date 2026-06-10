#!/usr/bin/env bash
# Conductor stash-pop ledger-row guard for docs/build-plan/BUILD_STATUS.md.
#
# Guards the failure mode documented in R98/R99 (2026-05-27):
#   1.  conductor stages a new BUILD_STATUS.md row (claim, IN_REVIEW flip,
#       BLOCKED note, ...)
#   2.  `git stash push` to make room for `git pull --rebase`
#   3.  `git stash pop` after the pull
#   4.  if a worker push landed in the same hunk window, the 3-way merge
#       silently picks the worker's text and drops the conductor's row
#   5.  conductor commit+pushes a working tree that LOOKS clean but is
#       missing the row — surfaces only as a "ghost-stamp" commit one or
#       two evaluator rounds later
#
# This script diffs BUILD_STATUS.md against HEAD and refuses (or warns,
# under --warn) if a `| TODO |` or `| CLAIMED |` row is DELETED without
# the same brief-id reappearing elsewhere in the file (benign reorders
# are ignored).
#
# Usage:
#   scripts/check-ledger-not-eaten-by-stash-pop.sh           # working tree vs HEAD
#   scripts/check-ledger-not-eaten-by-stash-pop.sh --staged  # staged copy vs HEAD
#   scripts/check-ledger-not-eaten-by-stash-pop.sh --warn    # warn-only (exit 0)
#
# Exit codes:
#   0  no TODO/CLAIMED row dropped, or --warn supplied
#   1  TODO/CLAIMED row dropped without relocation
#   64 bad CLI args
#
# Test hook:
#   BUILD_STATUS_PATH  override the ledger path (default
#                      docs/build-plan/BUILD_STATUS.md). Used by the smoke
#                      harness; UNSET in production.

set -uo pipefail

LEDGER_PATH="${BUILD_STATUS_PATH:-docs/build-plan/BUILD_STATUS.md}"
mode=working
warn_only=0

while (( $# > 0 )); do
    case "$1" in
        --staged) mode=staged ;;
        --warn)   warn_only=1 ;;
        --help|-h)
            sed -n '2,33p' "$0"
            exit 0
            ;;
        *)
            echo "unknown arg: $1" >&2
            echo "usage: $0 [--staged] [--warn]" >&2
            exit 64
            ;;
    esac
    shift
done

# Capture the diff. Both branches scope to the ledger path so unrelated
# staged/working changes are invisible here.
if [[ "$mode" == staged ]]; then
    diff_output="$(git diff --cached HEAD -- "$LEDGER_PATH" 2>/dev/null || true)"
    current_content="$(git show ":$LEDGER_PATH" 2>/dev/null || true)"
else
    diff_output="$(git diff HEAD -- "$LEDGER_PATH" 2>/dev/null || true)"
    current_content="$(cat "$LEDGER_PATH" 2>/dev/null || true)"
fi

[[ -z "$diff_output" ]] && exit 0

dropped=()

while IFS= read -r line; do
    # Only deletions of pipe-table rows.
    [[ "$line" =~ ^-\|[[:space:]] ]] || continue
    # Filter to ledger-status rows only. Section headers carry `| — |`
    # placeholders in the status column, so this naturally skips them.
    if [[ "$line" != *"| TODO |"* && "$line" != *"| CLAIMED |"* ]]; then
        continue
    fi
    # Extract brief-id (first table column).
    rest="${line#-| }"
    brief_id="${rest%% |*}"
    # Trim trailing/leading whitespace.
    brief_id="${brief_id%"${brief_id##*[![:space:]]}"}"
    brief_id="${brief_id#"${brief_id%%[![:space:]]*}"}"
    [[ -z "$brief_id" ]] && continue
    # Section markers wrap their first column in `**...**`. Skip those.
    [[ "$brief_id" == \*\** ]] && continue
    # Relocation check: brief-id still present somewhere in the current
    # file? Pure-bash fixed-string substring match — no subprocess, so it
    # is structurally immune to the SIGPIPE race that bit the old
    # `printf … | grep -qF` form: under `set -o pipefail`, grep's
    # first-match early-exit closed its stdin while printf was still
    # streaming the 2.3 MB ledger, printf took SIGPIPE (exit 141), the
    # pipeline reported 141, and the `if` evaluated FALSE — falsely
    # refusing a valid in-place row edit (CLAIMED→IN_REVIEW). The quoted
    # `*"..."*` pattern means even a hostile brief-id can't glob;
    # kebab-case ids carry no glob metacharacters anyway. Newlines are
    # irrelevant to fixed-string containment, so semantics match grep -F.
    if [[ "$current_content" == *"| $brief_id |"* ]]; then
        continue
    fi
    dropped+=("$brief_id")
done <<<"$diff_output"

if (( ${#dropped[@]} == 0 )); then
    exit 0
fi

{
    echo ""
    echo "🛑 ledger row dropped from $LEDGER_PATH without relocation:"
    for b in "${dropped[@]}"; do
        echo "   - $b"
    done
    echo ""
    echo "TODO/CLAIMED rows were deleted in this change set and the brief-ids"
    echo "do not reappear elsewhere in the file. This is almost always a"
    echo "'git stash pop' that ate a row when a worker push landed in the"
    echo "same hunk window. Recovery:"
    echo ""
    echo "  1.  git stash list                                              # find the stash"
    echo "  2.  git show stash@{0} -- $LEDGER_PATH    # see what was there"
    echo "  3.  manually re-add the row + create a fresh commit"
    echo ""
    echo "Run with --warn to bypass while debugging."
} >&2

if (( warn_only == 1 )); then
    echo "(warn-only: continuing despite detection)" >&2
    exit 0
fi
exit 1
