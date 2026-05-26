# fix-skills-active-view-source-toast-stub-or-navigate

**Owner**: claude.
**Authored against**: origin/main `80e149df55f6` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-skills-active-view-source-toast-stub-or-navigate`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-1028/scene-4-and-5-claim-4-and-5.md` F16 [P2].

🟡 **P2 — Worker skills tab "View source" button only fires an announcement toast. An investor clicking it sees "Skill source viewer ships with the marketplace surface" — reads as half-built.**

## Root cause

`web/app/(app)/workers/[workerId]/skills/page.tsx:53-55` — `onViewSource` is a toast stub. `web/components/skycot/worker/SkillsActiveCard.tsx:97-101` renders the button regardless.

## Findings this brief closes

- scene-4-and-5-claim-4-and-5 F16 [P2] — "View source" toast stub looks half-built

## Fix recipe

Pick ONE.

**(a) Navigate to the skill-detail page** on click (preferred):
1. Update `onViewSource` to `router.push(/skills/${skillName})`.
2. `/skills/<id>` already renders Provenance + Workers-using sections; that's the natural source view.

**(b) Hide the button** until a dedicated source viewer ships:
1. Conditional render based on a feature flag.
2. Reduces the half-built feel; reverts when the viewer lands.

Prefer (a) — navigates to existing surface; zero UI work needed downstream.

## Acceptance criteria

- [ ] Click on "View source" navigates to `/skills/<id>` (no toast).
- [ ] Detail page renders the skill's full provenance.
- [ ] No regression on the active-skills list rendering.

## May-touch

- `web/app/(app)/workers/[workerId]/skills/page.tsx:53-55` (handler)
- `web/components/skycot/worker/SkillsActiveCard.tsx:97-101` (verify)
- `web/tests/worker-skills-view-source.test.ts` (NEW)

## Out of scope

- New source-code viewer (post-v1.0)
- Skill versioning UI

## Effort

~15-25 LOC + 1 test. ~30min.
