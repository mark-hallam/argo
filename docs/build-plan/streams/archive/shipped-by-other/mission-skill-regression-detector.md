# Mission load-bearing wire — skill regression detector + auto-demote on quality drop

**Owner**: claude. **Depends on**: `argo-skill::quality::QualityScore::latest_all` exists (verified at `crates/argo-skill/src/quality.rs:193`); skill_quality_score table exists (per inline comment line 39).
**Authored against**: origin/main `6da4af77` on `2026-05-10`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-skill-regression-detector`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 4 item #82 ("Build skill regression detector — quality drops > X% → demote"). Today `skill_quality_score` records every recompute as a row, but nothing watches the timeline. A skill that worked at score 85 last week and drops to 40 today silently keeps the high promotion grade. Mission Claim 4 ("skills shared and learned") implies quality is a feedback signal — when the signal degrades, the skill needs demoting before bad outputs reach more workers.

## Scope

Add a regression-detection primitive that compares the latest `QualityScore` for each skill against its prior baseline (highest score in the trailing window) and emits a "demote" decision when the drop exceeds a configurable threshold:

```rust
pub struct RegressionConfig {
    pub min_score_drop: u8,           // default 20 (20-point absolute drop)
    pub min_pct_drop: u8,             // default 25 (25% relative drop)
    pub trailing_window_days: u32,    // default 30
    pub min_history_count: u8,        // default 3 (need ≥ N scores to baseline)
}

pub enum RegressionDecision {
    NoBaseline,                                 // < min_history_count
    Stable { current: u8, baseline: u8 },
    Regressed { current: u8, baseline: u8, drop_abs: i16, drop_pct: u8 },
}

pub async fn detect_regression(
    skill_id: SkillId,
    store: &dyn QualityScoreReader,
    cfg: &RegressionConfig,
) -> ArgoResult<RegressionDecision>;
```

The brief delivers the detection + `RegressionDecision` enum + a thin classifier function. The actual demote-action wire is a SEPARATE follow-up brief — this brief stops at "decision emitted." Workers / cron tasks consuming the decision can call into `argo-skill::trust` to demote.

This is NOT in scope: writing the cron tick that calls the detector (separate brief; pairs with `mission-skill-quality-cron`); the demote action itself; alerting / notification of demotes; cross-tenant baseline.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `crates/argo-skill/src/regression.rs` (NEW, ≤ 200 LOC) — `RegressionConfig`, `RegressionDecision`, `detect_regression`, `QualityScoreReader` trait (narrow read surface). Pure logic + DB read; no side effects.
- `crates/argo-skill/src/regression_tests.rs` (NEW, sibling) — 8 unit tests: no-baseline-when-below-min-history; stable-when-no-drop; stable-when-drop-below-thresholds; regressed-on-absolute-drop; regressed-on-percentage-drop; regressed-when-both-thresholds-hit; baseline-uses-trailing-window-max; respects-min_history_count.
- `crates/argo-skill/src/lib.rs` — narrowly scoped: ONE `pub mod regression;` line + ONE re-export `pub use regression::{detect_regression, RegressionConfig, RegressionDecision};`. NO other edits.
- `crates/argo-skill/Cargo.toml` — narrowly scoped: confirm `chrono` is in `[dependencies]` (it is, used by `QualityScore`). NO other Cargo edits.
- `crates/argo-skill/tests/skill_regression_smoke.rs` (NEW) — integration smoke: seed `skill_quality_score` table with synthetic timeline (3 high scores + 1 low recent), call `detect_regression`, assert `Regressed` decision.

## Files you must NOT touch

- `crates/argo-skill/src/quality.rs` — `QualityScore` and persistence are upstream contracts; reuse only
- `crates/argo-skill/src/trust.rs` — demote action is separate brief
- `crates/argo-store/migrations/**` — no schema change (uses existing skill_quality_score)
- `crates/argo-cli/handlers/skill_*.rs` — CLI surface separate brief

## Acceptance criteria

- [ ] `RegressionConfig` struct with `Default` impl exists; defaults are documented
- [ ] `RegressionDecision` enum has 3 variants: `NoBaseline`, `Stable`, `Regressed`
- [ ] `detect_regression(skill_id, store, cfg)` is async, returns `ArgoResult<RegressionDecision>`
- [ ] Returns `NoBaseline` when fewer than `cfg.min_history_count` scores exist for the skill
- [ ] Returns `Stable` when the latest score does NOT exceed BOTH absolute and percentage drop thresholds (note: BOTH must be exceeded, AND-logic, to avoid noise from small-baseline percentage swings)
- [ ] Returns `Regressed` with `drop_abs` (latest - baseline as i16; baseline > latest in this case) and `drop_pct` (rounded percentage drop, u8)
- [ ] Baseline = max score in trailing `cfg.trailing_window_days` (NOT the most recent — the window-max, so a single recovered score doesn't mask a regression from peak)
- [ ] Trailing window is exclusive of the latest score being evaluated (otherwise current would baseline against itself when it's already low)
- [ ] `QualityScoreReader` trait exposes the minimum surface needed: a single method returning `Vec<QualityScore>` for a skill in a time window
- [ ] 8 unit tests cover the matrix above
- [ ] 1 integration smoke exercises end-to-end against seeded `skill_quality_score` rows
- [ ] No new public API beyond the listed exports
- [ ] `cargo build -p argo-skill` clean
- [ ] `cargo clippy -p argo-skill --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-skill` passes (skip pre-existing flakes per KNOWN_FLAKES.md)
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-skill-regression-detector`
- PR title: `skill: detect_regression primitive over skill_quality_score timeline (mission-skill-regression-detector)`

## Notes for the implementer

- The AND-logic on absolute + percentage thresholds prevents two failure modes: (a) a baseline of 100 dropping to 99 (1% drop, 1-point drop) — neither threshold; (b) a baseline of 4 dropping to 3 (25% drop, 1-point drop) — pct only, but the absolute is meaningless at low scores. AND requires both → regressed. Document this rationale in the rustdoc.
- Use `Vec<QualityScore>` from `latest_all` filtered by `skill_id` and time window — that's the simplest read shape. If `latest_all` doesn't filter, add a narrow `recent_scores_for_skill(skill_id, since: DateTime<Utc>)` method on the `QualityScoreReader` trait you're introducing — don't modify the upstream `QualityScore` persistence.
- The min_history_count guard prevents new skills (with 1-2 scores) from registering as regressed when their second score is lower than their first. A skill needs to have stabilized before regression detection makes sense.
- Don't emit any side effects (no logging, no audit row, no demote). The detector returns a `Decision` and that's it. The cron tick / demoter is a separate concern.
- The default thresholds (20-point absolute / 25% relative / 30-day window / 3-min-history) are tunable. Don't optimize them in this brief — operators iterate via `RegressionConfig`. Document the defaults' rationale in the struct's rustdoc.
- If `QualityScore::latest_all()` returns a Vec without time filtering and that Vec is huge in production, accept it for v1 — the table is small. A future brief can add a windowed query.
