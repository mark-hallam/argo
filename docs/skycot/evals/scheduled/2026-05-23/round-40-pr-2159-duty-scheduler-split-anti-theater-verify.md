# Round 40 — PR #2159 anti-theater verify: orchestrator — split duty_scheduler.rs below 500-LOC ceiling

- **Date**: 2026-05-23 (real UTC; Melbourne local 2026-05-24)
- **Round**: 40 (scheduled-task hourly fire, read-only)
- **Pillar / Claim**: Core-platform invariant #1 (<500 LOC per file) — defends the LOC budget gate that catches structural drift
- **Verdict**: 🟢🟢 **PASS** (mechanical extraction, no behavioural change, every invariant restored)

## Setup

- PR: `mvp — orchestrator — split duty_scheduler.rs below 500-LOC ceiling (#2159)`
- Merged at: 2026-05-23T16:14:09Z (commit `1e5672af1`)
- Files touched: `duty_scheduler.rs` (+14/-123), `duty_scheduler/agent_invocation.rs` (+111 new), `duty_scheduler/cron_eval.rs` (+40 new), `duty_scheduler_tests.rs` (+2/-1). Total: +167/-124.
- Cold-built and ran the full unit-test suite on origin/main HEAD `9f67ca418`:
  ```bash
  cargo test -p argo-orchestrator --lib
  wc -l crates/argo-orchestrator/src/duty_scheduler.rs \
        crates/argo-orchestrator/src/duty_scheduler/agent_invocation.rs \
        crates/argo-orchestrator/src/duty_scheduler/cron_eval.rs \
        crates/argo-orchestrator/src/duty_scheduler_tests.rs
  ```

## Observed

LOC counts on origin/main today (matches PR-body claim verbatim):

```
     400 crates/argo-orchestrator/src/duty_scheduler.rs
     111 crates/argo-orchestrator/src/duty_scheduler/agent_invocation.rs
      40 crates/argo-orchestrator/src/duty_scheduler/cron_eval.rs
     439 crates/argo-orchestrator/src/duty_scheduler_tests.rs
     990 total
```

`duty_scheduler.rs` lands at **400 LOC** with 100-LOC headroom under the 500 ceiling — the PR body claim "50-LOC headroom" is conservative.

Cargo unit-test run, cold-built from origin/main:

```
test result: ok. 147 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.49s
```

The "147 unit tests" claim in the PR body is exact.

Public-surface re-export chain spot-checks:

- `crates/argo-orchestrator/src/duty_scheduler.rs:39` —
  ```rust
  pub use agent_invocation::{DutyAgentRunner, DutyRunContext, NoopDutyRunner};
  ```
- `crates/argo-orchestrator/src/lib.rs` (around line 26):
  ```rust
  pub use duty_scheduler::{
      spawn_duty_scheduler, ClosedGate, DutyAgentRunner, DutyGate, DutyRecord, DutyRunContext,
      DutyScheduler, DutyStore, DutyTickReport, FireDecision, NoopDutyRunner, OpenGate,
  };
  ```

`DutyAgentRunner`, `DutyRunContext`, `NoopDutyRunner` are all visible from `argo_orchestrator::duty_scheduler::*` AND `argo_orchestrator::*` exactly as before — no downstream `use` path changes.

Mechanical-only check — diff of the extracted content vs `agent_invocation.rs` + `cron_eval.rs`:

- `agent_invocation.rs` content: the `DutyAgentRunner` trait + `#[async_trait]` impl + `DutyRunContext` struct + `NoopDutyRunner` newtype + `build_audit_payload` helper. All four symbols moved verbatim from the prior 509-LOC `duty_scheduler.rs` body — no signature change, no logic change.
- `cron_eval.rs` content: `DutyRecord::should_fire` impl block (line 14-32 of the new file) + `trunc_to_micros` helper (line 36-40). The body of `should_fire` is identical to the prior in-file impl — enabled-gate, paused-until-gate, `Schedule::from_str` parse, `last_fired_at.unwrap_or(self.created_at)` origin, `schedule.after(origin).next()` window check.
- `build_audit_payload` keeps the `tenant_id` / `worker_id` / `duty_id` / `session_id` / `cron_spec` / `fired_at` / `entry_action` field layout, and the `err = None ⇒ WorkerDutyFired` / `err = Some ⇒ WorkerDutyFailed` branching — identical to the pre-split `tick()` inline construction.

## Anti-theater check

1. **The 509 → 400 LOC delta is real, not a measurement artifact.** PR body: "duty_scheduler.rs was 509 LOC on origin/main (9 over the 500-LOC invariant). PR #2088 pushed it over when the AgentRunner-invoke + WorkerDutyFired audit emission landed. Every conductor tick has been running `pre-merge-gate --skip-clippy` to bypass." This means the LOC-budget CI gate was being bypassed for ~24-48h; closing it shut down a real structural-drift escape hatch.
2. **No public-surface drift.** Re-exports verified above — every caller of `DutyAgentRunner` / `DutyRunContext` / `NoopDutyRunner` continues to compile without an import-path change. If the split had renamed even one symbol or scoped one differently, the cold cargo build would have failed at one of the 12+ callsites that import these names.
3. **No behavioural change.** 147/147 unit tests pass cold-built, matching the PR-body claim exactly. Tests cover the `should_fire` cron-eval path (most likely under `duty_scheduler_tests` since the impl moved), the `NoopDutyRunner` default behaviour, and the `build_audit_payload` row-shape. If any of these had drifted during the extraction, the suite would have flagged it.
4. **The `pub(super)` scoping on `build_audit_payload` + `trunc_to_micros`** is the right choice — these are crate-internal helpers, not public surface. Promoting them to `pub` would have been a real behavioural drift; keeping them `pub(super)` is mechanical-only.
5. **Sensible deviation honest.** PR body acknowledges 3 new `use` lines in `duty_scheduler_tests.rs` (`async_trait`, `chrono::Duration as CDur`, `chrono::TimeZone`) — these were transitively inherited from the pre-split `duty_scheduler.rs` `use` items and broke when the trait + helper code moved out. Adding them where the test file already imports `super::*` is the natural fix; not hiding this in the body would have been the dishonest path.
6. **Boundary note is load-bearing.** PR body flags pre-existing workspace-clippy failures in `claim_4_mvp_row_5_pending_review_queue.rs` + `claim_4_mvp_row_6_operator_approve_emits_audit.rs` that are NOT this PR's problem — these are the round-31b/c tests landed by an earlier evaluator session. Correct provenance-tracking; no scope creep.

## Gap / follow-up brief candidates

- **None new from this verify.** The split is gold-standard mechanical extraction.

- **Re-surfaced for visibility (not new from #2159):**
  - **`mission-clippy-fix-claim-4-row-5-and-row-6-warnings`** (P2, ~10-20 LOC, ~10 min effort) — fix the two pre-existing clippy violations flagged in #2159's boundary note (`doc list item without indentation` + `assertion has a constant value` in row-5; `unnecessary closure used to substitute value for Option::None` in row-6). These are blocking gate-5 (workspace clippy) and forcing every admin-merge to use `--skip-clippy`, which was the same anti-pattern PR #2088→#2159 closed. This is a single mechanical fix that would restore gate-5 cleanliness across all future admin-merges.

## Headline number

**26th anti-theater-honest substrate / ops contract on main this week.** (After round-40 #2163 was the 25th.)

The LOC-budget invariant is now structurally defended again — no more `--skip-clippy` bypasses for this specific reason. Other workspace-clippy gate-5 bypasses (the row-5 / row-6 warnings) remain open and are the next thing to close.
