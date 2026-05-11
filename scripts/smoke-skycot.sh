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

heading "13. doctor reports healthy state"
doctor_out=$("$ARGO_BIN" worker doctor --vertical gtm-engineer --json 2>&1) || fail "doctor" "$doctor_out"
echo "$doctor_out" | grep -qE '"id"[[:space:]]*:[[:space:]]*"gtm-engineer"' || fail "doctor missing gtm-engineer in verticals" "$doctor_out"
echo "$doctor_out" | grep -qE '"completed"[[:space:]]*:[[:space:]]*1' || fail "doctor expected 1 completed action" "$doctor_out"
echo "$doctor_out" | grep -qE '"blocked"[[:space:]]*:[[:space:]]*1' || fail "doctor expected 1 blocked action" "$doctor_out"
ok "doctor reports verticals + counts as expected"

heading "14. all shipped action-emitting verticals lint clean"
# Each Worker v3 reference vertical (#729 → #746) ships with
# its own [actions] declarations + instruction templates.
# Catch template-key typos / malformed manifests before the
# customer hits them at runtime.
for vid in gtm-engineer ai-receptionist sales-follow-up; do
  lint_out=$("$ARGO_BIN" vertical lint "$vid" 2>&1) || fail "lint $vid" "$lint_out"
  echo "$lint_out" | grep -q "lint clean" || fail "$vid not lint-clean" "$lint_out"
done
ok "all 3 Worker v3 verticals lint clean"

heading "15. all shipped action-emitting verticals have valid eval suites"
# Cohort-wide structural lock for the eval fixtures landed in
# #759. A future "rip out fixtures" change has to either fix the
# manifest or trip this check.
for vid in gtm-engineer ai-receptionist sales-follow-up; do
  eval_out=$("$ARGO_BIN" vertical eval "$vid" --json 2>&1) || fail "eval $vid" "$eval_out"
  echo "$eval_out" | grep -qE '"ok"[[:space:]]*:[[:space:]]*true' || fail "$vid eval reported errors" "$eval_out"
  total=$(echo "$eval_out" | grep -oE '"fixture_count":[0-9]+' | awk -F: '{s += $2} END {print s+0}')
  [[ "$total" -ge 5 ]] || fail "$vid: expected ≥5 eval fixtures, got $total" "$eval_out"
done
ok "all 3 Worker v3 verticals carry structurally valid eval suites"

heading "16. argo setup --check reports FAIL with no provider keys"
# Locks the launch-checklist contract from #761: with NO
# provider env vars exported, --check must exit non-zero and
# emit a [FAIL] marker on stderr. The launch checklist relies
# on this to surface a missing-key gap loudly.
check_no_keys=$(env -u ANTHROPIC_API_KEY -u OPENAI_API_KEY -u GEMINI_API_KEY \
  -u AWS_ACCESS_KEY_ID -u ELEVENLABS_API_KEY \
  "$ARGO_BIN" setup --check 2>&1)
check_no_keys_exit=$?
[[ $check_no_keys_exit -ne 0 ]] || fail "setup --check should fail with no provider keys" "$check_no_keys"
echo "$check_no_keys" | grep -q "\[FAIL\]" || fail "setup --check stderr missing [FAIL] marker" "$check_no_keys"
ok "setup --check FAIL path verified"

heading "17. argo setup --check reports OK with at least one provider key"
# Inverse of #16: with ANTHROPIC_API_KEY set (any non-empty
# value), --check must exit 0 and emit [ok]. Locks the
# happy-path contract.
check_with_key=$(env -u OPENAI_API_KEY -u GEMINI_API_KEY \
  -u AWS_ACCESS_KEY_ID -u ELEVENLABS_API_KEY \
  ANTHROPIC_API_KEY="sk-ant-smoke-only-not-real" \
  "$ARGO_BIN" setup --check 2>&1)
check_with_key_exit=$?
[[ $check_with_key_exit -eq 0 ]] || fail "setup --check should pass with ANTHROPIC_API_KEY set" "$check_with_key"
echo "$check_with_key" | grep -q "\[ok\]" || fail "setup --check stdout missing [ok] marker" "$check_with_key"
echo "$check_with_key" | grep -q "ANTHROPIC_API_KEY" || fail "setup --check stdout missing detected key name" "$check_with_key"
ok "setup --check OK path verified"

heading "18. argo setup --check --require enforces strict-pair contract"
# Locks the launch-checklist invariant from #764: with both
# ANTHROPIC + OPENAI required but only ANTHROPIC set, --check
# must exit non-zero AND name OPENAI_API_KEY in the stderr so
# the operator can fix it. Then with both set, must exit 0.
strict_one_only=$(env -u OPENAI_API_KEY -u GEMINI_API_KEY \
  -u AWS_ACCESS_KEY_ID -u ELEVENLABS_API_KEY \
  ANTHROPIC_API_KEY="sk-ant-smoke" \
  "$ARGO_BIN" setup --check \
    --require ANTHROPIC_API_KEY \
    --require OPENAI_API_KEY 2>&1)
strict_one_only_exit=$?
[[ $strict_one_only_exit -ne 0 ]] || fail "strict-pair must fail when fallback missing" "$strict_one_only"
echo "$strict_one_only" | grep -q "OPENAI_API_KEY" || fail "strict-pair FAIL must name missing var" "$strict_one_only"
strict_both=$(env -u GEMINI_API_KEY -u AWS_ACCESS_KEY_ID -u ELEVENLABS_API_KEY \
  ANTHROPIC_API_KEY="sk-ant-smoke" OPENAI_API_KEY="sk-smoke" \
  "$ARGO_BIN" setup --check \
    --require ANTHROPIC_API_KEY \
    --require OPENAI_API_KEY 2>&1)
strict_both_exit=$?
[[ $strict_both_exit -eq 0 ]] || fail "strict-pair must pass when both set" "$strict_both"
echo "$strict_both" | grep -q "all required keys loadable" || fail "strict-pair OK must confirm loadable" "$strict_both"
ok "setup --check --require strict-pair contract verified"

heading "19. BrowserExecutor smoke"
# Drives the existing integration smoke
# `crates/argo-cli/tests/browser_action_executor_smoke.rs`, which
# spawns `python3 tests/fixtures/browser_mock_mcp.py` as a stub
# MCP server. Skip-with-warn when python3 is absent so operators on
# minimal images still get a green smoke run for the other 19
# checks; the BrowserExecutor wiring is locked elsewhere in CI.
if ! command -v python3 >/dev/null 2>&1; then
  echo "  SKIP #19 BrowserExecutor — python3 not in PATH"
else
  cd "$SCRIPT_DIR/.." || fail "cd to repo root for cargo test" ""
  bx_out=$(cargo test -p argo-cli --test browser_action_executor_smoke --quiet 2>&1) \
    || fail "browser_action_executor_smoke" "$bx_out"
  cd "$WORKDIR" || fail "cd back to workdir" ""
  ok "browser_action_executor_smoke passed"
fi

heading "20. Slack approval roundtrip"
# Drives the existing integration smoke
# `crates/argo-cli/tests/slack_approval_roundtrip_e2e.rs`. Pure-Rust
# HMAC verification — no external dependencies, no skip path.
cd "$SCRIPT_DIR/.." || fail "cd to repo root for cargo test" ""
slack_out=$(cargo test -p argo-cli --test slack_approval_roundtrip_e2e --quiet 2>&1) \
  || fail "slack_approval_roundtrip_e2e" "$slack_out"
cd "$WORKDIR" || fail "cd back to workdir" ""
ok "slack_approval_roundtrip_e2e passed"

echo
echo "✓ SKYCOT smoke test PASSED"
echo "  workspace: $WORKDIR"
echo "  argo home: $HOME_DIR"
echo "  job_id:    $job_id"
echo
echo "Next step: read docs/skycot/getting_started.md to run a real"
echo "(non-mock) job against your provider of choice."
