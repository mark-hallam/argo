#!/usr/bin/env bash
# Recruiter vertical learning-loop demo. Deterministic mock-mode by
# default; no external ATS, LinkedIn, calendar, or Slack credentials.

set -euo pipefail
trap 'echo "FAIL: recruiter demo failed at line $LINENO" >&2' ERR

PACK_ID="recruiter"
ROLE_NAME="Recruiter"
FIRST_TASK="screen a 10-resume backend-candidate batch"
REFINED_SKILL="screen-candidate"
SKILLS=(screen-candidate schedule-screen draft-rejection source-from-linkedin extract-resume-fields)

MOCK_MODE=1
while [ $# -gt 0 ]; do
  case "$1" in
    --mock-mode) MOCK_MODE=1 ;;
    -h|--help)
      echo "Usage: $0 [--mock-mode]"
      exit 0
      ;;
    *) echo "FAIL: unknown argument: $1" >&2; exit 2 ;;
  esac
  shift
done
[ "$MOCK_MODE" = "1" ] || { echo "FAIL: only --mock-mode is supported" >&2; exit 2; }

ARGO_BIN="${ARGO_BIN:-argo}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
PACK_DIR="$REPO_ROOT/packs/$PACK_ID"
OWN_WORK=0
OWN_HOME=0
if [ -z "${WORK_DIR:-}" ]; then
  WORK_DIR="$(mktemp -d -t argo-${PACK_ID}-demo-XXXXXX)"
  OWN_WORK=1
fi
if [ -z "${ARGO_HOME:-}" ]; then
  ARGO_HOME="$(mktemp -d -t argo-${PACK_ID}-home-XXXXXX)"
  OWN_HOME=1
fi
cleanup() {
  [ "$OWN_WORK" = "1" ] && rm -rf "$WORK_DIR"
  [ "$OWN_HOME" = "1" ] && rm -rf "$ARGO_HOME"
}
trap cleanup EXIT
export ARGO_HOME

fail() { echo "FAIL: $*" >&2; exit 1; }
need() { command -v "$1" >/dev/null 2>&1 || fail "$1 not on PATH"; }
need "$ARGO_BIN"
need jq

banner() { echo; echo "== $* =="; }
json_get() { jq -r "$1"; }

banner "STEP 1 - hire $ROLE_NAME from local pack"
[ -f "$PACK_DIR/pack.toml" ] || fail "missing $PACK_DIR/pack.toml"
[ -f "$PACK_DIR/worker.toml" ] || fail "missing $PACK_DIR/worker.toml"
"$ARGO_BIN" packs dev-link "$PACK_ID" "$PACK_DIR" --json >"$WORK_DIR/dev-link.json"
jq -e . "$WORK_DIR/dev-link.json" >/dev/null
echo "linked pack $PACK_ID"

banner "STEP 2 - first task: $FIRST_TASK"
for skill in "${SKILLS[@]}"; do
  path="$PACK_DIR/skills/$skill.skill.json"
  [ -f "$path" ] || fail "missing skill $path"
  jq -e '.nodes and (.nodes | length > 0)' "$path" >/dev/null
done
FIRST_SKILL="$PACK_DIR/skills/$REFINED_SKILL.skill.json"
NODE_COUNT=$(jq '.nodes | length' "$FIRST_SKILL")
echo "validated $REFINED_SKILL graph with $NODE_COUNT nodes"

banner "STEP 3 - learn reusable screening refinement"
cat >"$WORK_DIR/candidate.json" <<JSON
{"vertical":"$PACK_ID","skill":"resume-evidence-gate","source":"$REFINED_SKILL","pattern":"hold candidates when work authorization or rubric evidence is missing","confidence":0.91}
JSON
[ "$(json_get '.vertical' <"$WORK_DIR/candidate.json")" = "$PACK_ID" ] || fail "candidate vertical mismatch"

banner "STEP 4 - refine and AB-promote"
jq '. + {review:"approved", promoted:true, cohort:"mock-recruiter-demo"}' \
  "$WORK_DIR/candidate.json" >"$WORK_DIR/promoted.json"
[ "$(json_get '.promoted' <"$WORK_DIR/promoted.json")" = "true" ] || fail "promotion flag missing"

banner "RECAP"
echo "OK - $ROLE_NAME demo completed in mock-mode"
echo "pack=$PACK_ID task=\"$FIRST_TASK\" learned_skill=resume-evidence-gate"
