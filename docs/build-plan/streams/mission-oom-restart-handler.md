# Mission load-bearing wire — OOM-kill detection + graceful worker restart

**Owner**: claude. **Depends on**: existing `crates/argo-executor-computer-use/src/restart.rs` (canonical retry-strategy pattern); existing argo-runtime worker lifecycle (`crates/argo-runtime/src/worker.rs` or equivalent — verify at claim time).
**Authored against**: origin/main `20240927` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-oom-restart-handler`.

> SOURCE: MISSION_BACKLOG.md Section 8 Claim 8 #237 ("OOM kill detection + graceful restart"). **Tier P (reliability).** Today: when a worker subprocess dies from an OOM kill (Linux OOM killer = SIGKILL/exit code 137; macOS = signal 9; cgroup memory-cap breach), the supervisor sees a generic "process exited with status X" and the brief work is lost. Operators get no actionable signal — no audit row, no metric, no automatic relaunch. This brief adds typed OOM detection + a bounded exponential-backoff restart loop + an audit-chain row recording the kill + restart attempts.

## Scope

NEW `crates/argo-runtime/src/oom_restart.rs` (NEW, ≤ 240 LOC):

- `pub enum WorkerExitReason { Clean { code: i32 }, OomKilled { signal: i32, cgroup_evidence: Option<String> }, OtherSignal { signal: i32 }, Hung { last_heartbeat: DateTime<Utc> } }` — typed classification of a worker subprocess exit.
- `pub fn classify_exit(status: std::process::ExitStatus, last_heartbeat: Option<DateTime<Utc>>, hung_threshold: Duration) -> WorkerExitReason` — derives the typed reason:
  - Exit code 137 OR signal 9 with no clean teardown → `OomKilled`. Linux: try to read `/sys/fs/cgroup/<worker_cgroup>/memory.events` for `oom_kill` count to confirm + populate `cgroup_evidence`.
  - Exit code 0 → `Clean`.
  - Other signals (SIGTERM = clean shutdown handler, SIGSEGV = crash, etc.) → `OtherSignal`.
  - Process still alive but `last_heartbeat > hung_threshold` ago → `Hung`.
- `pub struct OomRestartConfig { max_restarts: u32, base_backoff: Duration, max_backoff: Duration, restart_window: Duration }` — defaults `max_restarts=3` per `restart_window=15min`, `base_backoff=2s`, `max_backoff=2min`.
- `pub struct OomRestartCoordinator { config: OomRestartConfig, restart_history: VecDeque<(DateTime<Utc>, WorkerExitReason)> }` — tracks restart attempts in a rolling window.
- `pub fn decide_restart(coord: &mut OomRestartCoordinator, exit: &WorkerExitReason) -> RestartDecision` — returns `Restart { after_backoff: Duration }`, `GiveUp { reason: String }`, or `IgnoreCleanExit`. Exponential backoff doubles each attempt; clamped to `max_backoff`. After `max_restarts` within `restart_window`, returns `GiveUp` with a fix-recipe message.
- `pub async fn emit_audit_row(deps: &AgentDeps, worker_id: &str, exit: &WorkerExitReason, decision: &RestartDecision) -> ArgoResult<()>` — appends to audit chain with `kind = "worker_oom_restart"` and JSON body `{worker_id, exit_reason, decision, attempt_count}`.

NEW `crates/argo-runtime/src/oom_restart_tests.rs` (NEW, ≤ 180 LOC, included via `#[path]`) — 8 unit tests: classify-exit-137-as-OomKilled, classify-clean-exit-as-Clean, classify-sigsegv-as-OtherSignal, classify-stale-heartbeat-as-Hung, restart-coordinator-allows-up-to-max-restarts, restart-coordinator-blocks-after-budget-exhausted, backoff-doubles-and-clamps-at-max, audit-row-shape-matches-spec.

### Wiring

EXTEND `crates/argo-runtime/src/worker.rs` (or the actual supervisor file — verify at claim time via `rg -l 'Child::wait\|process::Child' crates/argo-runtime/`). At the existing exit-handler site (≤ 30 LOC delta):

- Call `classify_exit(...)` instead of bare status inspection.
- Call `decide_restart(...)` and act on the decision: `Restart` → sleep + spawn fresh worker subprocess inheriting the same brief assignment; `GiveUp` → release the worker's brief claim (set ledger row back to TODO with a salvage note); `IgnoreCleanExit` → existing clean-shutdown path.
- Always call `emit_audit_row(...)` for non-clean exits.

### CLI

EXTEND `crates/argo-cli/src/cli_worker_cmds.rs` (≤ 20 LOC delta — verify file at claim time) — new `WorkerCmd::Restarts` to show recent OOM/restart audit-chain entries:
```
argo worker restarts [--worker <id>] [--since <duration>]
```

NEW `crates/argo-cli/src/handlers/worker_restarts.rs` (NEW, ≤ 110 LOC) — queries audit chain for `kind = "worker_oom_restart"` rows, emits JSON-line output.

NO migration. NO schema change. Audit-chain row uses existing free-form body.

This is NOT in scope: cgroup-based memory caps for workers (separate brief #236 — `Per-worker resource limits`); auto-tuning the memory cap based on observed OOM frequency (v2 if operators ask); cross-host worker restart (single-host supervisor only in v1); OOM detection on Windows (skip on Windows — `cfg(not(target_os = "windows"))`; document); restart on transient non-OOM errors (the executor-level retry path in `restart.rs` handles those — this brief is the SUPERVISOR-level path for whole-process death); reaping orphaned child processes (use the existing `argo sandbox e2b reap-idle` pattern from the in-flight `mission-e2b-reap-idle` brief for sandbox-side cleanup).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-runtime/src/oom_restart.rs` (NEW, ≤ 240 LOC)
- `crates/argo-runtime/src/oom_restart_tests.rs` (NEW, ≤ 180 LOC)
- `crates/argo-runtime/src/lib.rs` (EXTEND, ≤ 5 LOC — re-export)
- `crates/argo-runtime/src/worker.rs` (EXTEND, ≤ 30 LOC — verify file path at claim time)
- `crates/argo-cli/src/cli_worker_cmds.rs` (EXTEND, ≤ 20 LOC)
- `crates/argo-cli/src/handlers/worker_restarts.rs` (NEW, ≤ 110 LOC)
- `crates/argo-cli/src/handlers/mod.rs` (EXTEND, ≤ 3 LOC)
- `crates/argo-cli/tests/worker_restart_smoke.rs` (NEW, ≤ 120 LOC — synthesizes a worker that exits 137; verifies audit row + restart + ledger-release on budget-exhausted)
- `docs/skycot/runbook/oom-restart.md` (NEW, ≤ 150 LOC) — operator runbook

## Files you must NOT touch

- `crates/argo-executor-computer-use/src/restart.rs` (different path — per-action retry, not whole-process restart)
- `crates/argo-store/migrations/**` (no migration)
- Audit-chain hash logic (immutable)

## Acceptance criteria

- [ ] `cargo build -p argo-runtime -p argo-cli` succeeds
- [ ] `cargo test -p argo-runtime oom_restart` passes all 8 unit tests
- [ ] `cargo test -p argo-cli --test worker_restart_smoke` passes (synthesizes exit-137 worker)
- [ ] On Linux, cgroup `memory.events` `oom_kill` counter is read to confirm OOM (best-effort; absence does not block classification)
- [ ] Exponential backoff: `base * 2^attempt`, clamped at `max_backoff`
- [ ] After `max_restarts` within `restart_window`, worker is given up + ledger row demoted to TODO with salvage note
- [ ] Audit-chain row appended for every non-clean exit (typed `WorkerExitReason` serialized in body)
- [ ] `argo worker restarts --since 1h` lists recent restart events
- [ ] OOM-detection path is `cfg(not(target_os = "windows"))`; on Windows, classifier falls through to `OtherSignal`
- [ ] No migration; no schema change
- [ ] Runbook documents: how to read restart history; how to tune `max_restarts` / backoff; when to escalate to per-worker memory caps (#236)
- [ ] All new files < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-oom-restart-handler`
- PR title: `argo-runtime: OOM-kill detection + exponential-backoff worker restart + audit + worker restarts CLI (mission-oom-restart-handler)`

## Notes for the implementer

- Exit-code 137 is the canonical Linux OOM-kill signature (128 + 9 = SIGKILL). macOS sends signal 9 directly. WSL2 follows Linux. Document the platform matrix in `oom_restart.rs` rustdoc.
- The cgroup `memory.events` file at `/sys/fs/cgroup/<worker_cgroup>/memory.events` is the most reliable OOM confirmation on Linux — `oom_kill <count>` increments each kill. Best-effort read: if the worker isn't in a cgroup OR cgroup-v2 isn't available, the classifier still works (just no `cgroup_evidence` field). Don't fail the classifier on cgroup-read errors.
- DO NOT introduce a config knob to disable OOM detection. It's read-only diagnostic; the restart-decision logic is what the operator tunes.
- Restart-window semantics: 3 restarts within 15 min triggers give-up. Reset is **per-window**, not cumulative — a worker that hits 2 restarts then runs cleanly for 16 minutes gets a fresh budget.
- Give-up path is load-bearing: the ledger row MUST be released (CLAIMED → TODO with a salvage note containing exit-reason summary + last-3-restart-attempts). Otherwise the brief becomes zombied. Use the existing `claim-with-retry` salvage-note format for consistency.
- DO NOT log credential values from the worker's stderr capture. The runbook should document a stderr-tail snippet under sealed-debug only.
- The smoke test should synthesize a child process that exits 137 (use `std::process::exit(137)` after a brief delay) — DO NOT actually allocate to OOM-out in the test (flaky + slow).
- Coordinate w/ in-flight `mission-cron-tick-maintenance-jobs`: that brief sets up cron scaffolding; OOM-restart isn't a cron job (it's reactive supervisor logic), so no overlap.
- DO NOT introduce a notification channel (Slack/email on restart). v1 emits the audit row; channel notification is a follow-up brief that consumes the audit row.
- The CLI `argo worker restarts` is read-only — no `--clear` or `--retry-budget-reset` flags. Restarts are immutable audit history.
