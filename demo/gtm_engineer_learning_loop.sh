#!/usr/bin/env bash
# GTM Engineer learning-loop demo — exercises the full lifecycle:
#
#   role  →  work  →  gap/pattern  →  learned-skill candidate
#         →  review/promote  →  active skill  →  improved second run
#
# All seven steps are driven by existing argo CLI surfaces against the
# shipped GTM Engineer vertical. The "agent does work" steps
# (analyze_campaign, detect_gtm_pattern, propose_gtm_skill) are
# simulated by direct memory writes that match the JSON shapes those
# skills would emit — so the demo is fully reproducible without a
# paid LLM API key. Set ARGO_BIN to point at a non-PATH binary;
# defaults to `argo`.

set -euo pipefail
trap 'echo; echo "FAILED at line $LINENO" >&2' ERR

# ---------------------------------------------------------------- setup
ARGO_BIN="${ARGO_BIN:-argo}"
DEMO_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$DEMO_DIR/.." && pwd)"

# Caller (e.g. the CI lock test) may pre-set ARGO_HOME / WORK_DIR
# in order to inspect the resulting state after the script exits.
# When unset, we mint our own temp dirs and clean them up on exit.
ARGO_HOME_OWNED=0
WORK_DIR_OWNED=0
if [ -z "${ARGO_HOME:-}" ]; then
  ARGO_HOME="$(mktemp -d -t argo-demo-XXXXXX)"
  ARGO_HOME_OWNED=1
fi
if [ -z "${WORK_DIR:-}" ]; then
  WORK_DIR="$(mktemp -d -t argo-demo-cwd-XXXXXX)"
  WORK_DIR_OWNED=1
fi
ARGO_VERTICALS_DIR="${ARGO_VERTICALS_DIR:-$REPO_ROOT/verticals}"
export ARGO_HOME ARGO_VERTICALS_DIR

cleanup() {
  [ "$ARGO_HOME_OWNED" = "1" ] && rm -rf "$ARGO_HOME"
  [ "$WORK_DIR_OWNED" = "1" ] && rm -rf "$WORK_DIR"
  return 0
}
trap cleanup EXIT

# `argo workspace init` writes argo.toml into cwd, so we run
# from a dedicated work dir to avoid clobbering any existing workspace.
cd "$WORK_DIR"

banner() { echo; echo "═══ $* ═══"; }
fail() { echo "FAIL: $*" >&2; exit 1; }

# Extract the first whitespace-separated token after a known prefix
# (e.g. "appended <uuid> ..." → <uuid>). Reads stdin.
extract_token_after() {
  local prefix="$1"
  awk -v p="^${prefix}" '$0 ~ p { for (i=1; i<=NF; i++) { if ($i == "'"$prefix"'") { print $(i+1); exit } } print $2; exit }'
}

require() {
  command -v "$1" >/dev/null 2>&1 || fail "$1 not on PATH (needed by demo)"
}
require "$ARGO_BIN"
require jq

echo "ARGO_BIN=$ARGO_BIN"
echo "ARGO_HOME=$ARGO_HOME (temp; will be cleaned on exit)"
echo "ARGO_VERTICALS_DIR=$ARGO_VERTICALS_DIR"

# Workspace bootstrap (writes argo.toml + db row in $ARGO_HOME).
"$ARGO_BIN" workspace init >/dev/null

# ----------------------------------------------------- 1. role binding
banner "STEP 1 — role: bind a session to the GTM Engineer vertical"
NEW_OUT=$("$ARGO_BIN" new \
  --provider mock --model mock \
  --vertical gtm-engineer \
  --title "demo: GTM learning loop")
echo "$NEW_OUT" | sed 's/^/  /'
SID=$(printf '%s\n' "$NEW_OUT" \
  | awk '/^created session / { print $3; exit }')
[ -n "$SID" ] || fail "could not parse session id from: $NEW_OUT"

# --------------------------------------------- 2. work: 3 campaigns
banner "STEP 2 — work: operator records 3 campaign outcomes"
echo "  (in a real run, the agent's analyze_campaign skill would emit"
echo "  these memory writes from operator-supplied raw outcomes.)"
for i in 1 2 3; do
  body=$(cat "$DEMO_DIR/data/campaign_${i}_results.json")
  out=$("$ARGO_BIN" memory append memory "$body" \
    --vertical gtm-engineer --session "$SID")
  echo "  $out"
done

# --------------------------------- 3. gap/pattern: 3 obs trigger detect
banner "STEP 3 — gap/pattern: detect_gtm_pattern surfaces a 3-observation pattern"
echo "  (in a real run, the agent's detect_gtm_pattern skill would emit"
echo "  this — three reinforcing observations across distinct campaigns.)"
pattern_body=$(cat "$DEMO_DIR/data/detected_pattern.json")
pat_out=$("$ARGO_BIN" memory append memory "$pattern_body" \
  --vertical gtm-engineer --session "$SID")
echo "  $pat_out"

# ------------------------------------ 4. propose_gtm_skill writes candidate
banner "STEP 4 — candidate: propose_gtm_skill emits a learned-skill candidate"
echo "  (in a real run, the agent's propose_gtm_skill skill would emit"
echo "  this candidate to memory.learned_skill_candidates[]. It is"
echo "  IMMUTABLE once written and is never auto-activated.)"
candidate_body=$(cat "$DEMO_DIR/data/skill_candidate.json")
APPEND_OUT=$("$ARGO_BIN" memory append memory "$candidate_body" \
  --vertical gtm-engineer --session "$SID")
echo "  $APPEND_OUT"
CANDIDATE_ID=$(printf '%s\n' "$APPEND_OUT" \
  | awk '/^appended / { print $2; exit }')
[ -n "$CANDIDATE_ID" ] || fail "could not parse candidate id from: $APPEND_OUT"

# --------------------------------------- 5. review: candidate visible
banner "STEP 5a — review: candidate appears in the operator review queue"
LIST=$("$ARGO_BIN" skill review list --json)
echo "  candidates in queue:"
echo "$LIST" | jq -r '.memory_candidates[]
  | "    [\(.memory_entry_id)] skill_name=\(.skill_name) vertical=\(.vertical_id)"'
echo "$LIST" \
  | jq -e --arg id "$CANDIDATE_ID" \
      '.memory_candidates[] | select(.memory_entry_id == $id)' >/dev/null \
  || fail "candidate $CANDIDATE_ID missing from review queue"

# --------------------------- 5b. promote: materialise as pending_review
banner "STEP 5b — promote: materialise the candidate as a pending_review skill"
PROMOTE=$("$ARGO_BIN" skill review promote "$CANDIDATE_ID" --json)
echo "  $PROMOTE" | jq '.'
SKILL_ID=$(printf '%s' "$PROMOTE" | jq -r '.skill_id')
[ "$(printf '%s' "$PROMOTE" | jq -r '.lifecycle_state')" = "pending_review" ] \
  || fail "expected lifecycle_state=pending_review, got: $PROMOTE"
[ "$(printf '%s' "$PROMOTE" | jq -r '.vertical_id')" = "gtm-engineer" ] \
  || fail "expected vertical_id=gtm-engineer, got: $PROMOTE"

# Confirm it shows in the same review-list surface operators use.
RELIST=$("$ARGO_BIN" skill review list --json)
echo "$RELIST" \
  | jq -e --arg id "$SKILL_ID" \
      '.pending[] | select(.id == $id)' >/dev/null \
  || fail "promoted skill $SKILL_ID not in pending queue"

# ----------------------------------- 6. approve: pending_review → active
banner "STEP 6 — approve: flip pending_review → active"
APPROVE=$("$ARGO_BIN" skill review approve "$SKILL_ID" --json)
echo "  $APPROVE" | jq '.'
[ "$(printf '%s' "$APPROVE" | jq -r '.to')" = "active" ] \
  || fail "expected to=active, got: $APPROVE"

# Confirm the active skill list now includes the promoted one (jsonl).
ACTIVE_LIST=$("$ARGO_BIN" skill list --json)
ACTIVE_NAME=$(printf '%s' "$ACTIVE_LIST" \
  | jq -r --arg id "$SKILL_ID" 'select(.id == $id) | .name')
[ -n "$ACTIVE_NAME" ] && [ "$ACTIVE_NAME" != "null" ] \
  || fail "promoted skill $SKILL_ID not visible in argo skill list --json"
ACTIVE_COUNT=$(printf '%s\n' "$ACTIVE_LIST" | jq -s 'length')
echo "  active skills now: $ACTIVE_COUNT (includes \"$ACTIVE_NAME\")"

# ------------------------------ 7. improved second run: scored eval
banner "STEP 7 — improved second run: scored eval confirms platform health"
echo "  The promoted skill body is now installed and visible to future agent"
echo "  runs. The next analyze_campaign turn that matches the trigger"
echo "  conditions (technical founder + agent-platform GTM hypothesis) will"
echo "  consult \"$ACTIVE_NAME\" instead of re-deriving the framing."
echo
echo "  We re-run the scored eval suite to confirm the platform stayed"
echo "  green throughout — it is the same suite + actuals that the project"
echo "  ships as known-good."
EVAL=$("$ARGO_BIN" vertical eval gtm-engineer \
  --score \
  --actuals "$REPO_ROOT/verticals/gtm-engineer/eval/known_good_actuals.jsonl" \
  --json)
PASS_RATE=$(printf '%s' "$EVAL" | jq -r '.suites[0].pass_rate')
PASSED=$(printf '%s' "$EVAL" | jq -r '.suites[0].passed')
SCORED=$(printf '%s' "$EVAL" | jq -r '.suites[0].scored_count')
echo "  eval: passed=$PASSED / scored=$SCORED  (pass_rate=$PASS_RATE)"
EVAL_OK=$(printf '%s' "$EVAL" | jq -r '.ok')
[ "$EVAL_OK" = "true" ] \
  || fail "eval.ok != true; full body:\n$EVAL"

# -------------------------------------------------------------- recap
banner "RECAP"
cat <<EOF
  Session:              $SID
  Memory candidate:     $CANDIDATE_ID
  Promoted skill:       $SKILL_ID
  Active skill name:    $ACTIVE_NAME
  Active skill count:   $ACTIVE_COUNT
  Eval pass rate:       $PASS_RATE  ($PASSED/$SCORED)

  This demo exercised the full role → work → gap/pattern → candidate
  → review → promote → active loop end-to-end against the existing
  GTM Engineer vertical, using only shipped argo CLI surfaces.

  The "agent does work" steps were simulated by direct memory writes
  (campaign_results, patterns, learned_skill_candidates) using the
  exact JSON shapes the corresponding skills would emit. The
  review → promote → approve → active path was driven by the real
  argo skill review CLI and produced a real lifecycle_state=active
  skill row, visible to the next agent run that matches the new
  skill's trigger conditions.

  To exercise the LLM-driven path end-to-end (so the agent really
  produces the candidate from raw campaign data), set
  ANTHROPIC_API_KEY and re-run the same flow with
    --provider anthropic --model claude-sonnet-4-6
  on the argo new line. The review/promote/approve steps stay
  identical because the operator-side surface is provider-agnostic.
EOF
echo
echo "OK — demo completed successfully."
