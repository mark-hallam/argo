# Claude — Orchestrator handoff walker (escalation autonomy)

**Owner**: claude. **Depends on**: H-orchestrator-persist
merged.

## Scope

The H-3 / H-orchestrator-persist landing brought
`Session`, `Handoff`, and `EscalationChain` to the platform.
What's missing: a runtime that actually walks the chain
when a handoff times out.

Today: a worker emits a Handoff with `deadline = now + 1h`.
After 1h, the row's `is_overdue(now)` returns true — but
nothing flips it to `Expired` or escalates to the next
chain step.

This brief adds the walker — a periodic sweeper that:
1. Lists handoffs where status is `Pending` and
   `deadline` has passed.
2. For each: marks `Expired` (per `Handoff::expire`),
   then constructs a NEW handoff to the next
   `EscalationStep` in the chain (looked up via
   `EscalationChain::advance`).
3. If the chain is exhausted, emits an
   `escalation_exhausted` event + notifies the worker's
   default channel + flips the parent Session to
   `Expired`.

The sweeper runs as a `tokio::task` on the gateway
process. Sweep interval default 60s, configurable via
`ARGO_HANDOFF_SWEEP_SECS` env.

## Files you may touch
- `crates/argo-orchestrator/src/walker.rs` (NEW)
- `crates/argo-orchestrator/src/lib.rs` — narrowly:
  declare module + re-export `HandoffWalker`
- `crates/argo-cli/src/handlers/gateway/sweepers.rs` (NEW
  — the tokio task wrapper that runs the walker on a
  cadence)
- `crates/argo-cli/src/handlers/gateway/mod.rs` — narrowly:
  spawn the sweeper alongside the existing routes
- `crates/argo-cli/Cargo.toml` — narrowly: add
  `argo-orchestrator` path dep if not already there

## Files you must NOT touch
- `crates/argo-orchestrator/src/{session,handoff,escalation}.rs`
  — fixed types
- The persist API — read-only consumer
- Any new migration

## Acceptance criteria
- [ ] `HandoffWalker::sweep_once(store, now)` advances every
      overdue Pending handoff per the rules above
- [ ] Chain exhaustion writes an audit row labelled
      `escalation_exhausted` with the original session_id
- [ ] When the chain has a `Human(addr)` step, the walker
      calls the appropriate `NotifyChannel` (look up
      channel by address scheme — `mailto:` → email,
      `slack:` → slack, etc — fall back to email if
      ambiguous)
- [ ] Tokio sweeper task spawned at gateway startup
- [ ] Sweep interval respects `ARGO_HANDOFF_SWEEP_SECS`
      env (default 60s)
- [ ] Tests:
      - overdue handoff with worker step ahead → new
        handoff created, original Expired
      - overdue handoff with human step ahead → channel
        notified, original Expired
      - chain exhausted → audit row + session Expired
      - non-overdue handoff → no change
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean
- [ ] Every authored file < 500 LOC

## Branch + PR
- Branch: `claude-orchestrator-handoff-walker`
- PR title: `orchestrator: periodic walker that escalates overdue handoffs (claude-orchestrator-handoff-walker)`

## Notes
- The sweeper must be idempotent — a slow sweep that runs
  twice in quick succession should NOT double-emit
  handoffs. Each step's status flip is the synchronization
  point.
- For the audit row, use the existing audit crate's
  `record_event` (mirror what the H-2 gate does).
- This is the brief that makes the system genuinely
  AUTONOMOUS — without the walker, escalation chains are
  static documentation. With it, work flows through
  agents + humans without manual nudge.
