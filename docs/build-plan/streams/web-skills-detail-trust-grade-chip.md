# web-skills-detail-trust-grade-chip

**Authored against**: `origin/main` `652a1825` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-skills-detail-trust-grade-chip`.

## Summary

Smallest-possible slice of `mission-skill-discovery-marketplace-surface`. Adds ONE chip to `/skills/[skillId]` showing the skill's canonical trust grade (consumed from `argo-trust-engine` — landed via #1949). This is the visible-substrate-proof bite for claim 4 ("skills shared + learned") — workers can pick this without taking on the full marketplace-surface scope.

Pre-condition: **#1949 MERGED ✅** (trust-engine canonical types on main).

## Acceptance

- [ ] `/skills/[skillId]` renders a trust-grade chip alongside the existing skill metadata. Chip shows letter grade (A / B / C / etc.) with the canonical color mapping from `argo-trust-engine` (read the canonical color/grade table; do not redefine).
- [ ] Chip sources its value from the gateway (existing skill endpoint augmented to return `trust_grade`, OR an additive endpoint — pick whichever is fewer LOC).
- [ ] Fallback: if no trust data is available for a skill, chip renders "Ungraded" in muted gray.
- [ ] No regression on the locked A1 marketplace register (operator memory `project_skills_page_design_locked.md`).
- [ ] One unit/component test: chip renders with grade `A`, `B`, `Ungraded`.

## May touch

- `web/app/(app)/skills/[skillId]/page.tsx` (or wherever the detail page renders)
- `web/components/skycot/skills/TrustGradeChip.tsx` (new file, or co-located helper)
- `web/lib/api/skills.ts` (additive — pass through new field)
- `crates/argo-gateway/src/routes_skills.rs` (or equivalent) — additive field on response struct

## Must NOT touch

- `argo-trust-engine` internals — read from its public surface only.
- The skill-library substrate.
- Other surfaces (workers detail, cohorts, audit) — that's the broader brief's scope.

## Notes

- Pre-flight: `rg 'pub enum TrustGrade' crates/` to find the canonical enum from #1949; consume it via the gateway response shape.
- Sizing: 100–200 LOC. Small + visible — high "demo charm" leverage.
- This brief is **paired** with `mission-skill-discovery-marketplace-surface` — that one carries the broader scope (workers using this skill, author tenant, adoption count, learning-loop chip on worker pages). Workers may pick either; both can land independently.
