# Mission load-bearing wire — cron-tick maintenance jobs (forge / quality / dreams / A-B)

**Owner**: claude. **Depends on**: existing `argo_cron::{CronExpr, CronJob, CronTarget, InMemoryJobStore, JobRunner, JobStore, Scheduler, TickLock}` primitives shipped via the cron handler (`crates/argo-cli/src/handlers/cron.rs` — list/create/delete/trigger/tick/daemon already wired). Existing handlers `forge_subject`, `forge_by_skill`, `forge_optimize_judge`, `forge_by_day` already invoke Forge runs from the CLI side. This brief is the integration that wires those + Dreams + A/B + skill-quality recompute into the existing scheduler as named system jobs.
**Authored against**: origin/main `d1b6501b` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-cron-tick-maintenance-jobs`.

> SOURCE: MISSION_BACKLOG.md Section 5 Claim 7-9 #196 + Section 6 Self-improvement-infra #201 ("Single Forge-cron-tick scheduler" + "Build cron-tick architecture (Forge + quality recompute + dreams + ab-status)"). **Tier M (self-improvement).** Today the cron primitives exist (`argo_cron`) and per-command handlers (`forge_optimize_judge`, `forge_by_skill`, etc.) can be invoked manually. What's MISSING: a default set of named system cron jobs that operators bootstrap with `argo cron init-system` and that run on a sensible cadence. This brief lands the 4 canonical maintenance jobs + the system-init command.

## Scope

### NEW `cron/system/` directory of canonical job definitions

NEW `cron/system/` (4 JSON job files, ≤ 30 LOC each — operator copies to `cron/jobs/` via `argo cron init-system`):

- `cron/system/forge_optimize_tick.json` — cron `0 */6 * * *` (every 6 hours), `target = "system:forge.optimize"`, runs `argo forge optimize-judge` against the most-stale 5 skills (criterion: lowest TrustGrade × oldest forge run).
- `cron/system/skill_quality_recompute.json` — cron `0 3 * * *` (daily 3am UTC), `target = "system:skill.quality_recompute"`, walks every skill version + recomputes per-skill `quality_score` from the trust observations table (existing).
- `cron/system/dreams_curation_tick.json` — cron `30 4 * * *` (daily 4:30am UTC), `target = "system:dreams.curation_tick"`, runs Dreams curation w/ Forge feedback for the day's accumulated trajectories.
- `cron/system/ab_promotion_tick.json` — cron `0 5 * * 0` (weekly Sun 5am UTC), `target = "system:ab.promotion"`, walks A/B experiments past their min-sample threshold + promotes the winner (or extends, or terminates).

### `system:*` CronTarget kind

EXTEND `crates/argo-cron/src/target.rs` (NEW or EXTEND):

- New variant `CronTarget::System { name: String }` — the existing target enum gains a `system:` namespace alongside the existing target kinds.
- `System` target resolution dispatches via a registry pattern: `SystemJobRegistry` (a `HashMap<&'static str, Box<dyn SystemJobHandler>>`) registered at boot.
- New trait `SystemJobHandler` — `async fn run(&self, ctx: SystemJobContext) -> ArgoResult<SystemJobOutcome>` where `SystemJobOutcome` is `{ jobs_processed: usize, skipped: usize, errors: Vec<String>, summary: String }`.

### 4 NEW handlers under `crates/argo-cli/src/system_jobs/`

- `forge_optimize.rs` (NEW, ≤ 200 LOC) — finds 5 stalest skills by (TrustGrade ASC, last_forge_at ASC); invokes the existing `forge_optimize_judge` per-skill; aggregates outcomes.
- `skill_quality_recompute.rs` (NEW, ≤ 150 LOC) — walks `skill_versions`; for each, queries `trust_observations` w/in last 14 days, recomputes `quality_score` (weighted-avg w/ exponential decay); writes back via existing skill catalog API.
- `dreams_curation_tick.rs` (NEW, ≤ 180 LOC) — wraps existing `dreams_plan_templates` + `dreams_distillation` modules; runs one full curation pass; reports `templates_proposed`, `templates_promoted`, `templates_discarded`.
- `ab_promotion_tick.rs` (NEW, ≤ 200 LOC) — queries A/B experiments via the existing variant_selector / variant_store; for each past-min-sample (default 100 trials), computes winner via existing statistical test; promotes / extends / terminates.

### CLI: `argo cron init-system` subcommand

EXTEND `crates/argo-cli/src/cli_command_enum.rs` (≤ 10 LOC) — new `CronCmd::InitSystem { dry_run, force }` variant.

EXTEND `crates/argo-cli/src/handlers/cron.rs` (≤ 60 LOC) — copy/install the 4 `cron/system/` job files into `cron/jobs/`, skipping any that already exist (unless `--force`); print a summary.

### Tests

- 4 unit tests per system-job handler (16 total in `system_jobs/*_tests.rs` files, each ≤ 100 LOC): happy path + 1 failure mode + 1 skip-when-no-work + 1 partial-failure path.
- 1 integration smoke `crates/argo-cli/tests/system_jobs_smoke.rs` (≤ 200 LOC) — boots an in-memory Scheduler + invokes all 4 system jobs via the registry + asserts outcomes.

NO `[escalation]` config changes. NO migration (the 4 handlers consume existing tables; no new columns).

This is NOT in scope: a new persistent job-run history table (operator inspects job outcomes via stdout/stderr today; persistence is a v2 if the runbook surfaces a need); the cron HTTP UI (separate productization brief #198 / #200); per-job alerting (operator wires `argo-channels` notifications via existing anomaly-alerting #1 brief); per-tenant scheduling (system jobs are workspace-global by design in v1; tenancy-scoped jobs are a follow-up); the bootstrap-default-on for new installs (`argo cron init-system` is opt-in; auto-run on first `argo gateway start` is a v2 ergonomics).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-cron/src/target.rs` (NEW or EXTEND, ≤ 80 LOC delta) — `System` target variant + registry trait
- `crates/argo-cron/src/system_registry.rs` (NEW, ≤ 100 LOC) — `SystemJobRegistry` + `SystemJobHandler` trait
- `crates/argo-cli/src/system_jobs/mod.rs` (NEW, ≤ 25 LOC)
- `crates/argo-cli/src/system_jobs/forge_optimize.rs` (NEW, ≤ 200 LOC)
- `crates/argo-cli/src/system_jobs/forge_optimize_tests.rs` (NEW, ≤ 100 LOC)
- `crates/argo-cli/src/system_jobs/skill_quality_recompute.rs` (NEW, ≤ 150 LOC)
- `crates/argo-cli/src/system_jobs/skill_quality_recompute_tests.rs` (NEW, ≤ 100 LOC)
- `crates/argo-cli/src/system_jobs/dreams_curation_tick.rs` (NEW, ≤ 180 LOC)
- `crates/argo-cli/src/system_jobs/dreams_curation_tick_tests.rs` (NEW, ≤ 100 LOC)
- `crates/argo-cli/src/system_jobs/ab_promotion_tick.rs` (NEW, ≤ 200 LOC)
- `crates/argo-cli/src/system_jobs/ab_promotion_tick_tests.rs` (NEW, ≤ 100 LOC)
- `crates/argo-cli/src/handlers/cron.rs` (EXTEND, ≤ 60 LOC delta) — `init_system` handler
- `crates/argo-cli/src/cli_command_enum.rs` (EXTEND, ≤ 10 LOC delta)
- `crates/argo-cli/tests/system_jobs_smoke.rs` (NEW, ≤ 200 LOC)
- `cron/system/forge_optimize_tick.json` (NEW, ≤ 30 LOC)
- `cron/system/skill_quality_recompute.json` (NEW, ≤ 30 LOC)
- `cron/system/dreams_curation_tick.json` (NEW, ≤ 30 LOC)
- `cron/system/ab_promotion_tick.json` (NEW, ≤ 30 LOC)
- `docs/skycot/runbook/cron-system-jobs.md` (NEW, ≤ 200 LOC) — schedules / what each does / how to inspect outcomes / how to disable

## Files you must NOT touch

- `crates/argo-store/migrations/**` (no new schema)
- `crates/argo-forge/**` source (consume forge as existing dep; do not modify)
- `crates/argo-skill/src/dreams_distillation.rs` (consume; do not modify)

## Acceptance criteria

- [ ] `argo cron init-system` creates 4 jobs in `cron/jobs/` from the `cron/system/` templates
- [ ] `argo cron init-system --dry-run` previews without writing
- [ ] `argo cron tick` resolves `system:forge.optimize` etc. through the new registry without error
- [ ] Each of the 4 system handlers passes its 4 unit tests
- [ ] Integration smoke `cargo test -p argo-cli --test system_jobs_smoke` passes
- [ ] System jobs emit `SystemJobOutcome` w/ structured `jobs_processed` / `skipped` / `errors` / `summary`
- [ ] Runbook covers default schedules + what each job mutates + how to disable
- [ ] No file > 500 LOC; per-handler file ≤ 200 LOC
- [ ] No migration; no Cargo.lock churn beyond the cron-target enum touch
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-cron-tick-maintenance-jobs`
- PR title: `cron: 4 system maintenance jobs (forge / quality / dreams / ab) + init-system CLI (mission-cron-tick-maintenance-jobs)`

## Notes for the implementer

- The 4 system jobs are SELF-CONTAINED — they call into existing handlers (`forge_optimize_judge`, dreams curation API, A/B variant_selector). Don't re-implement the work; wrap.
- The `SystemJobRegistry` is the load-bearing extensibility seam — future v2 system jobs (`system:audit.chain_watchdog`, `system:trust_observation.gc`, etc.) plug in through the same trait. Keep the trait minimal: `async fn run(ctx) -> Outcome`.
- Each system handler should be IDEMPOTENT — re-running the same tick should be a no-op (or process the same backlog). Operators may re-trigger via `argo cron trigger system:forge.optimize` for debugging; safety is on the handler.
- `skill_quality_recompute` exponential-decay weight: 14-day half-life is reasonable starting point. Document the formula in the runbook so operators understand "why does my skill's quality drop after a week of no usage."
- `ab_promotion_tick` should be CONSERVATIVE — extend when min-sample crossed but confidence interval still overlaps; only promote when CI is fully separated. Per-experiment configuration via existing variant_store; do NOT add new config.
- The 4 cron expressions (every-6h forge / 3am quality / 4:30am dreams / Sunday-5am A-B) are starting points. Operators tune via `argo cron edit <name>`. Document in runbook + comment-block in each JSON file.
- `init-system` should NEVER overwrite without `--force`. Operators may have customized; respect the existing file.
- The handlers print structured outcome JSON to stdout AND a human-readable summary to stderr. Cron daemon collects via tracing-subscriber; manual `argo cron trigger` shows both directly.
- DO NOT add a `system:cleanup.gc_old_runs` job in v1. GC of old job-run records is a v2 brief once we have a persistent job-run history.
- DO NOT register the system jobs automatically on `argo gateway start` — opt-in via `init-system` is intentional; auto-registration creates "where did these jobs come from" surprise for operators.
- All 4 handlers should TIMEBOX their work: forge_optimize processes ≤ 5 skills per tick; quality_recompute processes ≤ 200 skills per tick; dreams_curation processes ≤ 24h of trajectories; ab_promotion_tick processes ≤ 50 experiments per tick. Document the bounds in the runbook + per-handler rustdoc.
