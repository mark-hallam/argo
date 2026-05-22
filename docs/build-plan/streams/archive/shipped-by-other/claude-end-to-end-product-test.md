# V1.4 — End-to-end product test

**Owner**: claude. **Depends on**: claude-onboarding-flow,
codex-setup-wizard-cli (#854), G-genesis (#790) merged.

## Scope

The existing `claude-e2e-integration-test` (#810/#832) covers
the platform's internal flow (action emit → gate → channel →
executor → audit). It does NOT test the operator's experience:
"I install SKYCOT, run `argo onboard`, end up with a working
worker doing real work."

Ship an operator-perspective integration test:

1. Fresh tempdir as `ARGO_HOME`
2. Run `argo onboard --non-interactive` against a test
   `onboarding.toml` (Anthropic key from env, no other
   credentials, mock connector for OAuth)
3. Verify workspace + credentials.toml were created
4. Verify a worker was spawned via genesis
5. Hand the worker a sample task (script-injected action emit)
6. Verify the action goes through the auto-approval gate
7. Verify the worker completes the task (mock executor)
8. Verify audit trail is intact + ends with action.completed
9. Cleanup tempdir

This is THE smoke test that asserts SKYCOT is a working product,
not just a toolbox of primitives.

## Files you may touch
- `crates/argo-cli/tests/product_e2e_smoke.rs` (NEW — the
  big integration test)
- `crates/argo-cli/tests/fixtures/onboarding-min.toml` (NEW —
  test config)
- `crates/argo-cli/tests/fixtures/sample-task.json` (NEW)
- `crates/argo-cli/tests/common/product_e2e_helpers.rs` (NEW)
- BLANKET CLI ALLOWLIST applies for the cli touches if any

## Acceptance criteria
- [ ] Full 9-step flow passes end-to-end against a local mock
      connector
- [ ] Test runs in CI (no real API keys required; uses fixture
      Anthropic-mock OR skips with clear log if `ANTHROPIC_API_KEY`
      unset)
- [ ] On failure, test prints which step failed + recent audit
      records to aid debugging
- [ ] Cleanup is idempotent (test cleanup runs even if assertion
      panics)
- [ ] Total runtime under 60s (excluding any LLM calls)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean

## Branch + PR
- Branch: `claude-end-to-end-product-test`
- PR title: `e2e: operator-perspective product integration test (claude-end-to-end-product-test)`
