#!/usr/bin/env bash
# stop-gate-verify-agent.sh — DEC-042 Stage-1 B-4: the SEPARATE verify-agent.
#
# B-2 (stop-gate) re-runs DETERMINISTIC gates, but in the SAME loop that drove the
# builder `claude -p` session — nothing reviews the change as an agent with FRESH
# context (does the diff satisfy the brief's acceptance criteria? do the added
# tests pin the claimed behavior, or are they theater?). DEC-042 §3 B-4: "the
# agent that wrote the code is biased when grading it ... make a separate
# verify-agent a required stage of every loop ... Verifiable by: every merged
# change carries a verify-agent verdict from a DIFFERENT context than the author."
# This script IS that stage: it spawns a fresh, read-only `claude -p` verify
# session (a different context by construction — new process, new context window,
# no write tools), records a machine-parseable verdict, posts it on the PR, and on
# FAIL holds the row IN_REVIEW->NEEDS-FIX. It COMPOSES with B-2 (the worker loop
# runs the cheap deterministic stop-gate first; this is the judgment layer on top,
# only when that gate did not already hold the row).
#
# No-op (VERIFY_AGENT_PASS, exit 0, no side effects) unless ARGO_SEPARATE_VERIFIER=1
# — flag unset = byte-identical to pre-B-4 (same discipline as B-2's ARGO_STOP_GATE).
#
# Usage:
#   ARGO_SEPARATE_VERIFIER=1 scripts/stop-gate-verify-agent.sh \
#       --brief <id> --branch <branch> [--pr <n>] [--repo <path>]
#
# Output contract (last line is machine-parseable):
#   VERIFY_AGENT_PASS
#   VERIFY_AGENT_FAIL: <one-line reason>
#
# Exit codes:
#   0 = verdict PASS, OR disabled (ARGO_SEPARATE_VERIFIER != 1) — no-op
#   1 = verdict FAIL (row moved IN_REVIEW->NEEDS-FIX, reason on the PR)
#   2 = invalid arguments
#
# Test seams (env) — scripts/test-verify-agent.sh injects a verdict, NO model/network:
#   VERIFY_AGENT_CMD            — replace `claude -p`; stdout parsed for the verdict line
#   VERIFY_AGENT_SKIP_PR_COMMENT=1 — do not call `gh pr comment`
#   VERIFY_AGENT_TIMEOUT_S      — wall-clock budget for the verify session (default 600)

set -uo pipefail

REPO="$(cd "$(dirname "$0")/.." && pwd)"
BRIEF=""
BRANCH=""
PR=""
TIMEOUT_S="${VERIFY_AGENT_TIMEOUT_S:-600}"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --brief)    BRIEF="${2:-}"; shift 2 ;;
    --brief=*)  BRIEF="${1#*=}"; shift ;;
    --branch)   BRANCH="${2:-}"; shift 2 ;;
    --branch=*) BRANCH="${1#*=}"; shift ;;
    --pr)       PR="${2:-}"; shift 2 ;;
    --pr=*)     PR="${1#*=}"; shift ;;
    --repo)     REPO="${2:-}"; shift 2 ;;
    --repo=*)   REPO="${1#*=}"; shift ;;
    -h|--help)  grep '^#' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *)          echo "stop-gate-verify-agent: unknown arg: $1" >&2; exit 2 ;;
  esac
done

# DISABLED no-op (no-behavior-change proof, mirrors stop-gate-verify.sh): returns
# BEFORE any VERIFY_AGENT_CMD runs — test case C asserts this via a side-effect file.
if [[ "${ARGO_SEPARATE_VERIFIER:-0}" != "1" ]]; then
  echo "VERIFY_AGENT_PASS"
  exit 0
fi

if [[ -z "$BRIEF" ]]; then
  echo "stop-gate-verify-agent: --brief is required" >&2
  echo "VERIFY_AGENT_FAIL: missing --brief"
  exit 2
fi

LEDGER="$REPO/docs/build-plan/BUILD_STATUS.md"

# Derive the PR number from the ledger row when --pr was not passed (the worker
# loop wires this without --pr; mirrors stop-gate-enforce.sh's row scrape).
if [[ -z "$PR" && -f "$LEDGER" ]]; then
  row="$(grep -E "^\| ${BRIEF} \|" "$LEDGER" 2>/dev/null | head -1)"
  PR="$(printf '%s' "$row" | grep -oE '#[0-9]+' | head -1 | tr -d '#')"
fi

echo "[verify-agent] spawning fresh read-only verify session for $BRIEF (branch $BRANCH)" >&2

# shellcheck disable=SC2329  # invoked indirectly: `export -f` + bounded `bash -c "run_real_claude"`
run_real_claude() {
  # A different context from the builder by construction: a brand-new `claude -p`
  # process, read-only review prompt, NO write tools (no --dangerously-skip-permissions).
  local brief_file="$REPO/docs/build-plan/streams/${BRIEF}.md"
  local criteria diff prompt
  criteria="$( [[ -f "$brief_file" ]] && cat "$brief_file" || echo "(brief file not found: $brief_file)" )"
  diff="$( cd "$REPO" && git diff "origin/main...${BRANCH}" 2>/dev/null | head -4000 )"
  prompt="You are an INDEPENDENT verify-agent (DEC-042 B-4), reviewing with FRESH
context a change you did NOT write. Judge ONLY two things: (1) does the diff
satisfy the brief's acceptance criteria, and (2) do the added/changed tests
actually pin the claimed behavior (not theater)? Do not run commands or edit
files. Your response MUST end with a single machine-parseable LAST line, exactly
one of:
  VERIFY_AGENT_PASS
  VERIFY_AGENT_FAIL: <one-line reason>

=== BRIEF + ACCEPTANCE CRITERIA (${BRIEF}) ===
${criteria}

=== BRANCH DIFF (origin/main...${BRANCH}, truncated to 4000 lines) ===
${diff}"
  # Read-only: restrict to non-mutating tools; no --dangerously-skip-permissions.
  printf '%s' "$prompt" | claude -p --allowedTools "Read,Grep,Glob" 2>&1
}

# Portable bounded run (macOS ships no `timeout`): prefer gtimeout/timeout, else a
# background-process + watcher fallback. On timeout -> FAIL; the loop is never hung.
bounded() {
  if command -v timeout >/dev/null 2>&1; then
    timeout "$TIMEOUT_S" bash -c "$1"; return $?
  elif command -v gtimeout >/dev/null 2>&1; then
    gtimeout "$TIMEOUT_S" bash -c "$1"; return $?
  fi
  bash -c "$1" & local pid=$!
  ( sleep "$TIMEOUT_S"; kill -TERM "$pid" 2>/dev/null ) & local watcher=$!
  wait "$pid" 2>/dev/null; local rc=$?
  kill "$watcher" 2>/dev/null; wait "$watcher" 2>/dev/null
  return $rc
}

out=""; rc=0
if [[ -n "${VERIFY_AGENT_CMD:-}" ]]; then
  out="$( cd "$REPO" && bash -c "$VERIFY_AGENT_CMD" 2>&1 )"; rc=$?
else
  export -f run_real_claude       # so the bounded `bash -c` subshell sees it + inputs
  export REPO BRIEF BRANCH
  out="$( bounded run_real_claude )"; rc=$?
fi

# Parse the machine-parseable verdict (last marker line wins).
verdict="$(printf '%s\n' "$out" | grep -E '^VERIFY_AGENT_(PASS|FAIL)' | tail -1)"
reason=""
if [[ "$rc" -eq 124 ]]; then
  verdict="VERIFY_AGENT_FAIL: verify session timed out"
fi
case "$verdict" in
  VERIFY_AGENT_PASS) ;;
  VERIFY_AGENT_FAIL:*) reason="${verdict#VERIFY_AGENT_FAIL:}"; reason="${reason# }" ;;
  *) verdict="VERIFY_AGENT_FAIL: verify session produced no parseable verdict"
     reason="verify session produced no parseable verdict" ;;
esac

post_comment() {  # $1=body
  [[ "${VERIFY_AGENT_SKIP_PR_COMMENT:-0}" = "1" ]] && return 0
  [[ -n "$PR" ]] || return 0
  command -v gh >/dev/null 2>&1 || return 0
  gh pr comment "$PR" --body "$1" 2>&1 | tail -2 || true
}

if [[ -z "$reason" ]]; then
  echo "[verify-agent] ✅ $BRIEF PASSED independent verify; IN_REVIEW stands." >&2
  post_comment "🔎 **verify-agent (DEC-042 B-4): PASS** — an independent, read-only verify session (different context from the author) judged this change to satisfy the brief's acceptance criteria and its tests to pin the claimed behavior."
  echo "VERIFY_AGENT_PASS"
  exit 0
fi

echo "[verify-agent] 🛑 $BRIEF FAILED independent verify — $reason; moving IN_REVIEW->NEEDS-FIX." >&2

# Flip the row so a rejected change can never PRESENT as done. NEEDS-FIX (not TODO)
# parks it for a human/conductor (the awk picker only re-claims TODO). NB: MIRRORS
# the row-flip in scripts/stop-gate-enforce.sh (B-2) — keep the two regexes in sync.
if [[ -f "$LEDGER" ]]; then
  VERIFY_AGENT_REASON="$reason" python3 - "$BRIEF" "$REPO" <<'PY'
import re, os, sys, pathlib
brief, repo = sys.argv[1:3]
reason = os.environ.get("VERIFY_AGENT_REASON", "verify-agent failed")
p = pathlib.Path(repo) / "docs/build-plan/BUILD_STATUS.md"
text = p.read_text()
note = (f" **HELD by verify-agent (DEC-042 B-4 — independent fresh-context review):** "
        f"PR opened but a separate read-only verify session rejected it: {reason}. "
        f"Row moved IN_REVIEW->NEEDS-FIX; address the finding and refresh the PR.")
new, n = re.subn(
    rf"^(\| {re.escape(brief)} \| (?:claude|codex)) \| IN_REVIEW \|(.*) \|$",
    lambda m: f"{m.group(1)} | NEEDS-FIX |{m.group(2)}{note} |",
    text, count=1, flags=re.MULTILINE,
)
if n:
    p.write_text(new)
PY
fi

# Serialized commit+push only when the loop's helpers are present (absent in the
# standalone meta-verifier, which asserts the on-disk row — same as B-2's enforce).
if command -v with_ledger_lock >/dev/null 2>&1 && command -v ledger_commit_push >/dev/null 2>&1; then
  with_ledger_lock ledger_commit_push "status: verify-agent held $BRIEF (independent review failed: $reason)"
fi

post_comment "🔎 **verify-agent (DEC-042 B-4) held this PR.** Every change must carry a verdict from a context different than the author's, and the independent read-only verify session rejected this one:

\`\`\`
${reason}
\`\`\`

The ledger row was moved IN_REVIEW→NEEDS-FIX. Address the finding above and refresh the PR."

echo "VERIFY_AGENT_FAIL: $reason"
exit 1
