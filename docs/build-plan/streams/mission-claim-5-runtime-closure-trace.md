# mission-claim-5-runtime-closure-trace — prove audit-chain → replay end-to-end (v1.0 launch closure)

**Owner**: claude. **Sizing hint**: integration-test brief; consume existing API surface only.
**Depends on**: existing `argo-audit::verify_session` + `argo-replay::Replayer` + `argo-cli` session-emit path (all MERGED, individually tested).
**Authored against**: origin/main `bc3841a4` on `2026-05-18`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-claim-5-runtime-closure-trace`.
**Source**: 2026-05-18 audit P1 finding #2 — claim 5 (hash-chained audit + replay) is PARTIAL because no E2E test exercises the full session→chain→replay path. Operator confirmed v1.0 SHIP-GATE (Decision #2 "d" → before launch).

## Mission alignment

- **🎯 Mission claim 5** — the launch claim is "every action lives on a tamper-evident hash chain that anyone can replay." Three components individually pass; without an E2E lock, a silent regression in any one of them (or in their composition) lands without a test failure.
- **Reliability** (CLAUDE.md priority #1) — regression-lock on a load-bearing differentiator.
- **Observability** (#2) — replay is the operator's primary forensic tool; broken replay = blind operator.

## Scope

P0 v1.0 SHIP-GATE. Integration test in `crates/argo-cli/tests/audit_chain_replay_smoke.rs` that drives the full session → chain → replay pipeline end-to-end and asserts trajectory equivalence.

### Test flow

1. **Spawn an isolated session.** Create a tenant + worker via substrate APIs; emit a deterministic sequence of `ToolDispatch` + `ToolResult` + `ApprovalAsked` + `ApprovalReplied` events.
2. **Assert chain emission.** After session close, query the audit table; verify every row's `chain_hash` correctly chains back to the previous row's hash.
3. **Run `argo audit verify-chain --session <id>`.** Assert exit-0 + "chain valid" message. Also call `verify_session` directly via the Rust API to catch any CLI/library skew.
4. **Run `argo replay <session> --output <tmp-trajectory.json>`.** Assert exit-0.
5. **Assert trajectory equivalence.** Compare the replayed trajectory to the original event sequence: same actions, same order, same payloads (modulo timestamps + chain hashes).
6. **Tamper-evidence negative case.** Hand-corrupt one audit row's `payload_json`. Re-run `verify-chain` — must report tampering at the corrupted row's offset.
7. **Cross-tenant negative case.** Replay attempt across tenant boundary fails with clear error (verify the audit chain is tenant-scoped).

### Audit trail assertions

After step 5, the audit chain should contain the full lifecycle for the test session. The corrupted-row test (step 6) writes its mutation to an isolated test session — never to the shared chain.

LOC: ~300.

## Files you may touch

- `crates/argo-cli/tests/audit_chain_replay_smoke.rs` (NEW) — main test. ≤250 LOC.
- `crates/argo-cli/tests/fixtures/audit_chain_seed.rs` (NEW) — deterministic event sequence generator. ≤60 LOC.
- `docs/skycot/operator/audit-chain-forensics.md` (NEW or extend) — runbook documenting the full flow + how to debug when verify-chain reports tampering. ≤100 LOC.

## Files you must NOT touch

- Production audit chain emit / verify code (`argo-audit/src/`) — pure read-only consumer.
- The `argo replay` CLI surface (`argo-cli/src/handlers/replay*.rs`) — read-only consumer.
- The `verify_session` Rust API — read-only consumer.
- Audit chain hash algorithm — read-only consumer.

## Acceptance criteria

- [ ] Test runs end-to-end via `cargo test -p argo-cli --test audit_chain_replay_smoke -- --nocapture`.
- [ ] Test takes <30 seconds on a typical dev laptop.
- [ ] All 7 waypoints (or however many the worker structures) assertable + asserted.
- [ ] Tamper-evidence negative case: corrupted row detected at the right offset.
- [ ] Cross-tenant negative case: replay across tenant boundary fails clearly.
- [ ] Operator runbook documents: how to run, what each waypoint proves, debugging if any waypoint fails.
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `mission-claim-5-runtime-closure-trace`
- Title: `test(claim-5): audit-chain → replay closure smoke — session-emit → verify-chain → replay → tamper-evidence (mission-claim-5-runtime-closure-trace)`
- Worker verification: crate-scoped clippy + integration test + cross-platform smoke per SWARM_PROTOCOL.md.

## Hidden constraints

- **No new production code.** Pure consumer of existing audit + replay APIs. If a gap surfaces, declare as discovered drift in the PR body (per #1822 precedent).
- **Deterministic event sequence.** The test must not depend on wall-clock timestamps; either freeze time via a test clock or assert structural equivalence ignoring timestamps.
- **Tamper-evidence test runs in an isolated session.** Never corrupt rows the test framework itself uses.
- **Cleanup teardown:** delete the test tenant + all derived audit/skill rows after the test. Idempotent re-runs.
- **Closes audit P1 finding #2.** Claim 5 graduates from PARTIAL to PROVEN when this lands.
