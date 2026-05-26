# fix-skills-canvas-flat-route-404-or-redirect

**Owner**: claude.
**Authored against**: origin/main `56f4ea0b4d84ebf52c14e387d95c4e99e1cca0b7` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-skills-canvas-flat-route-404-or-redirect`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-27-0415/whole-site-mvp-claims-coverage.md` F19 [P1].

🟠 **P1 — `/skills-canvas` is a 404 — no top-level route exists. The canvas only lives at `/workers/[workerId]/skills/[skillId]/canvas`. Multiple briefs in `streams/` AND the operator's e2e-agent system prompt reference `/skills-canvas` as if it were a marketplace-level surface. So at least one brief points users at a 404.**

## Root cause

`grep -rn "skills-canvas" docs/build-plan/streams/` (sample audit step) found brief references to a `/skills-canvas` URL — but `web/app/(app)/skills-canvas/page.tsx` does NOT exist. The canvas surface only exists at the nested per-worker / per-skill path:

```
web/app/(app)/workers/[workerId]/skills/[skillId]/canvas/page.tsx  ← actual route
web/app/(app)/skills-canvas/page.tsx                               ← does NOT exist
```

Two possible intents behind the `/skills-canvas` references:

1. **The flat route was planned but never built.** A marketplace-level "open the skill canvas without picking a worker" affordance was scoped but didn't ship.
2. **The references are stale and the canvas was always nested.** The flat URL is a typo / artefact of an old refactor.

Either way, operator-facing docs + briefs that point at `/skills-canvas` produce a 404 when followed.

## Findings this brief closes

- whole-site-mvp-claims-coverage F19 [P1] — `/skills-canvas` is a 404 even though briefs reference it

## Fix recipe

Two acceptable approaches, operator-decisional:

### Option A — implement the flat route as a worker-picker → canvas redirect (if the flat route was intended)

Add `web/app/(app)/skills-canvas/page.tsx` that prompts the user to pick a worker + skill, then redirects to the nested canvas:

```tsx
// /skills-canvas → "Pick a worker and skill to open in the canvas"
// On click → router.push(`/workers/${workerId}/skills/${skillId}/canvas`)
```

Slim shell, ~50-80 LOC. Closes the dead-link gap without committing to a parallel canvas surface.

### Option B — remove stale references (if `/skills-canvas` was always a typo)

1. `grep -rn "skills-canvas" docs/ web/ crates/ scripts/` — enumerate every reference.
2. For each reference, replace with `/workers/<workerId>/skills/<skillId>/canvas` (or the canonical path).
3. Audit `agents/e2e-ux-journey-test-agent` system prompt + remove any `/skills-canvas` mention.

Cheaper if confirmed stale. Operator + tech-lead decision.

### Option C — add a redirect rule (cheapest, preserves any in-the-wild links)

`web/next.config.ts` (or `next.config.js`) — add:

```ts
{
  source: "/skills-canvas",
  destination: "/skills",
  permanent: false,
}
```

Any visitor hitting `/skills-canvas` lands on the marketplace. Doesn't address the underlying ambiguity (canvas is nested only), but kills the 404.

**Recommend B** — confirm if the flat route was intended; if not, scrub the stale references. Cheapest correct fix.

## Acceptance criteria

- [ ] Visiting `/skills-canvas` does NOT 404 (either renders something useful per Option A/C, OR the link source is fixed so no one hits `/skills-canvas` anymore per Option B).
- [ ] `grep -rn "skills-canvas" docs/ web/` returns either zero hits (Option B) or only legitimate ones (Option A/C).
- [ ] The e2e-ux-journey-test-agent system prompt is correctly grounded against the actual route.
- [ ] No broken doc-link in any active brief.

## May-touch

- `web/app/(app)/skills-canvas/page.tsx` (Option A — NEW)
- `web/next.config.ts` (Option C — redirect rule)
- `docs/build-plan/streams/*` (Option B — scrub stale references)
- `agents/e2e-ux-journey-test-agent/system-prompt.md` (Option B — verify path; scrub)

## Out of scope

- Marketplace search / install UX (covered by sibling skills briefs)
- Worker-detail skills tab (covered by `fix-worker-detail-responsibilities-schedules-tab-end-to-end`)
- Ghostwriter skill-authoring flow

## Effort

Option A: ~80-150 LOC. Option B: ~10-30 doc edits, ~30-60 min. Option C: ~5 LOC, ~10 min. P1 because doc-source-of-truth integrity matters for downstream agents.
