# Claude — End-to-end integration test (the v1 demo gate)

**Owner**: claude. **Depends on**: ALL of A, B, D-email,
E-identity, F-memory-layering, G-genesis, H-auto-approval
merged (currently TRUE). Also benefits from H-orchestrator-
persist + H-spend-persistence merging but does NOT block on
them.

## Scope

A single integration test that exercises the entire SKYCOT
stack end-to-end. This is the **v1 demo gate** — when this
test passes on a fresh checkout with no env keys, the
platform is genuinely ready for a customer-launch demo.

Test flow:
1. Genesis-output: programmatically generate a vertical via
   `argo_genesis::pipeline::write_vertical("be a marketplace
   seller for inline skates")` into a temp dir.
2. Manifest validation: `argo_vertical::load_dir(...)` round-
   trips it cleanly.
3. Worker creation: `IdentityStore::create_worker(...)` from
   E-identity.
4. Action emission: emit a `verify_listing_price` action
   (low-risk, requires_approval=true) via the V3 worker
   ledger.
5. Auto-approval gate: with policy
   `enabled=true, max_usd_micros_per_action=500000`, assert
   the H-2 hookup writes an `approved_by="auto:..."` audit
   row.
6. Channel notify: with a STUB
   `EmailChannel::disabled_for_tests()`, assert the channel
   was queried but no real email was sent.
7. Action transition: assert the action's state is
   `executed` (HumanExecutor stub) and a `result.json`
   bundle was written to the workspace.
8. Memory layering: write a memory entry under the worker's
   `MemoryNs`, query it back, assert it round-trips.
9. Audit trail: assert at least 4 audit rows exist
   (action_emitted, auto_approved, channel_queried, executed).

The test passes WITHOUT any of these: ANTHROPIC_API_KEY,
E2B_API_KEY, OPENAI_API_KEY, gmail/slack/etc tokens. Every
LLM-touching path is stubbed.

## Files you may touch
- `crates/argo-cli/tests/e2e_v1_demo.rs` (NEW)
- `crates/argo-cli/tests/e2e_helpers/mod.rs` (NEW — small
  helper module with stub builders)

## Files you must NOT touch
- Any source crate (this is a test-only brief)
- Any existing test file

## Acceptance criteria
- [ ] `cargo test -p argo-cli --test e2e_v1_demo` passes on
      a fresh checkout with no env keys
- [ ] Test takes < 10 seconds wall-clock (uses in-memory
      sqlite + stubbed providers)
- [ ] 9 numbered flow steps each have at least 1 assertion
- [ ] Test failures emit clear messages identifying which
      step failed
- [ ] CI-friendly: no flaky time-based assertions, no
      sleep-based polling
- [ ] `cargo clippy -p argo-cli --tests -- -D warnings`
      clean
- [ ] Every authored file < 500 LOC

## Branch + PR
- Branch: `claude-e2e-integration-test`
- PR title: `tests: end-to-end v1 demo integration (claude-e2e-integration-test)`

## Notes
- This test is the customer-demo readiness signal. If it
  passes, the platform really does what MISSION.md
  promises.
- Stubbed providers: study the existing test patterns in
  `crates/argo-channels/src/email.rs` for the
  `disabled_for_tests` shape; mirror it for any other
  channel the test path touches.
- If a flow step turns out to be unimplementable without
  major plumbing changes, mark that step `// TODO:` with a
  comment identifying which brief unblocks it. Do NOT fake
  the assertion. The test is allowed to skip 1-2 steps
  with a TODO; more than that means we have a real v1 gap
  to surface.
