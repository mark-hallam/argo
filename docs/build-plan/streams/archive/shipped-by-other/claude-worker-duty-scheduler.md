# V1.3 — Worker duty scheduler (the autonomous loop)

**Owner**: claude. **Depends on**: E-identity (#812),
H-orchestrator-persist (#806), claude-workflow-dag (#857) merged.

**This is a load-bearing brief.** The mission says workers
"work autonomously and alongside humans". Today they don't —
they only act when an operator hands them a Session. This brief
ships the duty scheduler so workers fire actions on their own
schedule, per their role.

## Scope

A `Role` already has duties (free-text + escalation path). Extend
the schema to make duties **firable** and add a tokio-driven
scheduler that fires them per-worker on cron.

- Extend `Role.duties[]` schema: each duty gets `cron: String`
  (e.g. `0 9 * * 1-5` weekday-mornings), `entry_action: String`
  (handler name in worker action vocabulary), `enabled: bool`,
  `paused_until: Option<DateTime>`.
- New `argo-orchestrator::duty_scheduler` module — tokio interval
  ticking once per minute, walks all active workers + their
  duties, fires `entry_action` for any whose cron matches.
- Each fired duty becomes a Session with `started_by_duty:
  Some(<duty-id>)` so audit trails attribute it correctly.
- New CLI: `argo duty list/show/pause/resume/fire-now`.
- Per-worker `auto_approval_policy` still gates actions — duty
  scheduler doesn't bypass approval.

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- EXHAUSTIVE-MATCH amendment applies
- `crates/argo-identity/src/types.rs` — narrowly: extend Role
  duty struct
- `crates/argo-store/migrations/<next>__role_duties.sql` (NEW)
- `crates/argo-orchestrator/src/duty_scheduler.rs` (NEW)
- `crates/argo-orchestrator/src/lib.rs` — narrowly: re-export +
  spawn scheduler at gateway boot
- `crates/argo-cli/src/handlers/duty_*.rs` (5 NEW)
- `crates/argo-cli/src/cli_duty_cmds.rs` (NEW)
- `crates/argo-cli/src/cli_subcommands.rs` — narrowly: register
  `Duty` subcommand
- `cron` crate as dep on argo-orchestrator/Cargo.toml

## Acceptance criteria
- [ ] Role duty schema persisted with cron + entry_action + enabled
- [ ] Scheduler tick fires matching duties exactly once per cron
      window (idempotency via `last_fired_at` per duty)
- [ ] Fired duty creates Session attributed to `started_by_duty`
- [ ] `argo duty pause <id>` sets `paused_until`; resume clears
- [ ] `argo duty fire-now <id>` triggers immediately ignoring cron
- [ ] AutoApprovalPolicy gate honored
- [ ] 12 unit tests + 2 integration tests (full cron-fire +
      gate cycle)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean

## Branch + PR
- Branch: `claude-worker-duty-scheduler`
- PR title: `orchestrator: worker duty scheduler — autonomous cron-driven actions (claude-worker-duty-scheduler)`
