# Mission load-bearing wire — wire skill QualityScore into A/B promote decision

**Owner**: claude. **Depends on**: `argo-skill::ab_test` exists; `QualityScore` persistence at `argo-skill::quality::QualityScore` (verified at quality.rs:102).
**Authored against**: origin/main `e17857b0` on `2026-05-10`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-skill-ab-quality-promote`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 4 item #70 ("Wire skill quality scores into A/B promote decision"). Today A/B promote uses raw success rate + chi-square significance only (`maybe_complete` in `ab_test.rs`). Two variants with similar success rates but very different `QualityScore` should NOT promote the lower-quality one. Mission Claim 4 ("skills shared and learned") needs quality as a real signal in the promote decision.

## Scope

Extend `maybe_complete` (and the public `argo skill ab-promote` CLI) to factor `QualityScore` into the winner-selection logic when both variants have a recent score. Specifically:

1. After chi-square significance is established + min-samples gate is met, fetch `latest_score` for both variants from `skill_quality_score` (via existing `QualityScore::latest` API).
2. If BOTH variants have a recent quality score (within last 7 days, configurable):
   - Compute weighted score: `winner_score = success_rate * 0.7 + (quality_score / 100) * 0.3` per variant
   - Pick the variant with higher weighted score
3. If neither (or only one) variant has a recent quality score: fall back to existing success-rate-only winner selection (today's behavior preserved).

Add config knob: `QualityWeightConfig { weight_quality: f32 = 0.3, max_score_age_days: u32 = 7 }`. Defaults are conservative; operators tune via env or config.

This is NOT in scope: changing chi-square logic; adding new statistical tests; recomputing quality at promote-time (use latest cached); cross-tenant promote.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-skill/src/ab_test.rs` — narrowly scoped: extend `maybe_complete` (or its sibling fn that picks the winner) to fetch `QualityScore` and apply weighted selection. Add `QualityWeightConfig` struct + `Default`. NO change to chi-square or min-samples logic.
- `crates/argo-skill/src/ab_test_tests.rs` (or sibling) — narrowly scoped: append 5 NEW unit tests: both-have-quality-quality-tips-decision; only-one-has-quality-falls-through-to-success-rate; neither-has-quality-falls-through-to-success-rate; quality-score-too-old-treated-as-missing; weight-knob-changes-decision.
- `crates/argo-cli/src/handlers/skill_ab.rs` (or wherever `argo skill ab-promote` handler lives — verify in first 5min) — narrowly scoped: ONE config-flag pass-through for `QualityWeightConfig` from CLI args / config to `maybe_complete`. NO logic change here.

## Files you must NOT touch

- `crates/argo-skill/src/quality.rs` — `QualityScore` + `latest` are upstream contracts; reuse only
- `crates/argo-skill/src/ab_test_stats.rs` — chi-square stats are upstream; reuse only
- `crates/argo-store/migrations/**` — no schema change (uses existing skill_quality_score)
- Any non-skill, non-cli crate

## Acceptance criteria

- [ ] `QualityWeightConfig` struct with `Default` exists, weights documented
- [ ] When both variants have quality scores within `max_score_age_days`, winner = highest `success_rate * (1 - weight) + (quality / 100) * weight`
- [ ] When only one variant has a recent quality score, falls back to today's success-rate-only logic
- [ ] When max_score_age_days threshold expires for either variant, treats as missing → falls back
- [ ] Existing chi-square + min-samples gates remain unchanged
- [ ] No-op when `weight_quality = 0.0` (operator can disable)
- [ ] 5 new unit tests cover the matrix
- [ ] All existing ab_test tests still pass
- [ ] `cargo build -p argo-skill` clean
- [ ] `cargo clippy -p argo-skill -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-skill -p argo-cli` passes (skip pre-existing flakes)
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-skill-ab-quality-promote`
- PR title: `skill: factor QualityScore into A/B promote winner-selection (mission-skill-ab-quality-promote)`

## Notes for the implementer

- The 0.7/0.3 weights are documented defaults; the brief's intent is "quality matters but success rate is primary." Don't bikeshed; ship the defaults, operators tune via config.
- Quality-score-age guard: a stale quality score is misleading. If `now - latest_score.computed_at > max_score_age_days`, treat as missing.
- Don't recompute quality at promote-time. The cron in `mission-skill-quality-cron` (#78) handles refreshes; A/B promote just reads the latest cached score.
- If `argo skill ab-promote <run-id>` is currently a manual operator action: the promote handler should print the chosen variant + reason ("won by success rate" vs "won by weighted-quality decision") so operators see the logic that fired.
