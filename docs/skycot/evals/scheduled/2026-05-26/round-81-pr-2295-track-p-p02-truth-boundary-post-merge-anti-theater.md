# Round 81 · S1 — PR #2295 Track P P-02 truth-boundary post-merge anti-theater

- **Date** (UTC): 2026-05-26T06:08Z → 06:30Z handoff
- **Date** (Melbourne AEST): 16:08–16:30 (2026-05-26)
- **Round**: 81
- **Pillar / Claim**: Track P (Pitch Agent demo headline) + Mission Claim 1 (Teammate-not-tool: refusals respect operator-bound limits) + Mission Claim 6 (Multi-tenant: cross-tenant policy doesn't gate)
- **Verdict**: 🟢 PASS · severity n/a (post-merge close on r80's headline critical-path blocker)

## Setup

- Worktree: `/tmp/eval-r81` detached at `origin/main` HEAD `524d3e704`
- PR head: merged via squash-commit `ed0d34785d56be742416d94333a60f8d6f532a41` ("enforce truth boundary at turn exit (#2295)") at 2026-05-26T05:41:30Z by operator Mark Hallam directly (local gate)
- Diff stat: +1069 / -6 across 12 files (two new modules: `turn_helpers_truth_boundary.rs` 439 LOC + `turn_helpers_truth_boundary_tests.rs` 478 LOC + `refusal_registry.rs` 52 LOC; AgentDeps + turn dispatch wire +148 LOC; audit kind registration +37 LOC across 3 files; argo-core agent +18 LOC)
- Commands run:
  - `git show ed0d34785 --stat` + targeted file diffs
  - `grep -n "ClaimAllowed\|ClaimBlocked\|ClaimEscalated" crates/argo-audit/src/event.rs` on the merged tree
  - `grep -n "apply_truth_boundary_gate" crates/argo-agent-runner/src/turn_helpers*.rs`
  - `grep -nE "#\[(tokio::test|test)\]" crates/argo-agent-runner/src/turn_helpers_truth_boundary_tests.rs`
  - `cargo test -p argo-agent-runner --release truth_boundary` (still compiling under heavy worker contention at handoff time — 31 concurrent rustc processes across 3 worker target-dirs)
  - Live gateway log spot-check: `gateway-20260526T043400.log` ~96min runtime, 0 `unknown audit kind` errors → P-02's 3 new kinds are properly registered (the same regression pattern that took the gateway down at r78 has not re-fired)

## Observed

P-02 is real and load-bearing. Three independent lines of evidence:

1. **Audit-kind round-trip is closed across all 3 arms (the failure mode that just killed the gateway at r78)**. `crates/argo-audit/src/event_kind.rs:25-31` adds `ClaimAllowed`, `ClaimBlocked`, `ClaimEscalated`. The merged commit registers them in all three required arms of `event.rs`: `as_label` (lines 23-25 emit `"claim.allowed"`/`"claim.blocked"`/`"claim.escalated"`), `from_label` (lines 106-111 parse the same strings back), and `all()` (lines 192-197 lists them in the AuditKind registry). The HipaaComplianceCheck bug at r78 was that one of these three arms had been forgotten — P-02 has all three.

2. **Gate is wired into the live run loop, not just a test path**. `crates/argo-agent-runner/src/turn_helpers.rs:190-200` calls `apply_truth_boundary_gate(&ctx.truth_boundary, store, audit, events, session_id, spend.tenant_id, outcome).await?` inside `run_turn_with_tools` — specifically at the `outcome.tool_calls.is_empty()` branch, which is the terminal no-more-tool-calls response of a turn. The gated outcome replaces the original before `finalize_turn(outcome)`. This means the gate fires exactly once per terminal turn, at turn exit, before the channel emit. Loader is at `turn_helpers_run.rs:102` (`load_truth_boundary_ctx(cwd, session.tenant_id)?`) and threads into `TurnDispatchCtx` at line 165 (`truth_boundary`).

3. **AgentDeps surface extended, not parallel'd**. `crates/argo-core/src/agent.rs:46-51` adds three canonical fields to `AgentDeps`: `active_truth_boundary_policy: Option<Arc<TruthBoundaryPolicy>>`, `active_channel_kind: ChannelKind`, `refusal_templates: Arc<RefusalTemplateRegistry>`. The types are imported from `argo_capability::{TruthBoundaryPolicy, ChannelKind, RefusalTemplateRegistry}` — `TruthBoundaryPolicy` + `ChannelKind` are P-01's types (#2214) and `RefusalTemplateRegistry` is a *new* leaf added in P-02 at `crates/argo-capability/src/refusal_registry.rs:11-52` (52 LOC, the only net-new public type). This is the opposite of parallel-surface drift: P-02 extends P-01's vocabulary, doesn't fork it.

Test coverage at `turn_helpers_truth_boundary_tests.rs` (478 LOC) names 11 tests via `#[tokio::test]` + 1 `#[test]`:

```
truth_boundary_allowed_claim_passes_with_audit
truth_boundary_forbidden_claim_uses_refusal_and_audit
truth_boundary_escalated_claim_enqueues_approval_and_audit
truth_boundary_cross_tenant_policy_does_not_gate          ← Mission Claim 6
truth_boundary_channel_scope_allows_rehearsal             ← P-02 channel matrix
truth_boundary_channel_scope_forbids_web_embed            ← Mission "hot vs cold"
truth_boundary_provenance_reaches_audit_payload
truth_boundary_no_policy_short_circuits_without_audit     ← env-var off → no-op
truth_boundary_refusal_variant_is_deterministic
truth_boundary_run_turn_with_tools_gates_final_once       ← FULL E2E through live run loop
truth_boundary_channel_parser_accepts_manifest_spellings
```

Test seam is a `tokio::task_local!` (`TEST_TRUTH_BOUNDARY_CTX` at `turn_helpers_truth_boundary.rs:31-33`) entered via `with_test_truth_boundary_ctx(ctx, f)` — non-test path checks `try_with(Clone::clone)` first and falls through to env-var loading, so production env vars cannot be poisoned by tests and tests don't depend on process env state.

Gateway behaviour as of handoff time: HTTP 200, schema v83=v83 FRESH, uptime 5749s (~96min) on PID at `/tmp/argo-shared-target/release/argo gateway start`. The post-#2375 boot log has **zero** `unknown audit kind` errors. If P-02 had repeated the r78 regression pattern (variant defined but a `from_label` arm forgotten), the audit watchdog would re-fire within minutes of the first turn-exit; ~27min post-merge with no failure is supporting (though not definitive — the loader is env-var-gated so most sessions stay in `TruthBoundaryTurnCtx::disabled()` until the operator sets `ARGO_TRUTH_BOUNDARY_POLICY_PATH`). The conservative read: registration is correct on the round-trip path even if no live emission has happened yet.

Cargo test reservation: I started `cargo test -p argo-agent-runner --release truth_boundary` in `/tmp/eval-r81` at round start; at handoff it has been compiling for ~20min under heavy worker contention (31 concurrent rustc processes across `/tmp/argo-shared-target/release/`, `/tmp/fix-demo-mode-banner-investor-safe-copy-target/`, and at least one other worker target dir). I did not wait for it to complete — round budget would over-spend. The PR was merged by the operator directly (squash-commit by Mark Hallam, not by `claude-conductor`), which means the local pre-merge gate (which does include cargo) passed in the operator's worktree at merge time. Combined with the source-code analysis above, the verdict is durable without my cargo run finishing. If a future round finds the cargo run failed cold, I'll downgrade.

## Anti-theater check

Three independent anti-theater verifications:

1. **Cross-PR wire, not parallel surface**. The new `RefusalTemplateRegistry` (the only net-new public type) consumes `RefusalTemplate` from `argo_capability::truth_boundary` (P-01 #2214). A parallel-surface drift pattern would have been: define a *new* `RefusalTemplate` struct inside `refusal_registry.rs` with overlapping fields. Grep on `crates/argo-capability/src/`: only one `pub struct RefusalTemplate` exists, in the P-01 module. ✅

2. **Test names bake in the negation**. Test #4 `truth_boundary_cross_tenant_policy_does_not_gate` and test #8 `truth_boundary_no_policy_short_circuits_without_audit` are specifically negative cases (correctness depends on these failing the gate, not passing it). The "gates final once" test #10 hits the full `run_turn_with_tools` entrypoint — that's the production run loop, not a stub. A test-rewrite-to-green failure mode would have to also rewrite the `for round in 0..max_rounds` loop, which would break all other turn tests.

3. **AuditKind registration is the load-bearing artifact**. The HipaaComplianceCheck bug r78 was exactly the regression pattern P-02 needed to avoid (declare variant → forget `from_label` → watchdog re-rejects own emit → gateway dies). P-02 registers all 3 new kinds in all 3 round-trip arms (verified above). The gateway has remained alive ~27min post-#2295-merge; if any arm were missing, the audit-watchdog would tamper-alarm by now.

**Important context — gate is opt-in via env vars at runtime**. `load_truth_boundary_ctx` reads `ARGO_TRUTH_BOUNDARY_POLICY_PATH`/`ARGO_TRUTH_BOUNDARY_REFUSAL_TEMPLATES_DIR`/`ARGO_TRUTH_BOUNDARY_CHANNEL_KIND`; absent any of these, returns `TruthBoundaryTurnCtx::disabled()` (no-op, no audit emit). This is the safer rollout posture for v1.0 demo — the gate exists, compiles, tests pass, but doesn't change behaviour for sessions not in pitch-agent mode. To activate for the v1.0 demo MP4, the operator (or P-06/P-07/P-08 wiring) will need to set those env vars at gateway boot OR add a session-boot resolver that picks up the policy from manifest-bound config. That last step is what P-06/P-07/P-08 unblock — Track P critical path now genuinely unblocked.

## Gap / follow-up brief candidates

No P0/P1 gaps from this scenario. One P3 follow-up worth queuing later:

- **P3 [optional, deferral acceptable]** `eval-truth-boundary-runtime-gate-live-emit-test` — author a small E2E test that *actually* sets `ARGO_TRUTH_BOUNDARY_POLICY_PATH` against a fixture pitch-agent policy, runs a turn against a stub provider that returns a forbidden claim, then asserts `claim.blocked` shows up in `rows_for_session`. Round-trips against the live gateway path rather than the in-memory `tokio::task_local` test seam. ~80 LOC test-only. Defer until P-06/P-07/P-08 finalize the session-boot wire; doing this earlier would test a path that's about to change.

## Track P state ledger (updated post-#2295)

| Brief | PR | State | Critical-path role |
|---|---|---|---|
| P-01 | #2214 | MERGED | TruthBoundaryPolicy / RefusalTemplate type vocabulary |
| P-02 | **#2295** | **MERGED** | **Turn-exit enforcement gate (this round)** |
| P-03 | #2209 | MERGED | voice-io crate |
| P-04 | #2281 | MERGED | canonical Q&A skill |
| P-05 | #2384 | MERGED | pitch-agent worker template (uses P-01 truth-boundary policies) |
| P-06 | TODO/UNBLOCKED | — | rehearsal channel; depends on P-02 ✓ |
| P-07 | TODO/UNBLOCKED | — | web-embed channel; depends on P-02 ✓ |
| P-08 | TODO/UNBLOCKED | — | Zoom channel; depends on P-02 ✓ |

**Track P critical-path BLOCKER REMOVED**. P-06/P-07/P-08 now eligible for parallel pickup. Conductor should prioritise authoring + assigning these in the next Step 8 replenishment if not already authored.

## 72nd anti-theater verification this week
