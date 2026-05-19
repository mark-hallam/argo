# Stream P2-13 — scheduled re-eval of active learned skills

**Owner**: codex.
**Depends on**: mission-pm-skill-overlay-load MERGED (so the cron knows which skills are "actively in use").
**Authored against**: origin/main `80ffa124` on `2026-05-19`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-pm-scheduled-skill-reeval`.
**Source**: skycot-pm-worker REPORT.md §8.1 #2 + §11 P2 #13.

## Scope

A skill that was approved 30 days ago may have degraded — underlying tools changed, model versions shifted, prompt templates moved. Today there is no scheduled re-eval; a stale skill keeps shipping until a human notices.

This brief lands a weekly cron in `argo-skill` that runs the existing `eval_runner` (`crates/argo-skill/src/eval_runner.rs`) against every active skill that has an eval suite. Regressions (eval pass rate drops > 10% from previous run) flip the skill to `lifecycle_state = 'flagged'` and emit a `SkillRegressionDetected` audit row. Operator reviews + decides to archive or re-approve.

The cron does NOT auto-archive — that's a future brief once the false-positive rate is measured.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-skill/src/scheduled_reeval_cron.rs` (NEW) — the cron. Mirrors the shape of `per_worker_learning_cron.rs`. ≤ 350 LOC.
- `crates/argo-skill/src/scheduled_reeval_cron_tests.rs` (NEW sibling).
- `crates/argo-store/migrations/<next>__skill_reeval_history.sql` (NEW) — `skill_reeval_history (skill_id, run_at, pass_rate, previous_pass_rate, regressed)` for trend visibility.
- `crates/argo-store/src/migrations/v<next>.rs` (NEW).
- `crates/argo-skill/src/lifecycle.rs` (or wherever `lifecycle_state` is defined) — narrowly scoped: add a `Flagged` variant to the lifecycle enum. Verify the enum is closed (CHECK constraint or exhaustive match) and update accordingly.
- `crates/argo-audit/src/event.rs` — add `SkillRegressionDetected` audit kind.
- `crates/argo-cli/src/cli_skill_cmds.rs` (or existing) — narrowly scoped: add `argo skill reeval [--skill <id>] [--all]` for on-demand runs (the CLI surface for the cron's loop body).
- `crates/argo-cli/src/handlers/skill_reeval.rs` (NEW).

## Files you must NOT touch

- `crates/argo-skill/src/eval_runner.rs` — locked; reuse its API.
- `crates/argo-skill/src/per_worker_learning_cron.rs` — different cron, different cadence.
- Any UI surface — surfacing regressions in the UI is a follow-up brief.

## Acceptance criteria

- [ ] Cron entry point `scheduled_reeval_tick(skills: &SkillStore) -> ReevalReport` runs `eval_runner` against every `lifecycle_state = 'active'` skill that has a non-empty eval set.
- [ ] `ReevalReport` includes `(evaluated, regressed, flagged, skipped_no_eval)`.
- [ ] Regression threshold: pass rate dropped > 10% absolute from the previous recorded run. Configurable via `ARGO_SKILL_REEVAL_REGRESSION_THRESHOLD` (default 0.10).
- [ ] On regression: flip `lifecycle_state` to `'flagged'`, emit `SkillRegressionDetected` audit row with payload `{skill_id, previous_pass_rate, current_pass_rate, eval_run_id}`.
- [ ] `skill_reeval_history` row inserted on every run (regressed or not) so trend is visible.
- [ ] CLI: `argo skill reeval --all --json` runs the tick and prints the report; `--skill <id>` runs against a single skill.
- [ ] Tests: (a) flagging fires when pass rate drops > threshold; (b) no flag when within threshold; (c) skills without eval suites are skipped (not failed); (d) history row inserted on every run; (e) audit row count == regressed count; (f) `Flagged` lifecycle state is treated as inactive by `load_prompt_skills_with_library` (regression test against P0 #1 — a flagged overlay does NOT surface in prompts).
- [ ] `cargo clippy -p argo-skill -p argo-store -p argo-cli --all-targets -- -D warnings` clean.
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean — claim explicitly in PR body.
- [ ] `cargo test -p argo-skill -p argo-store -p argo-cli` passes.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `codex-mission-pm-scheduled-skill-reeval`
- PR title: `skill: weekly re-eval cron with regression flagging (mission-pm-scheduled-skill-reeval)`

## Notes for the implementer

- The cron is NOT auto-spawned by this brief — only the tick function. Spawning lives in the existing scheduler wiring (mirror `per_worker_learning_cron::spawn_periodic` if it exists). If no scheduler wiring exists, the CLI command is sufficient for v1; the operator can wire it to a system cron.
- "Active" filter: `lifecycle_state = 'active'`. Skills without an eval suite (`eval` frontmatter absent) are skipped, NOT failed.
- The 10% threshold is absolute, not relative. From 0.90 → 0.79 triggers; from 0.90 → 0.81 does not.
- Do NOT auto-archive. The whole point is the cron gives the operator a signal; the operator decides.
- Mid-run failures (e.g. eval runner cannot reach a connector) are recorded with `pass_rate = NULL` and do not trigger regression. They emit a `tracing::warn!` and continue.
- Coordinate `SkillRegressionDetected` audit kind with parallel briefs adding kinds.

## Out of scope

- Auto-archive on regression — separate brief, after false-positive rate is measured.
- UI for browsing regression history — separate brief.
- Per-tenant regression-threshold tuning — global env is fine for v1.
- Re-eval triggered by model upgrade (delta against frontier model version) — separate brief.
