# Mission load-bearing wire — auto_continue_at_max_iterations policy (Amendment 9)

**Owner**: claude. **Depends on**: existing worker iteration loop in `crates/argo-runtime/src/` (the loop that drives agent turns); existing audit-chain emission + `argo-channels` notify path (consumed; not modified).
**Authored against**: origin/main `ea8b4406` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-auto-continue-max-iterations-policy`.

**Drift-fix amendment 2026-05-13** (vs HEAD 6159b789): brief premise partially shipped under a different name. The 3-variant policy enum + `evaluate()` + 6 unit tests the brief asks to NEW at `crates/argo-runtime/src/max_iter_policy.rs` ALREADY SHIPPED at `crates/argo-core/src/iter_policy.rs` (`IterationOnMax::{Halt, PromptUser, AutoContinue}` + `decide()` + `ExtensionLimits` hard-ceiling — same Amendment 9 §6.6 source). The brief's worker-iteration-loop target `crates/argo-runtime/src/lib.rs` is 86 LOC of planner-trait module-decls — no loop lives there. Actual `max_iterations` enforcement is `crates/argo-core/src/agent.rs:142` (still hard-stop, not yet wired to `iter_policy::decide`). **Rewrite scope** for a fresh claimer: may-touch `crates/argo-core/src/agent.rs` (wire `iter_policy::decide` + emit audit row on extend/halt), `crates/argo-config/src/sections_budget.rs` (`[budget.iter_policy]` config), `crates/argo-vertical/src/manifest_validate.rs` (per-vertical override), `crates/argo-cli/src/handlers/worker_continue.rs` NEW (operator resume CLI for the `PromptUser` path). The NEW `argo-runtime/src/max_iter_policy.rs` line is DROPPED (file already shipped under a different crate name). Sections file `sections.rs` → `sections_budget.rs` sibling per current pattern.

> SOURCE: MISSION_BACKLOG.md Section 5 Claim 7-9 #183 ("auto_continue_at_max_iterations policy (Amendment 9)"). **Tier M (worker-safety policy).** Today: when a worker hits `max_iterations` (the per-task safety cap), it hard-stops + raises an error. Operators with long-running tasks (research, deep coding, multi-step browser flows) want a configurable "auto-continue" policy: instead of hard-stopping, the worker pauses, emits a notification to the operator's preferred channel (e.g. Slack ask), and EITHER waits for explicit `/continue` AOR auto-resumes after a configurable grace (with a hard absolute-iteration ceiling). This brief lands the policy enum + wiring.

## Scope

NEW `crates/argo-runtime/src/max_iter_policy.rs` (≤ 200 LOC):

- `pub enum MaxIterationsPolicy` — 3 variants:
  - `HardStop` (default; today's behaviour preserved when no config set)
  - `NotifyAndWait { channel: ChannelHint, wait_secs: u64, resume_token_required: bool }` — pause; notify operator; wait `wait_secs` for `/continue` or timeout-then-hard-stop
  - `AutoResume { extend_iterations_by: u32, absolute_cap: u32, notify_channel: Option<ChannelHint> }` — auto-extend by N iterations; never exceed `absolute_cap`; optional notification at each extension
- `pub fn evaluate(reached: u32, configured_max: u32, policy: &MaxIterationsPolicy) -> MaxIterationsDecision` — `{ ContinueWithNewMax(u32), WaitForResume { wait_until, resume_token }, HardStop { reason } }`.
- `pub struct MaxIterationsDecision` w/ `audit_log_entry(&self) -> AuditEntry` — every decision writes to audit-chain for post-hoc explain.
- 6 unit tests: hard-stop default, notify-and-wait timeout-then-stop, notify-and-wait `/continue`-received-extends, auto-resume under absolute-cap, auto-resume hit absolute-cap → stop, ChannelHint serialization.

### Worker-loop integration

EXTEND `crates/argo-runtime/src/lib.rs` (or the worker-iteration site; verify at claim time — likely `agent_loop.rs` or similar; ≤ 40 LOC delta):

- At `iteration == configured_max - 1`, BEFORE the hard-stop, call `MaxIterationsPolicy::evaluate`.
- On `ContinueWithNewMax(new_max)`: update the in-memory iteration ceiling; emit audit-chain row "auto_continue_max_iter_extended"; continue.
- On `WaitForResume`: persist a `wait_resume_token` to the worker's session state; emit notification via `argo-channels`; suspend the worker (`Task::yield` + polled-wake on resume).
- On `HardStop`: emit audit-chain row "max_iter_hard_stop"; raise the existing error.

### Resume-token + `/continue` wire

EXTEND `crates/argo-cli/src/handlers/` (or gateway routes) (≤ 60 LOC across small delta):

- NEW `argo worker continue <session_id> [--resume-token <token>]` CLI subcommand (or HTTP `POST /worker/{id}/continue`).
- Verifies `resume_token` against the worker's session state; if `resume_token_required = true` and no token → reject.
- On success: signals the suspended worker via existing inter-task channel (the worker-loop's already-existing pause/resume primitive — verify at claim; if missing, the brief needs to add a minimal `tokio::sync::Notify` plus session-state field).

### Config wire-up

EXTEND `crates/argo-config/src/sections.rs` (≤ 30 LOC):

```toml
[worker.max_iterations]
default = 50          # existing — preserved
policy = "HardStop"    # NEW: "HardStop" | "NotifyAndWait" | "AutoResume"

[worker.max_iterations.notify_and_wait]
channel = "slack"      # which argo-channels target
wait_secs = 3600       # 1 hour default
resume_token_required = true

[worker.max_iterations.auto_resume]
extend_iterations_by = 25
absolute_cap = 200
notify_channel = "slack"
```

### Vertical-level override

EXTEND `crates/argo-vertical/src/manifest_validate.rs` (≤ 12 LOC) — accept optional `[vertical.max_iterations]` table in `vertical.toml` that overrides the workspace default (per-vertical policy). E.g. a long-running research vertical might `policy = "AutoResume"` while a sales-follow-up vertical stays `"HardStop"`.

### Tests

EXTEND existing worker-loop tests (≤ 100 LOC across existing test files):

- Add 4 integration tests in `crates/argo-runtime/src/agent_loop_tests.rs` (or equivalent — verify at claim): full hard-stop path, full NotifyAndWait → timeout-stop path, full NotifyAndWait → `/continue`-resume path, full AutoResume → absolute-cap-stop path.

NO migration. NO `[escalation]` config change (this is policy enforcement, not approval routing).

This is NOT in scope: cost-aware auto-continue (the `mission-llm-budget-hard-ceiling` brief already covers cost-based hard-stop; orthogonal); machine-learning-driven "should we auto-continue" heuristic (out-of-scope; operator config is the v1 mechanism); auto-summarize-then-continue (truncate context + continue is a separate "context-budget overflow" brief); per-iteration cost-attribution (separate brief); a "guess at what the operator wants" implicit policy (operator-explicit config only).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-runtime/src/max_iter_policy.rs` (NEW, ≤ 200 LOC)
- `crates/argo-runtime/src/max_iter_policy_tests.rs` (NEW, ≤ 180 LOC, included via `#[path]`)
- `crates/argo-runtime/src/lib.rs` (EXTEND, ≤ 40 LOC delta — wire into worker iteration site)
- `crates/argo-runtime/src/agent_loop_tests.rs` OR equivalent existing test file (EXTEND, ≤ 100 LOC) — 4 integration tests
- `crates/argo-cli/src/handlers/worker_continue.rs` (NEW, ≤ 120 LOC) — CLI handler + resume-token verification
- `crates/argo-cli/src/cli_command_enum.rs` (EXTEND, ≤ 12 LOC) — `WorkerCmd::Continue` variant
- `crates/argo-cli/src/handlers/mod.rs` (EXTEND, ≤ 3 LOC)
- `crates/argo-config/src/sections.rs` (EXTEND, ≤ 30 LOC)
- `crates/argo-config/src/defaults.rs` (EXTEND, ≤ 12 LOC)
- `crates/argo-vertical/src/manifest_validate.rs` (EXTEND, ≤ 12 LOC) — per-vertical override
- `docs/skycot/runbook/worker-max-iter-policy.md` (NEW, ≤ 180 LOC) — operator runbook for the 3 modes

## Files you must NOT touch

- `crates/argo-channels/**` (consume existing NotifyChannel; do not modify)
- `crates/argo-store/migrations/**` (no migration)
- Any vertical manifest (the brief lands the override SCHEMA support; verticals adopting it is per-vertical follow-up)

## Acceptance criteria

- [ ] `cargo test -p argo-runtime max_iter_policy` passes all 6 unit tests
- [ ] 4 integration tests in agent-loop tests cover all 3 policies' happy + sad paths
- [ ] `argo worker continue <session_id> --resume-token <token>` resumes a suspended worker
- [ ] `argo worker continue <session_id>` without `--resume-token` errors when `resume_token_required = true`
- [ ] Audit-chain rows appear for `max_iter_hard_stop` / `auto_continue_max_iter_extended` / `auto_continue_max_iter_resumed_by_operator`
- [ ] Default policy is `HardStop` (matches today's behaviour — backward-compatible)
- [ ] Per-vertical override in `vertical.toml [vertical.max_iterations]` is honored
- [ ] `absolute_cap` in `AutoResume` is enforced (test verifies hard-stop at cap)
- [ ] No file > 500 LOC; per-file ≤ briefed budgets
- [ ] No migration; no breaking config-schema change
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-auto-continue-max-iterations-policy`
- PR title: `runtime: max-iter policy (HardStop / NotifyAndWait / AutoResume) + worker-continue CLI (mission-auto-continue-max-iterations-policy)`

## Notes for the implementer

- The 3-variant enum is the load-bearing API. Don't add a 4th variant ("EscalateAndAutoResume") in v1 — that's NotifyAndWait composed with a default-resume on timeout, which the operator can express via a follow-up brief or config.
- Backward-compat is essential: default `HardStop` must match today's behaviour byte-for-byte (same error, same audit-chain row, same notification). The new policy variants are opt-in.
- `resume_token` should be a random 32-byte URL-safe string emitted only via the notify channel (not stored in stdout). The operator pastes it into `argo worker continue --resume-token`. If `resume_token_required = false`, the policy allows any operator with session-id to continue — appropriate for low-stakes work.
- The worker suspension primitive: ideally the worker loop already has a pause/resume awaitable (`tokio::sync::Notify` or similar). If not, the brief should add a minimal one + persist `wait_resume_token` to session state. Verify at claim time which path is shortest.
- Per-vertical override: the precedence chain is vertical > workspace > built-in default `HardStop`. Mirror the `operator_risk_overrides` precedence pattern (#1361).
- The runbook covers operator workflow: "you set NotifyAndWait, your worker pauses, you get a Slack message with a resume-token, you `argo worker continue ...`, work resumes." Include the audit-row schema so operators can grep for `auto_continue_*` events.
- The 4 integration tests should use a stub `MaxIterationsPolicy` source (the existing test-fixture pattern for worker-loop tests). Don't introduce real Slack/email channels in the test harness.
- The CLI `argo worker continue` is the operator escape hatch. Document it prominently in `--help` so operators discover it.
- DO NOT auto-resume when the LAST iteration's tool-call FAILED. If the worker hit max-iter while in an error state, hard-stop regardless of policy — auto-continuing a failing worker burns budget for no progress. Document this in `max_iter_policy.rs` rustdoc.
- DO NOT add an "estimated remaining iterations" heuristic. The operator chose the cap; the policy honors it. ML-driven "should we continue" is a separate, much bigger brief.
- The `audit_log_entry` method on `MaxIterationsDecision` makes the policy decision REVIEWABLE — operators can trace why a worker hard-stopped vs continued. Audit-chain integration is via the existing per-tenant audit path; no schema change.
- The `wait_secs` upper-bound is implicit: the worker process must stay alive. Don't add absolute-max `wait_secs > 24h` — if operators want overnight-pause, the architecture is wrong (the worker should be checkpointed + the system shutdown until resume). Document `wait_secs` ≤ 4h as the recommended upper-bound in the runbook.
