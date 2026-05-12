# Cron-tick maintenance jobs

Argo ships four canonical system maintenance jobs that operators
opt into via `argo cron init-system`. They share the regular cron
tick loop (one tick per minute under the `cron/.tick.lock`) but
dispatch through an internal [`SystemJobRegistry`] rather than an
output-delivery adapter — they don't message a Slack channel, they
invoke a workspace-internal handler.

The four jobs:

| job | cadence | system name | what it does |
| --- | --- | --- | --- |
| `forge_optimize_tick`     | every 6 h (00, 06, 12, 18 UTC) | `system:forge.optimize`          | Re-run Forge optimize-judge for the 5 stalest skills |
| `skill_quality_recompute` | daily at 03:00 UTC             | `system:skill.quality_recompute` | Recompute every active skill's quality score from a 14-day observation window |
| `dreams_curation_tick`    | daily at 04:30 UTC             | `system:dreams.curation_tick`    | Curate the last 24 h of trajectories through the Dreams template pipeline |
| `ab_promotion_tick`       | Sundays at 05:00 UTC           | `system:ab.promotion`            | Scan A/B experiments past their min-sample threshold and promote / extend / terminate |

## Installing

```sh
# Preview what would be written.
argo cron init-system --dry-run

# Install — refuses to overwrite existing files.
argo cron init-system

# Overwrite local customisations.
argo cron init-system --force
```

`init-system` copies the four templates from `cron/system/` into
`cron/jobs/`. Files already present in `cron/jobs/` are preserved
unless you pass `--force`. The handler validates that each
template parses as a `CronJob` before writing, so a corrupted
shipped template fails fast instead of producing a broken job.

## Verifying the schedule

```sh
argo cron list --format markdown
```

The four jobs appear with `platform=system` and `chat=<job-name>`.
`platform=system` is the convention that flags a job as
system-dispatched rather than delivered to a channel.

## Running a one-off invocation

```sh
argo cron trigger forge_optimize_tick
```

This dispatches the registered handler immediately and prints
the [`SystemJobOutcome`] (structured JSON to stdout + a one-line
summary to stderr).

## Outcome shape

Every handler returns a [`SystemJobOutcome`]:

```json
{
  "jobs_processed": 3,
  "skipped": 1,
  "errors": ["skill-foo: judge timeout"],
  "summary": "forge.optimize: 3/4 skills processed, 1 errors"
}
```

The `cron` daemon emits the JSON to stdout (collected by a
tracing-subscriber) and the human-readable `summary` to stderr.

## Default cron expressions

The four shipped expressions use literal / comma-list syntax
because `argo-cron` doesn't yet support `*/N` step syntax:

- forge: `0 0,6,12,18 * * *` (literal hour list, equivalent to `0 */6 * * *`)
- quality: `0 3 * * *`
- dreams: `30 4 * * *`
- ab: `0 5 * * 0`

Operators tune any of these via `argo cron edit <name>` — the
daemon picks up changes on the next tick.

## Per-handler bounds

Each handler timeboxes its work to keep a single tick under a
minute:

- **forge.optimize**: ≤ 5 stalest skills per tick (`MAX_STALE_PER_TICK`).
- **skill.quality_recompute**: ≤ 200 skills per tick (`MAX_SKILLS_PER_TICK`).
- **dreams.curation_tick**: 24 h trajectory window (`WINDOW_HOURS`).
- **ab.promotion**: ≤ 50 experiments per tick (`MAX_EXPERIMENTS_PER_TICK`).

Operators wanting a deeper sweep run `argo cron trigger
<system-job-name>` repeatedly.

## Idempotency

Each handler is idempotent — re-running the same tick is safe:

- forge.optimize: selector returns the same stale set if previous
  runs failed; freshly-stale set if they succeeded.
- skill.quality_recompute: `SkillQualityStore::record` uses
  `INSERT OR REPLACE` keyed on `(skill_id, computed_at)`.
- dreams.curation_tick: distillation persists template proposals
  by content hash; duplicates are coalesced.
- ab.promotion: applied decision is a no-op when the experiment is
  already in the post-decision state.

## Quality-score decay

`skill_quality_recompute` weights observations with a 14-day
half-life: an observation a week old contributes ~50 % of a fresh
observation. Skills with no usage for 14+ days will see their
quality score drift toward the default (best-case 100) rather than
the freshly-failed score, because there's no recent evidence.

## Disabling a system job

System jobs are regular `CronJob`s under the hood. Disable one
without deleting it:

```sh
# Edit cron/jobs/<name>.json and flip "enabled": true → false.
argo cron edit forge_optimize_tick
```

The daemon picks up the change on the next tick.

## Real-handler wiring (status)

This brief (`mission-cron-tick-maintenance-jobs`) lands the
wiring: the registry, the `system:*` target convention, the
four templates, the `init-system` CLI, and the dispatch path
through `argo cron tick`. Each handler is implemented as a
selector + invoker pair behind small traits so the production
deps (SessionStore, Forge, dreams_distillation, variant_store)
can be swapped in without touching the dispatch path.

The default registry registers no-op selector/invoker impls
today — `argo cron tick` resolves `system:forge.optimize` and
returns the empty `SystemJobOutcome` ("no stale skills found").
The follow-up briefs replace each no-op with the real selector:

- `claim-wire-forge-stale-skills-selector` — wires the skill catalog +
  forge_run history queries.
- `claim-wire-skill-quality-recompute-writer` — wires
  `SkillQualityStore::record`.
- `claim-wire-dreams-curator` — wires `dreams_distillation` +
  `dreams_plan_templates`.
- `claim-wire-ab-decider` — wires `argo_skill::ab_test_stats`.

Each follow-up touches one handler file and one (test) wiring
site — small, reversible, well-bounded.

## Out of scope (v1)

- Persistent job-run history table. Outcomes are emitted to
  stderr/stdout today; persistence is a v2 once a runbook surfaces
  the need for time-series querying.
- Cron HTTP UI. Productized separately (#198 / #200).
- Per-job alerting. Operators wire `argo-channels` notifications
  via the existing anomaly-alerting brief (#1).
- Per-tenant scheduling. System jobs are workspace-global by
  design; tenancy-scoped jobs are a follow-up.
- Auto-registration on `argo gateway start`. `init-system` is
  intentionally opt-in so "where did these jobs come from?" never
  becomes a surprise.
