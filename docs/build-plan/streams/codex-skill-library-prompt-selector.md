# codex-skill-library-prompt-selector — Add relevance/trust/budget caps to library prompt skills (F-11 P2)

**Owner**: codex. **Depends on**: nothing.
**Authored against**: origin/main `58e853b6` on `2026-05-16`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-skill-library-prompt-selector`.
**Source**: F-11 P2 in `docs/skycot/audit/2026-05-16-one-shot-architecture-audit.md`.

## Mission alignment

- **Claim 4 (skills shared and learned):** Workers draw from a shared library WITH PROVENANCE AND TRUST (`docs/skycot/MISSION.md:36`). The current implementation does NOT use trust policy for marketplace skills — only provenance for private rows.
- **CLAUDE.md drift rule:** Public surfaces need named consumers and should avoid parallel/dead scaffolds (`CLAUDE.md:169`).

## Scope

P2 trust + relevance. The skill-library ship-blocker fix landed (#1682), but the prompt selection is too broad:

`load_prompt_skills_with_library` (`crates/argo-agent-runner/src/turn_helpers.rs:61-68`):
- Lists library/workspace skills for action `"low"` blanket
- Filters private library rows by provenance (`:65`) — correct
- Filters workspace skills by vertical only (`:68`)
- **No action-type filter, no vertical match for marketplace library, no tenant trust policy, no prompt budget, no relevance cap**

Failure mode (escalating with marketplace growth): prompt pollution; over-broad skill exposure (e.g., a sales-vertical worker getting marketing-vertical skills); unbounded prompt LOC eating into context budget.

Scope:
- Add a `SkillRelevanceSelector` that filters library skills by:
  - Action-type match (the skill's declared action types intersect the current action)
  - Vertical match (the skill's vertical matches the worker's vertical, OR is explicitly tagged cross-vertical)
  - Tenant trust policy (`TrustGrade` of the skill ≥ tenant's minimum threshold)
  - Tag intersection (skill tags relevant to current task — light heuristic, not LLM-based)
- Cap rendered marketplace skills (operator-configurable, default 5 max, prompt-budget aware)
- Wire into `load_prompt_skills_with_library`
- Add tests:
  - Worker in vertical X does NOT see published marketplace skill bound to vertical Y
  - Skill with `TrustGrade::Untrusted` does NOT appear when tenant policy requires `Verified+`
  - Cap of 5 means 6th skill is dropped (by recency or rating)

LOC: ~280.

## Files you may touch

- `crates/argo-skill/src/relevance_selector.rs` (NEW) — `SkillRelevanceSelector` with the 4-axis filter + cap. ≤180 LOC.
- `crates/argo-skill/src/lib.rs` — export. ≤10 modified lines.
- `crates/argo-agent-runner/src/turn_helpers.rs` — call `SkillRelevanceSelector::select` after the existing list (`turn_helpers.rs:61-68`). ≤60 modified lines.
- `crates/argo-config/src/sections.rs` — `[skill_library.relevance]` config block (max_marketplace_skills, min_trust_grade). ≤30 modified lines.
- `crates/argo-skill/tests/relevance_selector_integration_tests.rs` (NEW) — 3 regression tests. ≤120 LOC.

## Files you must NOT touch

- `crates/argo-skill/src/library.rs` — `LayeredLookup::with_library` is the storage layer; this brief is the SELECTION layer above it.
- The session-attached-skill path (different code path, not affected).
- `TrustGrade` enum definition — consumed, not modified.

## Acceptance criteria

- [ ] `SkillRelevanceSelector` exists in argo-skill with 4-axis filter (action-type, vertical, trust, tags) + cap.
- [ ] `load_prompt_skills_with_library` applies the selector.
- [ ] Config block defaults: `max_marketplace_skills = 5`, `min_trust_grade = Verified`.
- [ ] Regression test: cross-vertical marketplace skill excluded.
- [ ] Regression test: untrusted skill excluded when policy requires Verified+.
- [ ] Regression test: 6th skill dropped at cap (by lowest rating).
- [ ] `cargo test -p argo-skill -p argo-agent-runner relevance` passes.
- [ ] `cargo clippy -p argo-skill -p argo-agent-runner -p argo-config --all-targets -- -D warnings` clean.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `codex-skill-library-prompt-selector`
- PR title: `feat(skill): relevance/trust/budget selector for library prompt skills (codex-skill-library-prompt-selector) — F-11 P2`

## Notes for the implementer

- The selector should be CONSERVATIVE in v1 — defaults that admit fewer skills are better than defaults that admit too many. Operators can loosen via config.
- Tag heuristic: simple lowercase intersection is sufficient for v1. LLM-based relevance can come later.
- Cap-tie-breaking: prefer higher `TrustGrade` first, then higher rating, then more recent. Document this in code.
- After this brief, Claim 4's "provenance and trust" half is actually enforced.
