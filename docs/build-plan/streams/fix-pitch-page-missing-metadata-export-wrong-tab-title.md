# fix-pitch-page-missing-metadata-export-wrong-tab-title

**Priority**: P2 — the `/pitch` page (the v1.0 demo headline, Portia's rehearsal surface) has no `metadata` export, so its browser-tab `<title>` falls back to the parent template and reads wrong/empty on camera. Caught by the real `scripts/check-route-metadata.sh` lint (the 2 remaining reds in PR #2667's test-health pass, which correctly flagged rather than masked it).
**Source**: 2026-05-29 flagged by PR #2667 (fix-stale-web-tests) per its hard rule "flag production gaps, don't touch production."
**Authored against**: origin/main `5ffb42917` on 2026-05-29
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-pitch-page-missing-metadata-export-wrong-tab-title`
**Authored by**: claude conductor (interactive session)

## The gap

`__tests__/lint/route-metadata.test.ts` runs `scripts/check-route-metadata.sh` against `web/` and expects every app route to export metadata. `/pitch` does not, so the lint is red (2 cases) and the tab title is wrong during demo recording. `web/app/(app)/pitch/page.tsx` is a `"use client"` component (it calls the pitch API + manages chat state), so it CANNOT export `metadata` directly — the canonical Next.js App-Router fix is a sibling `layout.tsx` with a static `metadata` export (same pattern already used at `web/app/(app)/workers/[workerId]/overview/layout.tsx` and shipped via #2451 for workflow/skill routes).

## Scope / fix

- Add `web/app/(app)/pitch/layout.tsx` exporting `metadata` (title + description + OG), e.g. `title: "Pitch rehearsal"` so the parent template renders `Pitch rehearsal · SKYCOT`. Use Portia's framing per DEC-016 (the Pitch Agent is Portia, female) — e.g. description "Rehearse with Portia, the truth-bounded Pitch Agent."
- The layout just wraps `children`; no behavior change to the client page.

## Acceptance

- [ ] `web/app/(app)/pitch/layout.tsx` exports `metadata`; `/pitch` tab title resolves to `Pitch rehearsal · SKYCOT` (not the bare parent template).
- [ ] `__tests__/lint/route-metadata.test.ts` → 0 failures (the 2 flagged reds clear).
- [ ] `pnpm exec vitest run __tests__/lint/route-metadata.test.ts` green; `pnpm exec tsc --noEmit` introduces no new errors.
- [ ] No change to `pitch/page.tsx` behavior.

## Out of scope

- Renaming/re-theming the pitch page; any pitch API change.

## May-touch

- `web/app/(app)/pitch/layout.tsx` (NEW)

## Operator authority

Web-only, 1 new file — eligible for the web-only admin-merge lane with vitest green in the PR body per the 2026-05-26 authorization.
