#!/usr/bin/env bash
# ledger-first-todo.sh — the ONE ledger row-parser shared by the brief PICKER
# (scripts/claude-worker-loop.sh) and the CLAIMER (scripts/claim-with-retry.sh).
#
# Root-fix for the 2026-06-12 *second* swarm-starvation incident. The picker
# used a greedy whole-line regex:
#
#     awk -F'|' '/^\| .* \| .* \| TODO \|/ { print $2 }'
#
# whose `.*` atoms match pipes, so the status match was NOT anchored to the
# status column. Any row whose DESCRIPTION cell contained a literal
# `| TODO |` substring (e.g. a brief that quoted a malformed row shape it had
# fixed) was mis-picked as the "first TODO" regardless of its real status.
# The claimer (claim-with-retry.sh apply_edit) was already column-anchored and
# correctly REFUSED the row, so picker and claimer disagreed: one malformed
# IN_REVIEW row fast-failed every worker and starved the whole queue for ~40
# min until the row was reworded pipe-free.
#
# Fix: parse the STATUS column (field 4) DIRECTLY instead of regex-matching the
# whole line. Both callers source this file so "what is a TODO row" can never
# diverge between pick and claim again (subtract before adding — one parser,
# two callers).
#
# Field layout (awk -F'|', leading pipe ⇒ $1 is the empty pre-pipe cell):
#   $2 = brief-id   $3 = owner   $4 = status   $5 = PR/commit   $6.. = description
#
# Owner (field 3) is intentionally NOT matched: any worker may claim any TODO
# row regardless of the owner column (SWARM_PROTOCOL.md HARD RULE 11 — the
# owner/prefix is a sizing hint, not a lock; the strict `(claude|codex)`
# owner-match was removed 2026-05-27).
#
# Sourced API (preferred — no subprocess; used by both callers):
#   ledger_todo_ids   <ledger>             # print every TODO brief-id, doc order
#   ledger_first_todo <ledger>             # print the first TODO brief-id (or empty)
#   ledger_status_of  <ledger> <brief_id>  # echo todo|claimed|blocked|other|not-found
#
# CLI form (ad-hoc / the brief's "Verify" step):
#   scripts/ledger-first-todo.sh <ledger>                 # first TODO brief-id
#   scripts/ledger-first-todo.sh --all <ledger>           # all TODO brief-ids
#   scripts/ledger-first-todo.sh --status <ledger> <id>   # one row's status token

# ledger_todo_ids <ledger>
#   Print, in document order, the brief-id of every row whose STATUS column is
#   exactly TODO. Column-anchored: a literal `| TODO |` inside a description
#   cell does NOT match (that is the bug this file exists to kill).
ledger_todo_ids() {
    local ledger="$1"
    awk -F'|' '
      NF >= 5 {
        s = $4; gsub(/^[ \t]+|[ \t]+$/, "", s)
        if (s == "TODO") {
          id = $2; gsub(/^[ \t]+|[ \t]+$/, "", id)
          if (id != "") print id
        }
      }' "$ledger"
}

# ledger_first_todo <ledger>
#   Convenience: the first TODO brief-id in document order, or empty string.
ledger_first_todo() {
    ledger_todo_ids "$1" | head -1
}

# ledger_status_of <ledger> <brief_id>
#   Echo the status token for the first row whose brief-id (field 2) matches:
#   todo | claimed | blocked | other | not-found. Same field-4 anchor as the
#   picker, so the claimer's classification can never disagree with the pick.
ledger_status_of() {
    local ledger="$1" brief_id="$2"
    awk -F'|' -v want="$brief_id" '
      NF >= 5 {
        id = $2; gsub(/^[ \t]+|[ \t]+$/, "", id)
        if (id == want) {
          s = $4; gsub(/^[ \t]+|[ \t]+$/, "", s)
          if (s == "TODO")         print "todo"
          else if (s == "CLAIMED") print "claimed"
          else if (s == "BLOCKED") print "blocked"
          else                     print "other"   # IN_REVIEW / MERGED / header
          found = 1
          exit
        }
      }
      END { if (!found) print "not-found" }' "$ledger"
}

# Run the CLI dispatcher only when executed directly; sourcing (the picker and
# claimer path) exposes the functions without side effects.
if [[ "${BASH_SOURCE[0]:-$0}" == "${0}" ]]; then
    set -uo pipefail
    case "${1:-}" in
        --all)
            ledger_todo_ids "${2:?usage: $0 --all <ledger>}"
            ;;
        --status)
            ledger_status_of "${2:?usage: $0 --status <ledger> <brief-id>}" \
                             "${3:?usage: $0 --status <ledger> <brief-id>}"
            ;;
        "" )
            echo "usage: $0 [--all | --status <brief-id>] <ledger>" >&2
            exit 2
            ;;
        *)
            ledger_first_todo "$1"
            ;;
    esac
fi
