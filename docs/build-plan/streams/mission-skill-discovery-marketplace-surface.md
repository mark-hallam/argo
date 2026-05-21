# mission-skill-discovery-marketplace-surface

**Authored against**: `origin/main` `93c013f8` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-skill-discovery-marketplace-surface`.

## Summary

Auditor's 2026-05-21 baseline flagged mission claim 4 as the largest substrate-vs-surface gap: skill-library worker-load is LIVE (#1682), but the **learning loop and cross-tenant trust+provenance are not yet wired into the UI**. The marketplace `/skills` page (locked A1 design with hero-search per operator memory) exists but doesn't surface (a) workers learning new skills over time, (b) skill provenance — which tenant authored, what trust grade, and (c) cross-tenant adoption signals.

This brief wires the existing substrate to the existing locked surface so claim 4 is **visibly** real, not just substrate-real.

## Acceptance

- [ ] `/skills/[skillId]` worker-detail card includes a "Workers using this skill" section sourced from real `worker_skill` table (existing migration 101).
- [ ] Each skill card surfaces (a) author tenant (anonymized as "Authored by SKYCOT" if shipped-in or "Authored by your org" if locally-authored), (b) trust grade (current canonical type from #1949 — read the canonical post-#1949 surface), (c) cross-tenant adoption count if the skill is sharable (existing share-or-private flag).
- [ ] "Learning loop" indicator on worker-detail pages: each skill the worker has acquired since hire shows a small chip "Learned 2 weeks ago" (sourced from the `worker_skill.acquired_at` column if it exists; if not, use `created_at`).
- [ ] Marketplace search filter "Skills my workers learned this month" works via the `worker_skill.acquired_at` filter.
- [ ] No regression on the locked A1 marketplace design (operator memory `project_skills_page_design_locked.md`) — purely additive UI inside the locked register.
- [ ] At least one integration test covering the learning-loop chip rendering on a worker-detail page.
- [ ] Plain-language copy: "Learned" not "Acquired", "Used by" not "Adopted by" — operator's plain-language preference.

## May touch

- `web/app/(app)/skills/[skillId]/page.tsx` and any owned skills components
- `web/app/(app)/workers/[workerId]/skills/page.tsx` (or wherever worker-skills tab lives)
- `web/lib/api/skills.ts` (gateway client)
- `crates/argo-gateway/src/routes_skills.rs` (or equivalent) — additive endpoint(s)
- `crates/argo-store/migrations/` — only if `worker_skill.acquired_at` does NOT already exist (additive migration via [docs/skycot/operator/migrations.md](docs/skycot/operator/migrations.md))
- New integration test under the gateway crate's `tests/`

## Must NOT touch

- The skill-library substrate (#1682 — that's settled).
- The trust-engine internals (#1949 — that's settled).
- The locked A1 register (hero-search pattern, gradient icon, Audiowide marketplace title — `project_skills_page_design_locked.md`).
- Onboarding / hire / approvals surfaces.

## Notes

- Auditor evidence: `docs/skycot/audit/scheduled/LATEST.md` claim-4 row — "substrate complete (skill-library worker-load LIVE per #1682); learning loop and cross-tenant trust+provenance not yet wired into UI". This brief closes that gap.
- Pre-flight: grep for the canonical types — `rg 'pub (struct|enum) (Skill|WorkerSkill|TrustGrade)' crates/` — and consume them. Per [feedback_brief_authoring_preflight_rules.md], re-use, don't redeclare.
- Sizing: 600–1000 LOC. Backend additive endpoint + UI wiring + 1 integration test.
- This is genuine mission-claim-4 surface work — high leverage for the v1.0 demo narrative.
