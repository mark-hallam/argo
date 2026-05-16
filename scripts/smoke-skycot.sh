#!/usr/bin/env bash
# SKYCOT end-to-end smoke test.
#
# Runs in mock-mode (no API key required) against a fresh workspace
# in a tempdir, drives every state transition the operator surface
# exposes, and asserts the artifact bundle landed correctly.
#
# Usage:
#   ./scripts/smoke-skycot.sh [argo-binary] [--target-url <URL>]
#
# Pass --target-url <URL> to additionally run 8 post-deploy scenarios
# (#31-#38) against a remote SKYCOT instance — Fly app, staging, etc.
# Default (no flag): local-only scenarios run, behavior unchanged.
#
# Internal cargo smoke scenarios inherit CARGO_TARGET_DIR. If unset,
# the script exports the repo-local target/ directory before running
# cargo so builds and tests use the same cache.
#
# Exits non-zero on the first failure with a clear "FAIL: <reason>"
# message + the offending command + its stderr. Operators run this
# right after install to confirm SKYCOT is wired end-to-end.

set -uo pipefail

# Parse --target-url out of $@ before consuming positional arg 1.
TARGET_URL=""
POSITIONAL=()
while (( "$#" > 0 )); do
  case "$1" in
    --target-url)
      if [[ -z "${2:-}" ]]; then
        echo "FAIL: --target-url requires a URL argument"
        exit 2
      fi
      TARGET_URL="$2"
      shift 2
      ;;
    --target-url=*)
      TARGET_URL="${1#--target-url=}"
      shift
      ;;
    *)
      POSITIONAL+=("$1")
      shift
      ;;
  esac
done
if (( ${#POSITIONAL[@]} > 0 )); then
  set -- "${POSITIONAL[@]}"
else
  set --
fi

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
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
VERTICALS_DIR="$REPO_ROOT/verticals"
if [[ ! -d "$VERTICALS_DIR/gtm-engineer" ]]; then
  echo "FAIL: gtm-engineer vertical not found at $VERTICALS_DIR"
  echo "      this script must run from the Argo repo (or have"
  echo "      ARGO_VERTICALS_DIR set to a directory containing"
  echo "      gtm-engineer/)"
  exit 2
fi
export ARGO_HOME="$HOME_DIR"
export ARGO_VERTICALS_DIR="$VERTICALS_DIR"
if [[ -z "${CARGO_TARGET_DIR:-}" ]]; then
  CARGO_TARGET_DIR="$REPO_ROOT/target"
fi
export CARGO_TARGET_DIR

cd "$WORKDIR" || {
  echo "FAIL: cd to workdir $WORKDIR"
  exit 1
}

# Scenario-result counters. Mission assertions count as PASS; compatibility
# checks that runtime-detect they can't exercise the mission path count as
# SKIP (a WARN signal — "compatibility check, not mission proof"). Failures
# still exit immediately via fail().
PASSED_COUNT=0
SKIPPED_COUNT=0
WARNED_COUNT=0
SKIPPED_REASONS=()
WARNED_REASONS=()

ok() {
  PASSED_COUNT=$((PASSED_COUNT + 1))
  echo "  OK: $*"
}
skip() {
  SKIPPED_COUNT=$((SKIPPED_COUNT + 1))
  SKIPPED_REASONS+=("$*")
  echo "  SKIP: $*"
}
warn() {
  WARNED_COUNT=$((WARNED_COUNT + 1))
  WARNED_REASONS+=("$*")
  echo "  WARN: $*"
}
fail() {
  echo "FAIL: $1"
  if [[ -n "${2:-}" ]]; then
    echo "      output:"
    while IFS= read -r line; do
      echo "        $line"
    done <<< "$2"
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

heading "14. canonical action-emitting vertical lint clean"
# The canonical Worker v3 reference vertical (#729 → #732)
# ships with its own [actions] declarations + instruction templates.
# Catch template-key typos / malformed manifests before the
# customer hits them at runtime.
vid="gtm-engineer"
lint_out=$("$ARGO_BIN" vertical lint "$vid" 2>&1) || fail "lint $vid" "$lint_out"
echo "$lint_out" | grep -q "lint clean" || fail "$vid not lint-clean" "$lint_out"
ok "canonical Worker v3 vertical lint clean"

heading "15. all shipped action-emitting verticals have valid eval suites"
# Cohort-wide structural lock for the eval fixtures landed in
# #759. A future "rip out fixtures" change has to either fix the
# manifest or trip this check.
for vid in gtm-engineer sales-follow-up; do
  eval_out=$("$ARGO_BIN" vertical eval "$vid" --json 2>&1) || fail "eval $vid" "$eval_out"
  echo "$eval_out" | grep -qE '"ok"[[:space:]]*:[[:space:]]*true' || fail "$vid eval reported errors" "$eval_out"
  total=$(echo "$eval_out" | grep -oE '"fixture_count":[0-9]+' | awk -F: '{s += $2} END {print s+0}')
  [[ "$total" -ge 5 ]] || fail "$vid: expected ≥5 eval fixtures, got $total" "$eval_out"
done
ok "all Worker v3 verticals carry structurally valid eval suites"

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
echo "  TARGET_DIR=$CARGO_TARGET_DIR" >&2
# Drives the existing integration smoke
# `crates/argo-cli/tests/browser_action_executor_smoke.rs`, which
# spawns `python3 tests/fixtures/browser_mock_mcp.py` as a stub
# MCP server. Skip-with-warn when python3 is absent so operators on
# minimal images still get a green smoke run for the remaining checks;
# the BrowserExecutor wiring is locked elsewhere in CI.
if ! command -v python3 >/dev/null 2>&1; then
  skip "BrowserExecutor — python3 not in PATH (compatibility check, not mission proof)"
else
  cd "$REPO_ROOT" || fail "cd to repo root for cargo test" ""
  bx_out=$(cargo test -p argo-cli --test browser_action_executor_smoke --quiet 2>&1) \
    || fail "browser_action_executor_smoke" "$bx_out"
  cd "$WORKDIR" || fail "cd back to workdir" ""
  ok "browser_action_executor_smoke passed"
fi

heading "20. macOS computer-use smoke (proves V1 success scenario #1 when ARGO_NATIVE_MACOS_INTEGRATION=1)"
# Runs the v1-primary native computer-use smoke. On non-macOS runners
# or when ARGO_NATIVE_MACOS_INTEGRATION is unset, the integration test
# prints a skip reason and still exercises config validation — we treat
# that path as SKIP (compatibility check) so the operator can distinguish
# it from a real mission proof.
cd "$REPO_ROOT" || fail "cd to repo root for cargo test" ""
macos_cu_out=$(cargo test -p argo-executor-computer-use-macos --test notes_smoke --quiet -- --nocapture 2>&1) \
  || fail "macOS computer-use notes_smoke" "$macos_cu_out"
cd "$WORKDIR" || fail "cd back to workdir" ""
if echo "$macos_cu_out" | grep -q "skipping native macOS smoke test"; then
  skip "macOS computer-use notes_smoke — native integration not exercised (set ARGO_NATIVE_MACOS_INTEGRATION=1 on macOS to prove V1 scenario #1)"
else
  ok "macOS computer-use notes_smoke passed (native integration exercised)"
fi

heading "21. Slack approval roundtrip"
# Drives the existing integration smoke
# `crates/argo-cli/tests/slack_approval_roundtrip_e2e.rs`. Pure-Rust
# HMAC verification — no external dependencies, no skip path.
cd "$REPO_ROOT" || fail "cd to repo root for cargo test" ""
slack_out=$(cargo test -p argo-cli --test slack_approval_roundtrip_e2e --quiet 2>&1) \
  || fail "slack_approval_roundtrip_e2e" "$slack_out"
cd "$WORKDIR" || fail "cd back to workdir" ""
ok "slack_approval_roundtrip_e2e passed"

heading "22. Auto-approval multi-window fallback smoke"
cd "$REPO_ROOT" || fail "cd to repo root for cargo test" ""
multiwindow_out=$(cargo test -p argo-cli --test auto_approval_multiwindow_e2e --quiet 2>&1) \
  || fail "auto_approval_multiwindow_e2e" "$multiwindow_out"
cd "$WORKDIR" || fail "cd back to workdir" ""
ok "auto_approval_multiwindow_e2e passed"

heading "23. Multi-worker handoff memory smoke"
cd "$REPO_ROOT" || fail "cd to repo root for cargo test" ""
handoff_out=$(cargo test -p argo-orchestrator --test multiworker_handoff_e2e_smoke --quiet 2>&1) \
  || fail "multiworker_handoff_e2e_smoke" "$handoff_out"
cd "$WORKDIR" || fail "cd back to workdir" ""
ok "multiworker_handoff_e2e_smoke passed"

heading "24. Cross-tenant skill discovery negative smoke"
cd "$REPO_ROOT" || fail "cd to repo root for cargo test" ""
cross_tenant_out=$(cargo test -p argo-cli --test cross_tenant_skill_discovery_negative --quiet 2>&1) \
  || fail "cross_tenant_skill_discovery_negative" "$cross_tenant_out"
cd "$WORKDIR" || fail "cd back to workdir" ""
ok "cross_tenant_skill_discovery_negative passed"

heading "25. Claim-4 skill marketplace loop smoke"
cd "$REPO_ROOT" || fail "cd to repo root for cargo test" ""
claim4_out=$(cargo test -p argo-skill --test end_to_end_claim4_loop --quiet 2>&1) \
  || fail "end_to_end_claim4_loop" "$claim4_out"
cd "$WORKDIR" || fail "cd back to workdir" ""
ok "end_to_end_claim4_loop passed"

heading "26. Memory tool namespace runtime (proves Claim 1 memory_namespace through agent dispatch loop, not just prompt hydration)"
# F-03: Memory::with_namespace must be reachable from the runtime dispatch
# path, not just the prompt-hydration helper. The dedicated integration
# crate covers bind / cross-worker isolation / unbound-error paths.
cd "$REPO_ROOT" || fail "cd to repo root for cargo test" ""
mem_ns_out=$(cargo test -p argo-agent-runner --test memory_namespace_runtime_tests --quiet 2>&1) \
  || fail "memory_namespace_runtime_tests" "$mem_ns_out"
cd "$WORKDIR" || fail "cd back to workdir" ""
ok "memory_namespace_runtime_tests passed"

heading "27. Per-worker learning cron runtime (proves V1 success scenario #2 — gateway boot spawns the cron)"
# F-04: gateway boot must spawn per_worker_learning_cron for active workers
# under [skill_learning]; the integration test exercises the spawned-cron
# path (rating rows propose a worker overlay without a direct tick call).
cd "$REPO_ROOT" || fail "cd to repo root for cargo test" ""
cron_rt_out=$(cargo test -p argo-skill --test per_worker_learning_cron_runtime --quiet 2>&1) \
  || fail "per_worker_learning_cron_runtime" "$cron_rt_out"
cd "$WORKDIR" || fail "cd back to workdir" ""
ok "per_worker_learning_cron_runtime passed"

heading "28. Policy-as-code enforcement (proves Claim 1 auto_approval_policy + V1 success scenario #4)"
# F-06: real tenant/role/roster resolution + fail-closed enforcement of
# require_signatures, quiet_hours, and audit_chain_required. The integration
# test is authored under codex-policy-as-code-enforcement; until that brief
# lands we surface this as a SKIP so the operator sees "compatibility gap,"
# not a false PASS.
policy_test_path="$REPO_ROOT/crates/argo-cli/tests/policy_as_code_enforcement_integration.rs"
if [[ ! -f "$policy_test_path" ]]; then
  skip "policy_as_code_enforcement_integration — F-06 test not yet landed (codex-policy-as-code-enforcement); scenario auto-activates once present"
else
  cd "$REPO_ROOT" || fail "cd to repo root for cargo test" ""
  policy_out=$(cargo test -p argo-cli --test policy_as_code_enforcement_integration --quiet 2>&1) \
    || fail "policy_as_code_enforcement_integration" "$policy_out"
  cd "$WORKDIR" || fail "cd back to workdir" ""
  ok "policy_as_code_enforcement_integration passed"
fi

heading "29. Notify fallback risk filter (proves Claim 6 risk-appropriate routing in fallback dispatch)"
# F-05: matches_risk must be applied in the fallback dispatch path so a
# same-kind-different-risk fixture routes correctly. The handler-internal
# unit tests live under crates/argo-cli/src/handlers/worker_actions_notify_fallback_tests.rs
# and are run via the worker_actions_notify_fallback filter.
cd "$REPO_ROOT" || fail "cd to repo root for cargo test" ""
fallback_out=$(cargo test -p argo-cli --bin argo worker_actions_notify_fallback --quiet 2>&1) \
  || fail "worker_actions_notify_fallback unit tests" "$fallback_out"
cd "$WORKDIR" || fail "cd back to workdir" ""
ok "worker_actions_notify_fallback unit tests passed"

heading "30. Slack approval round-trip with stub server (proves v1.0 SHIP-GATE Slack-approval scenarios)"
# Drives the Rust-native `slack_approval_smoke` integration test.
# Two cargo scenarios run under one invocation:
#   * outbound — a real SlackApprovalChannel POSTs Block Kit to an
#     in-process stub URL with signed approve/reject buttons;
#   * inbound — a freshly-booted `argo gateway start` accepts the v0
#     signed callback at `/channels/slack/webhook`; settle reaches
#     the missing-action terminator; `audit verify-chain --json`
#     emits a parseable summary afterwards.
# SKIP path lets minimal images opt out of the cargo+gateway compose
# while keeping the slack_approval_roundtrip_e2e check (scenario 21)
# as the mandatory channel-seam mission proof.
if [[ "${ARGO_SKIP_SLACK_SMOKE:-}" == "1" ]]; then
  skip "slack_approval_smoke — ARGO_SKIP_SLACK_SMOKE=1 set (compatibility opt-out, not mission proof)"
else
  cd "$REPO_ROOT" || fail "cd to repo root for cargo test" ""
  slack_smoke_out=$(cargo test -p argo-cli --test slack_approval_smoke --quiet 2>&1) \
    || fail "slack_approval_smoke" "$slack_smoke_out"
  cd "$WORKDIR" || fail "cd back to workdir" ""
  ok "slack_approval_smoke passed (outbound stub + inbound gateway round-trip)"
fi

# ---------------------------------------------------------------------
# Deployed-target scenarios (31-38). Only run when --target-url is set.
# Each scenario must be runnable without the local argo binary; they
# exercise the deployed HTTP surface (gateway) via curl + openssl.
# ---------------------------------------------------------------------
if [[ -n "$TARGET_URL" ]]; then
  # Strip trailing slash so $TARGET_URL/$path joins cleanly.
  TARGET_URL="${TARGET_URL%/}"
  if [[ ! "$TARGET_URL" =~ ^https?:// ]]; then
    fail "--target-url must start with http:// or https:// (got '$TARGET_URL')"
  fi
  if ! command -v curl >/dev/null 2>&1; then
    fail "--target-url requires curl on PATH"
  fi

  heading "31. health-deployed — GET /healthz"
  resp=$(curl -fsS --max-time 10 "$TARGET_URL/healthz" 2>&1) \
    || fail "GET $TARGET_URL/healthz" "$resp"
  echo "$resp" | grep -q '"ok"[[:space:]]*:[[:space:]]*true' \
    || fail "/healthz body missing \"ok\":true" "$resp"
  ok "/healthz returned 200 with ok:true"

  heading "32. meta-deployed — GET /meta"
  resp=$(curl -fsS --max-time 10 "$TARGET_URL/meta" 2>&1) \
    || fail "GET $TARGET_URL/meta" "$resp"
  echo "$resp" | grep -q '"version"' \
    || fail "/meta body missing version field" "$resp"
  ok "/meta returned 200 with version field"

  heading "33. meta-tools-deployed — GET /meta/tools"
  resp=$(curl -fsS --max-time 10 "$TARGET_URL/meta/tools" 2>&1) \
    || fail "GET $TARGET_URL/meta/tools" "$resp"
  tool_count=$(echo "$resp" | grep -oE '"name"[[:space:]]*:' | wc -l | tr -d ' ')
  if (( tool_count < 8 )); then
    fail "/meta/tools advertised $tool_count tools (expected ≥8)" "$resp"
  fi
  ok "/meta/tools returned 200 with $tool_count tools"

  heading "34. auth-signup-rate-limit — POST × 11 /api/v1/auth/signup"
  signup_path="/api/v1/auth/signup"
  rate_429=""
  for i in $(seq 1 11); do
    email="test-rl-${RANDOM}-${i}@smoketest.local"
    code=$(curl -sS -o /dev/null -w '%{http_code}' --max-time 10 \
      -X POST -H 'Content-Type: application/json' \
      -d "{\"email\":\"$email\"}" \
      "$TARGET_URL$signup_path" 2>&1)
    if [[ "$i" == "11" && "$code" == "429" ]]; then
      rate_429="yes"
    fi
  done
  if [[ "$rate_429" == "yes" ]]; then
    ok "11th signup correctly rate-limited (429)"
  else
    warn "rate-limit not observed on 11th request — gateway may not enforce burst limits yet (informational)"
  fi

  heading "35. auth-signup-happy — POST /api/v1/auth/signup"
  email_happy="test-happy-${RANDOM}@smoketest.local"
  resp=$(curl -sS --max-time 10 \
    -X POST -H 'Content-Type: application/json' \
    -d "{\"email\":\"$email_happy\"}" \
    "$TARGET_URL$signup_path" 2>&1) \
    || fail "POST $signup_path" "$resp"
  if echo "$resp" | grep -q '"status"[[:space:]]*:[[:space:]]*"sent"'; then
    ok "happy-path signup returned status:sent (throwaway $email_happy)"
  else
    warn "happy-path signup did not return status:sent — response was: $resp"
  fi

  heading "36. cors-preflight — OPTIONS /api/v1/dispatch"
  headers=$(curl -sS -i --max-time 10 \
    -X OPTIONS \
    -H 'Origin: https://app.skycot.com' \
    -H 'Access-Control-Request-Method: POST' \
    -H 'Access-Control-Request-Headers: content-type' \
    "$TARGET_URL/api/v1/dispatch" 2>&1) \
    || fail "OPTIONS /api/v1/dispatch" "$headers"
  if echo "$headers" | grep -qi '^access-control-allow-origin:'; then
    ok "CORS preflight returned Access-Control-Allow-Origin header"
  else
    fail "OPTIONS /api/v1/dispatch missing Access-Control-Allow-Origin" "$headers"
  fi

  heading "37. tls-cert — openssl handshake + expiry"
  if [[ "$TARGET_URL" =~ ^https:// ]]; then
    if ! command -v openssl >/dev/null 2>&1; then
      skip "tls-cert — openssl not on PATH (compatibility check, not mission proof)"
    else
      host="${TARGET_URL#https://}"
      host="${host%%/*}"
      port="${host##*:}"
      if [[ "$port" == "$host" ]]; then port="443"; fi
      host="${host%%:*}"
      cert_dates=$(openssl s_client -servername "$host" -connect "$host:$port" \
          -showcerts < /dev/null 2>/dev/null \
          | openssl x509 -noout -dates 2>&1) \
        || fail "openssl handshake to $host:$port failed" "$cert_dates"
      not_after=$(echo "$cert_dates" | grep '^notAfter=' | sed 's/^notAfter=//')
      if [[ -z "$not_after" ]]; then
        fail "could not parse certificate notAfter for $host" "$cert_dates"
      fi
      ok "TLS handshake succeeded for $host; cert expires $not_after"
    fi
  else
    skip "tls-cert — target is plain http:// (compatibility check, not mission proof)"
  fi

  heading "38. latency-budget — 100×GET /healthz (parallel)"
  lat_dir=$(mktemp -d)
  for i in $(seq 1 100); do
    (
      curl -sS -o /dev/null --max-time 10 \
        -w '%{time_total}\n' "$TARGET_URL/healthz" \
        > "$lat_dir/$i" 2>/dev/null
    ) &
  done
  wait
  samples=$(cat "$lat_dir"/* 2>/dev/null | grep -E '^[0-9]+\.?[0-9]*$' | sort -n)
  rm -rf "$lat_dir"
  n=$(echo "$samples" | wc -l | tr -d ' ')
  if (( n < 50 )); then
    fail "latency-budget collected only $n/100 samples — target unreachable?" ""
  fi
  pick_pct() {
    # $1 = percentile (50/95/99). Uses awk to read sorted lines.
    echo "$samples" | awk -v p="$1" -v n="$n" '
      BEGIN { idx = int((p/100) * n); if (idx < 1) idx = 1 }
      NR == idx { print; exit }'
  }
  p50=$(pick_pct 50)
  p95=$(pick_pct 95)
  p99=$(pick_pct 99)
  echo "  samples=$n p50=${p50}s p95=${p95}s p99=${p99}s"
  # Convert p95 to milliseconds via awk (portable; no bc dependency).
  p95_ms=$(awk -v v="$p95" 'BEGIN { printf "%.0f", v * 1000 }')
  if (( p95_ms > 2000 )); then
    fail "latency budget breached: p95=${p95_ms}ms > 2000ms hard ceiling" ""
  elif (( p95_ms > 500 )); then
    warn "latency yellow flag: p95=${p95_ms}ms > 500ms target (network-dependent, not a deploy block)"
  else
    ok "latency p95=${p95_ms}ms within 500ms target"
  fi
fi

echo
echo "✓ SKYCOT smoke test PASSED"
echo "  scenarios: $PASSED_COUNT passed, $SKIPPED_COUNT skipped, $WARNED_COUNT warned"
if (( SKIPPED_COUNT > 0 )); then
  echo "  skipped scenarios (compatibility checks, not mission proofs):"
  for reason in "${SKIPPED_REASONS[@]}"; do
    echo "    - $reason"
  done
fi
if (( WARNED_COUNT > 0 )); then
  echo "  warnings (informational, not failures):"
  for reason in "${WARNED_REASONS[@]}"; do
    echo "    - $reason"
  done
fi
echo "  workspace: $WORKDIR"
echo "  argo home: $HOME_DIR"
echo "  job_id:    $job_id"
if [[ -n "$TARGET_URL" ]]; then
  echo "  target:    $TARGET_URL"
fi
echo
echo "Next step: read docs/skycot/getting_started.md to run a real"
echo "(non-mock) job against your provider of choice."
