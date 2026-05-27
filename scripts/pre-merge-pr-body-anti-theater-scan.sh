#!/usr/bin/env bash
# pre-merge-pr-body-anti-theater-scan.sh — gate 9 helper for the
# conductor pre-merge gate.
#
# Reads the GitHub PR body (via `gh pr view <N> --json body`) and
# refuses the merge when the worker's own description admits the
# load-bearing test was never run. Triggered by R97 (2026-05-27): PR
# #2437 admin-merged with the body line "Local cargo check was
# interrupted by file-lock contention from concurrent worker
# compilation" — the fix broke its own regression test, and an entire
# evaluator round was burned catching what the conductor SHOULD have
# caught at merge-gate time.
#
# Counter-pressure is structural (operator urgency × worker quota
# anxiety × cargo contention under concurrent compilation). The fix is
# mechanical: read the body the worker wrote, match canonical
# admission phrases, fail the gate.
#
# An explicit `--allow-anti-theater-admit` override is the operator's
# escape hatch (emergency rollback, doc-only PRs whose body honestly
# admits the original test never ran, etc). The override emits a loud
# warning banner AND appends a `[anti-theater-admit]` audit line to
# `docs/skycot/agents/SHARED.md` so the next evaluator round catches
# the merge and confirms the load-bearing test actually passes cold.
#
# Invoked from `scripts/conductor-pre-merge-gate.sh` as gate 9 (after
# the existing 8 gates). Skips silently when invoked without `--pr`
# (preserves local-only conductor invocations + the existing
# pre-merge-gate-tests.sh smoke harness).
#
# Usage:
#   bash scripts/pre-merge-pr-body-anti-theater-scan.sh                       # no --pr → skip
#   bash scripts/pre-merge-pr-body-anti-theater-scan.sh --pr 2437             # fail if admission phrase present
#   bash scripts/pre-merge-pr-body-anti-theater-scan.sh --pr 2437 --allow-anti-theater-admit
#   bash scripts/pre-merge-pr-body-anti-theater-scan.sh --pr 2437 --quiet     # only print on failure
#
# Env hooks (testing):
#   FAKE_PR_BODY=...           — substitute for `gh pr view <N> --json body`
#   FAKE_SHARED_MD_PATH=/path  — redirect the SHARED.md audit-trail append
#
# Exit codes:
#   0 — no admission phrase, OR --allow-anti-theater-admit override, OR no --pr
#   1 — admission phrase detected and no override

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT" || exit 1

PR_NUMBER=""
ALLOW_ADMIT=0
QUIET=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    --pr)
      PR_NUMBER="${2:-}"
      shift 2
      ;;
    --pr=*)
      PR_NUMBER="${1#*=}"
      shift
      ;;
    --allow-anti-theater-admit)
      ALLOW_ADMIT=1
      shift
      ;;
    --quiet)
      QUIET=1
      shift
      ;;
    -h|--help)
      grep '^#' "$0" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      printf '\033[31m[anti-theater-scan FAIL]\033[0m unknown flag: %s\n' "$1" >&2
      exit 1
      ;;
  esac
done

log()  { [[ "$QUIET" -eq 1 ]] || printf '\033[1m[anti-theater-scan]\033[0m %s\n' "$*"; }
warn() { printf '\033[33m[gate-warning]\033[0m %s\n' "$*" >&2; }
fail() { printf '\033[31m[anti-theater-scan FAIL]\033[0m %s\n' "$*" >&2; }

# Canonical phrase set (case-insensitive substring). Phrases 1-3 are
# verbatim from PR #2437's body — the failure mode R97 named.
# Phrases 4-8 are the obvious paraphrases that future workers might
# reach for. Operator-driven expansion is one-file localized (here).
PHRASES=(
  "local cargo check was interrupted"
  "interrupted by file-lock contention"
  "formal pre-merge gate will run"
  "test was not run"
  "compilation was interrupted"
  "skipped tests due to"
  "did not run the test"
  "cargo test was not executed"
)

# No PR number → skip cleanly. Preserves local-only conductor
# invocations of conductor-pre-merge-gate.sh (the gate emits its own
# "skipped (no --pr)" line, so this script stays quiet).
if [[ -z "$PR_NUMBER" ]]; then
  log "no --pr given — SKIP"
  exit 0
fi

if [[ ! "$PR_NUMBER" =~ ^[0-9]+$ ]]; then
  fail "--pr must be a positive integer; got: $PR_NUMBER"
  exit 1
fi

# Fetch the PR body. Test harness injects FAKE_PR_BODY directly to
# stay hermetic (no real gh calls, no network, no auth required).
if [[ -n "${FAKE_PR_BODY+x}" ]]; then
  BODY="$FAKE_PR_BODY"
else
  if ! command -v gh >/dev/null 2>&1; then
    fail "gh CLI not found; install with: brew install gh"
    exit 1
  fi
  BODY="$(gh pr view "$PR_NUMBER" --json body --jq .body 2>/dev/null || true)"
  if [[ -z "$BODY" ]]; then
    fail "could not fetch PR #$PR_NUMBER body (gh pr view returned empty)"
    exit 1
  fi
fi

# Case-insensitive scan. We track the FIRST matched phrase so the
# warning + audit-trail line cite a single concrete trigger; multiple
# phrases hitting at once would just bloat the output.
BODY_LC="$(printf '%s' "$BODY" | tr '[:upper:]' '[:lower:]')"
MATCHED_PHRASE=""
for phrase in "${PHRASES[@]}"; do
  if [[ "$BODY_LC" == *"$phrase"* ]]; then
    MATCHED_PHRASE="$phrase"
    break
  fi
done

if [[ -z "$MATCHED_PHRASE" ]]; then
  log "PR #$PR_NUMBER body clean — no admission phrase"
  exit 0
fi

# Build a 140-char excerpt centered on the match. Useful for the
# warning output AND the SHARED.md audit-trail line — the operator
# reading either should see WHY the gate fired without re-opening the
# PR. Bash parameter expansion is the right tool here (awk choked on
# multi-line bodies via -v): `${BODY_LC%%"$phrase"*}` returns the
# substring before the first occurrence of phrase, treating phrase as
# literal because of the double-quotes. We then take the length of
# that prefix to locate the match index, and bash substring expansion
# on the original-case body renders the excerpt with the phrase as
# the worker actually wrote it. Newlines collapse to spaces so the
# excerpt fits on the single SHARED.md audit-trail line.
EXCERPT=""
prefix="${BODY_LC%%"$MATCHED_PHRASE"*}"
match_idx=${#prefix}
start=$(( match_idx - 50 ))
if (( start < 0 )); then start=0; fi
EXCERPT="${BODY:$start:140}"
EXCERPT="$(printf '%s' "$EXCERPT" | tr '\n\r\t' '   ' | tr -s ' ')"

# Loud warning banner. Goes to stderr so the operator's eye snaps to
# it even when gate output is being piped through tail/head.
warn ""
warn "================================================================"
warn "PR #$PR_NUMBER body contains anti-theater admission phrase:"
warn "  '$MATCHED_PHRASE'"
warn ""
warn "Excerpt:"
warn "  $EXCERPT"
warn ""
warn "Origin: R97 (2026-05-27) — PR #2437 admin-merged with body line"
warn "  'Local cargo check was interrupted by file-lock contention...'"
warn "  The fix broke its own regression test."
warn ""
warn "Resolutions:"
warn "  1. Worker re-runs the test cold (preferred): cargo test on a"
warn "     clean target dir, confirms green, amends the PR body to"
warn "     remove the admission phrase, force-pushes."
warn "  2. Operator emergency-override: re-run this gate with"
warn "     --allow-anti-theater-admit (logs to SHARED.md for the next"
warn "     evaluator round to verify)."
warn "================================================================"
warn ""

if [[ "$ALLOW_ADMIT" -eq 0 ]]; then
  fail "PR #$PR_NUMBER admission phrase detected and no override flag"
  exit 1
fi

# Override path: append audit trail to SHARED.md (or FAKE path under
# test). The line format is the canonical one specified in the brief
# — evaluators reading SHARED.md within ~6h scan for
# `[anti-theater-admit]` and verify the test actually passes cold.
SHARED_MD_PATH="${FAKE_SHARED_MD_PATH:-$REPO_ROOT/docs/skycot/agents/SHARED.md}"
TS="$(date -u +%Y-%m-%dT%H:%MZ)"
AUDIT_LINE="$TS [anti-theater-admit] PR #$PR_NUMBER admin-merge proceeded over admission phrase '$MATCHED_PHRASE' (operator override --allow-anti-theater-admit). Phrase context: \"$EXCERPT\". Gate 9 expects evaluator round to confirm test actually passes within ~6h."

if [[ -d "$(dirname "$SHARED_MD_PATH")" ]]; then
  printf '%s\n' "$AUDIT_LINE" >> "$SHARED_MD_PATH"
  warn ""
  warn "================================================================"
  warn "OPERATOR OVERRIDE ACCEPTED — gate 9 will return 0."
  warn "Audit line appended to: $SHARED_MD_PATH"
  warn "  $AUDIT_LINE"
  warn "Next evaluator round MUST verify PR #$PR_NUMBER's load-bearing"
  warn "test passes cold cargo. This override is on the record."
  warn "================================================================"
  warn ""
else
  warn "SHARED.md directory missing ($SHARED_MD_PATH) — override accepted but audit trail NOT written"
fi

exit 0
