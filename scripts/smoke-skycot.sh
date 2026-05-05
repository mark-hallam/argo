#!/usr/bin/env bash
# SKYCOT end-to-end smoke test.
#
# Runs in mock-mode (no API key required) against a fresh workspace
# in a tempdir, drives every state transition the operator surface
# exposes, and asserts the artifact bundle landed correctly.
#
# Usage:
#   ./scripts/smoke-skycot.sh [argo-binary]
#
# Exits non-zero on the first failure with a clear "FAIL: <reason>"
# message + the offending command + its stderr. Operators run this
# right after install to confirm SKYCOT is wired end-to-end.

set -uo pipefail

ARGO_BIN="${1:-argo}"
SCRIPT_NAME="$(basename "$0")"

if ! command -v "$ARGO_BIN" >/dev/null 2>&1; then
  echo "FAIL: argo binary '$ARGO_BIN' not on PATH"
  echo "      install via the project README; pass the path as arg 1"
  echo "      e.g. $SCRIPT_NAME ~/.local/bin/argo"
  exit 2
fi

WORKDIR="$(mktemp -d)"
HOME_DIR="$(mktemp -d)"
trap 'rm -rf "$WORKDIR" "$HOME_DIR"' EXIT

# Resolve the verticals dir relative to this script so the test
# uses the in-tree gtm-engineer manifest.
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
VERTICALS_DIR="$SCRIPT_DIR/../verticals"
if [[ ! -d "$VERTICALS_DIR/gtm-engineer" ]]; then
  echo "FAIL: gtm-engineer vertical not found at $VERTICALS_DIR"
  echo "      this script must run from the Argo repo (or have"
  echo "      ARGO_VERTICALS_DIR set to a directory containing"
  echo "      gtm-engineer/)"
  exit 2
fi
export ARGO_HOME="$HOME_DIR"
export ARGO_VERTICALS_DIR="$VERTICALS_DIR"

cd "$WORKDIR"

ok() { echo "  OK: $*"; }
fail() {
  echo "FAIL: $1"
  if [[ -n "${2:-}" ]]; then
    echo "      output:"
    echo "$2" | sed 's/^/        /'
  fi
  exit 1
}
heading() { echo; echo "=== $* ==="; }

heading "1. workspace init"
out=$("$ARGO_BIN" workspace init 2>&1) || fail "workspace init" "$out"
ok "workspace initialised under $WORKDIR"

heading "2. vertical loaded"
out=$("$ARGO_BIN" vertical list 2>&1) || fail "vertical list" "$out"
echo "$out" | grep -q "gtm-engineer" || fail "gtm-engineer vertical missing" "$out"
ok "gtm-engineer loaded"

heading "3. queue mock job"
add_out=$("$ARGO_BIN" work add \
  --vertical gtm-engineer \
  --prompt "smoke test — verify a price for skates" \
  --json 2>&1) || fail "work add" "$add_out"
ok "job queued"

heading "4. run worker once (mock)"
serve_out=$("$ARGO_BIN" work serve \
  --vertical gtm-engineer \
  --once \
  --max-steps 1 \
  --json 2>&1) || fail "work serve" "$serve_out"
job_id=$(echo "$serve_out" | grep -oE '"job_id":"[^"]+"' | head -1 | sed 's/.*"\(.*\)"/\1/')
[[ -n "$job_id" ]] || fail "job_id missing from serve output" "$serve_out"
ok "worker ran job $job_id"

heading "5. artifact bundle layout"
bundle="$WORKDIR/.skycot/outputs/$job_id"
for f in transcript.md assistant_output.md research.md pricing.md \
         listings.md approval_checklist.md execution_plan.md \
         actions.json job.json result.json; do
  [[ -f "$bundle/$f" ]] || fail "bundle missing $f" "$(ls -la "$bundle" 2>&1)"
done
ok "all 10 v1 bundle files present"

heading "6. action ledger has 2 rows (mock)"
list_out=$("$ARGO_BIN" worker actions list \
  --state all \
  --vertical gtm-engineer \
  --json 2>&1) || fail "actions list" "$list_out"
row_count=$(echo "$list_out" | grep -c '"action_id"')
[[ "$row_count" == "2" ]] || fail "expected 2 ledger rows, got $row_count" "$list_out"
verify_id=$(echo "$list_out" | grep '"action_type":"verify_price"' | grep -oE '"action_id":"[^"]+"' | sed 's/.*"\(.*\)"/\1/')
post_id=$(echo "$list_out" | grep '"action_type":"post_listing"' | grep -oE '"action_id":"[^"]+"' | sed 's/.*"\(.*\)"/\1/')
[[ -n "$verify_id" ]] || fail "verify_price row missing" "$list_out"
[[ -n "$post_id" ]] || fail "post_listing row missing" "$list_out"
ok "verify_price ($verify_id) + post_listing ($post_id) landed"

heading "7. approval gate enforced (post_listing blocked under mock evidence)"
post_state=$(echo "$list_out" | grep '"action_type":"post_listing"' | grep -oE '"state":"[^"]+"' | sed 's/.*"\(.*\)"/\1/')
[[ "$post_state" == "blocked" ]] || fail "post_listing should be blocked under mock evidence; got '$post_state'" "$list_out"
ok "post_listing correctly downgraded to blocked"

heading "8. approval refused for autonomous-safe action"
approve_out=$("$ARGO_BIN" worker actions approve "$verify_id" 2>&1)
approve_exit=$?
[[ $approve_exit -ne 0 ]] || fail "approve should refuse autonomous-safe verify_price" "$approve_out"
echo "$approve_out" | grep -q "autonomous-safe" || fail "approve error didn't mention 'autonomous-safe'" "$approve_out"
ok "approve correctly refused"

heading "9. execute → bundle written"
exec_out=$("$ARGO_BIN" worker actions execute "$verify_id" --json 2>&1) || fail "execute" "$exec_out"
echo "$exec_out" | grep -qE '"state"[[:space:]]*:[[:space:]]*"executing"' || fail "execute did not return state=executing" "$exec_out"
bundle_path=$(echo "$exec_out" | grep -oE '"execution_bundle_path"[[:space:]]*:[[:space:]]*"[^"]+"' | sed 's/.*"\([^"]*\)"$/\1/')
[[ -n "$bundle_path" ]] || fail "execute did not return execution_bundle_path" "$exec_out"
[[ -f "$bundle_path/instructions.md" ]] || fail "instructions.md missing in bundle" "$(ls -la "$bundle_path" 2>&1)"
[[ -f "$bundle_path/evidence.md" ]] || fail "evidence.md missing in bundle"
[[ -f "$bundle_path/action.json" ]] || fail "action.json missing in bundle"
grep -q "Verify price" "$bundle_path/instructions.md" || fail "verify_price template did not render" "$(cat "$bundle_path/instructions.md")"
ok "per-action bundle written + template rendered"

heading "10. complete → state advances"
complete_out=$("$ARGO_BIN" worker actions complete "$verify_id" \
  --note "smoke pass" \
  --result-json '{"verdict":"within-band"}' \
  --json 2>&1) || fail "complete" "$complete_out"
echo "$complete_out" | grep -qE '"state"[[:space:]]*:[[:space:]]*"completed"' || fail "complete did not return state=completed" "$complete_out"
ok "complete advanced state"

heading "11. status emits full transition history"
status_out=$("$ARGO_BIN" worker actions status "$verify_id" --json 2>&1) || fail "status" "$status_out"
state_lines=$(echo "$status_out" | grep -c '"action_id"')
[[ "$state_lines" -ge 3 ]] || fail "expected ≥3 state-rows, got $state_lines" "$status_out"
echo "$status_out" | grep -q '"state":"pending"' || fail "history missing pending row"
echo "$status_out" | grep -q '"state":"executing"' || fail "history missing executing row"
echo "$status_out" | grep -q '"state":"completed"' || fail "history missing completed row"
ok "history shows pending → executing → completed"

heading "12. render writes workspace ledger"
render_out=$("$ARGO_BIN" worker actions render --vertical gtm-engineer 2>&1) || fail "render" "$render_out"
for f in approval_queue.md completed_actions.md blocked_actions.md; do
  [[ -f "$WORKDIR/.skycot/$f" ]] || fail ".skycot/$f missing" "$(ls -la "$WORKDIR/.skycot" 2>&1)"
done
grep -q "post_listing" "$WORKDIR/.skycot/blocked_actions.md" \
  || fail "blocked_actions.md missing post_listing entry" "$(cat "$WORKDIR/.skycot/blocked_actions.md")"
grep -q "verify_price" "$WORKDIR/.skycot/completed_actions.md" \
  || fail "completed_actions.md missing verify_price entry" "$(cat "$WORKDIR/.skycot/completed_actions.md")"
ok "3 ledger files rendered, expected entries present"

echo
echo "✓ SKYCOT smoke test PASSED"
echo "  workspace: $WORKDIR"
echo "  argo home: $HOME_DIR"
echo "  job_id:    $job_id"
echo
echo "Next step: read docs/skycot/getting_started.md to run a real"
echo "(non-mock) job against your provider of choice."
