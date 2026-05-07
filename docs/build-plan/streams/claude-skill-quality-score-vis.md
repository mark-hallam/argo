# claude-skill-quality-score-vis — surface `skill_quality_score` everywhere

**Owner**: claude. **Depends on**: existing `skill_quality_score` table
(migration `24__skill_quality_score.sql`); existing `argo-skill::quality::SkillQualityStore`.

## Why

The quality scoring pipeline already runs and persists rows. The CLI surface
is partial (`skill_quality_list`, `skill_quality_recompute`, `skill_quality_trend`
exist) but there's no consolidated "show me the latest score with the inputs
that produced it" surface, and the gateway has zero exposure. Operators
running review cycles have to hop between commands.

## Scope

- **NEW CLI** `argo skill quality <id>` prints latest score + components
  (success_rate, refusal_rate, completion_latency, etc. — whatever columns
  the table has) in markdown + JSON.
- **NEW gateway route** `GET /skills/:id/quality` returning the same shape
  (JSON only — gateway is JSON-by-default for read endpoints).
- Both surfaces support `--limit <N>` / `?limit=N` for "last N scores."

## Approach

1. Read `crates/argo-skill/src/quality.rs` — note that `SkillQualityStore`
   already has reader methods (e.g. `latest_for`, `trend_for`). REUSE; do
   not duplicate query logic.
2. **NEW** `crates/argo-cli/src/handlers/skill_quality.rs` (~220 LOC).
   This is the single-skill quality view; existing `skill_quality_list.rs`
   covers the all-skills list — keep them sibling files.
3. **NEW** `crates/argo-cli/src/handlers/gateway/routes_skills_quality.rs`
   (~180 LOC). Wire it into `routes_skills.rs::dispatch_route` with one
   match arm.
4. Schema of returned JSON: `{ skill_id, latest: {...}, history: [{ computed_at, score, ... }] }`.

## Files you may touch

- BLANKET CLI ALLOWLIST applies.
- `crates/argo-cli/src/handlers/skill_quality.rs` (NEW, ≤ 280 LOC).
- `crates/argo-cli/src/handlers/gateway/routes_skills_quality.rs` (NEW, ≤ 250 LOC).
- `crates/argo-cli/src/handlers/gateway/routes_skills.rs` — narrowly: 1 dispatch arm.
- `crates/argo-cli/src/handlers/gateway/mod.rs` — narrowly: 1 `mod` line.
- `crates/argo-cli/src/cli.rs` — narrowly: add `SkillQuality` variant.
- `crates/argo-cli/tests/skill_quality_view.rs` (NEW, ≤ 250 LOC).

## Files you must NOT touch

- Existing `skill_quality_list.rs` — separate command, separate scope.
- Migration files — table already exists.
- `crates/argo-skill/src/quality.rs` — read-only consumer; no new fields.

## Acceptance criteria

- [ ] `argo skill quality <id>` returns latest score (markdown + JSON).
- [ ] `argo skill quality <id> --limit 10` returns last 10.
- [ ] `GET /skills/:id/quality` returns JSON with same shape.
- [ ] Skill not found → 404 (gateway), non-zero exit + error (CLI).
- [ ] No quality history → empty `history: []`, latest is `null`, status 200.
- [ ] 6+ tests: latest, history, not-found, empty history, gateway happy, gateway 404.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] Every authored file < 500 LOC.

## Branch + PR

- Branch: `claude-skill-quality-score-vis`
- PR title: `skills: surface quality score in CLI + gateway (claude-skill-quality-score-vis)`

## Notes

- The other skill_quality_* commands continue to work — this brief adds the
  single-skill detail view, not a refactor.
- Future work: add a small dashboard panel pulling from the new gateway route;
  out of scope for this brief.
