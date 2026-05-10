# Mission load-bearing wire — DutyGate per-duty configurable (OpenGate / ClosedGate / Schedule)

**Owner**: claude. **Depends on**: `DutyGate` trait + `OpenGate`/`ClosedGate` impls SHIPPED (verified at `crates/argo-orchestrator/src/duty_scheduler_gate.rs:53,64`); `DutyScheduler::fire_now` SHIPPED (`duty_scheduler.rs:176`).
**Authored against**: origin/main `b66b1924` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-duty-gate-per-duty-config`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 1 item #45 ("Add DutyGate::OpenGate configurable per-duty"). The gate trait + two impls (`OpenGate` always-fires, `ClosedGate` never-fires) exist; what's missing is **per-duty selection from config** — today every duty silently uses the scheduler's default gate, with no operator knob to set "this specific duty is closed during quiet hours" or "this duty only fires on weekdays."

## Scope

Add `gate: GateConfig` field to the per-duty config struct (verify exact name in `duty_scheduler.rs` first 5 min — likely `Duty` or `DutyConfig`). `GateConfig` is a NEW enum:

```rust
pub enum GateConfig {
    Open,                                          // default; uses OpenGate
    Closed,                                        // disabled; uses ClosedGate
    Schedule { allow_hours: Vec<u8> },             // e.g. [9,10,11,...,17] = business-hours only
    QuietHours { deny_hours: Vec<u8>, tz: String },// e.g. deny [22,23,0,...,7] tz "Australia/Melbourne"
}
```

`DutyScheduler::tick` (line 139) resolves each duty's gate from its config before invoking `should_fire`. The two new variants (`Schedule`, `QuietHours`) are NEW gate impls; `Open`/`Closed` reuse existing structs.

Backward compat: if a duty has no `gate` field set (legacy rows), default to `GateConfig::Open` — preserves today's behavior.

This is NOT in scope: per-tenant gate override; gate that depends on external signal (e.g. PagerDuty status); changing `fire_now` (manual fire bypasses gate by design); changing `should_fire` signature.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-orchestrator/src/duty_scheduler_gate.rs` — narrowly scoped: ADD `pub enum GateConfig`, ADD `pub struct ScheduleGate { allow_hours: Vec<u8> }`, ADD `pub struct QuietHoursGate { deny_hours: Vec<u8>, tz: chrono_tz::Tz }`, ADD impl `DutyGate for ScheduleGate` + `for QuietHoursGate`, ADD `pub fn gate_from_config(cfg: &GateConfig) -> Box<dyn DutyGate>`. Current file ~100 LOC; new growth ~120 LOC; well under 500. NO change to existing `OpenGate`/`ClosedGate`.
- `crates/argo-orchestrator/src/duty_scheduler.rs` — narrowly scoped: extend `Duty` (or `DutyConfig`) struct with `pub gate: GateConfig` (default `GateConfig::Open` via `serde(default)`). Resolve gate in `tick` via `gate_from_config`. NO change to `fire_now`.
- `crates/argo-orchestrator/src/duty_scheduler_gate_tests.rs` (sibling, NEW or append) — 6 unit tests: schedule-allows-in-window; schedule-blocks-outside-window; quiet-hours-blocks-during-deny; quiet-hours-allows-outside-deny; quiet-hours-respects-tz; gate-from-config-dispatches-correctly.
- `crates/argo-orchestrator/Cargo.toml` — narrowly scoped: ADD `chrono-tz` if not present (verify first).

## Files you must NOT touch

- `crates/argo-orchestrator/src/walker.rs` — handoff walker is upstream; unrelated
- `crates/argo-store/migrations/**` — no schema change (duty config rows already accommodate optional fields via JSON; verify in first 5 min)
- Any non-orchestrator crate

## Acceptance criteria

- [ ] `GateConfig` enum with `Open` / `Closed` / `Schedule` / `QuietHours` variants
- [ ] `ScheduleGate { allow_hours }` impls `DutyGate`: fires only when current hour in `allow_hours`
- [ ] `QuietHoursGate { deny_hours, tz }` impls `DutyGate`: blocks fires when current hour (in `tz`) in `deny_hours`
- [ ] `gate_from_config` factory returns the right boxed gate per variant
- [ ] `DutyScheduler::tick` resolves per-duty gate from config
- [ ] Legacy duties (no `gate` field) default to `GateConfig::Open` — today's behavior preserved
- [ ] `fire_now` unchanged (manual override bypasses gate)
- [ ] 6 unit tests cover the matrix
- [ ] No new public API outside the listed exports
- [ ] `cargo build -p argo-orchestrator` clean
- [ ] `cargo clippy -p argo-orchestrator --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-orchestrator` passes (skip pre-existing flakes)
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-duty-gate-per-duty-config`
- PR title: `orchestrator: DutyGate per-duty config (Open/Closed/Schedule/QuietHours) (mission-duty-gate-per-duty-config)`

## Notes for the implementer

- `chrono_tz::Tz` is the canonical tz type — store the tz string in `GateConfig` and parse at gate-construction time. Validation: a bad tz string returns an error at config-load, not at gate-tick.
- `Schedule` and `QuietHours` overlap conceptually — keep both: `Schedule` says "fire IFF in window," `QuietHours` says "fire UNLESS in window." Operators pick the one that matches their mental model. Don't fold them into one variant.
- The hour granularity is intentional — finer granularity (minute, day-of-week) is out of scope; if operators ask for that, file a follow-up brief.
- Memory of recent `mission-quiet-hours-channels` (#1264, in flight) is the **channel-side** quiet hours (NotifyChannel-level); this brief is the **duty-side** quiet hours (DutyScheduler-level). They are independent: a duty firing during quiet hours might still want to silence its NotifyChannel, and vice versa. Don't try to unify them in this brief.
