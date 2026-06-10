# split-pr-2787-isolation-tests-under-80loc

Authored against: origin/main 62765b6cf40a55540cdca653efb03e423ff3d139 on 2026-06-10
**Validate before claim**: bash scripts/check-brief-freshness.sh split-pr-2787-isolation-tests-under-80loc

## Goal

Split the 2 oversized test functions in PR #2787's
`crates/argo-agent-runner/tests/memory_tenant_vertical_isolation_runtime.rs`
under the 80-LOC invariant so #2787 passes the (now-enforced) fn-LOC gate.
#2787's security LOGIC is already adversarially-reviewed CONFIRMED-FIXED — ONLY
the test-length fix is needed. Do NOT touch any assertion or weaken the
isolation proof.

## Why now

The fn-LOC merge gate (added this session, #2782) correctly flags 2 NEW
functions over 80 LOC in #2787:
- `cross_tenant_run_does_not_read_other_tenants_memory` (95 LOC, line ~82)
- `cross_vertical_read_and_write_are_isolated_in_agent_runner_path` (85 LOC, ~184)
Do NOT baseline them (that g`s the existing-debt escape hatch, not for new
functions) — SPLIT them.

## How

Check out #2787's branch (`claude-fix-agent-runtime-memory-tool-tenant-and-vertical-scope`),
rebase on current main. Extract each test's SETUP boilerplate (open+migrate
SessionStore, ensure_workspace(s), the 1100ms sleep, MemoryStore/OrchestratorStore
construction, seeding) into helper fn(s) at the top of the test file; have each
test call the helper and keep ONLY its distinct assertions in the body. Each
function must drop under 80 LOC. Touch NO assertion text/values — the
git diff must show ONLY mechanical setup-extraction. Force-push to #2787's
branch (or open a successor PR that closes #2787).

## Acceptance / verifier

1. `bash scripts/check-fn-loc.sh` exit 0 on the branch (both functions <80 LOC,
   none added to baseline).
2. `cargo test -p argo-agent-runner --test memory_tenant_vertical_isolation_runtime`
   green — both isolation tests still PASS.
3. `git diff` of the assertion lines == empty (only setup moved).

## Size / owner

~20-40 LOC mechanical extraction. Owner: claude. 🟡 P2 — unblocks the
review-CONFIRMED #2787 (memory-tool tenant+vertical isolation, WS1).
