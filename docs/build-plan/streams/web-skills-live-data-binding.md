# web-skills-live-data-binding — wire /skills catalog to gateway skill registry

**Owner**: claude. **Sizing**: ~150-250 LOC (one useSkills hook + page swap).
**Authored against**: origin/main `3121e32e` on `2026-05-20`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-skills-live-data-binding`.
**Priority**: P2 — /skills currently shows 6 seed skills. Real data lives in `argo-skill::SkillStore` and the gateway exposes it via `/api/v1/skills`. This brief swaps seed → live data.

## Scope

### 1. New hook: `useSkills`
Path: `web/lib/hooks/use-skills.ts`
- `useSkills()` — fetch all skills via gateway
- TanStack Query key: `["skills"]`
- Mock fallback to existing seed if gateway empty

### 2. New API client: `web/lib/api/skills.ts`
- `fetchSkills()` — GET /api/v1/skills (returns list)
- Types match gateway response

### 3. Update `/skills/page.tsx`
Replace `FEATURED` array with `useSkills()`. Show count pill from real data ("N skills · M featured"). Loading + empty + error states.

## Acceptance criteria

- [ ] `useSkills()` hook lands and is unit-tested.
- [ ] Skills page renders real skills from the gateway.
- [ ] "Try" chips remain (curated suggested-searches; not from gateway).
- [ ] Cards link to `/skills/<skill_id>` (route doesn't exist yet; route fallthrough to a "Skill detail coming v1.x" stub).

## Out of scope

- Skill detail page (separate brief).
- Skill-to-worker assignment UX (covered by existing PM-track brief).
