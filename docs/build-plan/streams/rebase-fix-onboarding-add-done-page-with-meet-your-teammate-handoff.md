# rebase-fix-onboarding-add-done-page-with-meet-your-teammate-handoff

**Owner**: claude.
**Authored against**: origin/main `a5df90c41` on `2026-05-29`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh rebase-fix-onboarding-add-done-page-with-meet-your-teammate-handoff`.
**Supersedes**: PR #2486 (closed for re-author 2026-05-29) + original brief `streams/fix-onboarding-add-done-page-with-meet-your-teammate-handoff.md`.

⚪ **P3 — No celebration moment between `/onboarding/first-hire` submit and `/workers/<id>`. Brief is still load-bearing; PR #2486's approach can mostly be salvaged but `first-hire/page.tsx` has drifted via multiple merged PRs.**

## What changed on main since PR #2486 was authored

PR #2486's may-touch list:
- `web/app/(setup)/onboarding/done/page.tsx` (NEW — does not exist on main, no conflict)
- `web/app/(setup)/onboarding/done/__tests__/page.test.tsx` (NEW — no conflict)
- `web/app/(setup)/onboarding/first-hire/page.tsx` — touched by 6 merged PRs since authoring:
  - `7ab89623b` (#2619 — parameterise example prompt by vertical)
  - `58ea1c93f` (revert canonical-picker)
  - `718fb9f6d` (canonical-picker — later reverted)
  - `16aaab0ec` (#2504 — wizard polish cluster)
  - `ec4de733b` (#2477 — persist firstHirePrompt + provider key)
  - `7e4a8ddc5` (#2493 — soften copy)
- `web/__tests__/onboarding/wizard.test.tsx` — touched by `140ed52e4` (#2466)

The conflict is small but real: the original PR redirected from `first-hire/page.tsx` to `/onboarding/done?worker=<id>&name=<workerName>` on hire success; current main redirects directly to `/workers/<id>`. PR #2486 also updates `wizard.test.tsx` to assert the new redirect target.

## What this brief MUST ship

Re-implement PR #2486's full posture against current `origin/main` HEAD:

1. **New celebration page** at `web/app/(setup)/onboarding/done/page.tsx`:
   - Reuses `WizardEditorialShell` chrome
   - Eyebrow `MEET YOUR FIRST TEAMMATE`
   - Hero `<Name> is ready.` (gradient on the name)
   - Primary CTA `Meet <Name> →` → `/workers/<id>`
   - Secondary `View team →` → `/team`
   - 6-second `setTimeout` auto-advances to `/workers/<id>` if the operator freezes; clicking the primary CTA cancels the timeout
   - `sessionStorage` breadcrumb keyed by worker id prevents the celebration from re-firing if the user hits back to the same URL in the same tab
2. **Redirect change** in `web/app/(setup)/onboarding/first-hire/page.tsx`:
   - Current state: `router.push(\`/workers/\${res.workerId}\`)` at line ~78
   - New: `router.push(\`/onboarding/done?worker=\${res.workerId}&name=\${encodeURIComponent(workerName)}\`)`
   - Read `workerName` from the response shape (`useFirstHire` mutation result)
   - Do NOT touch the existing error banner / canonical-picker / vertical-example-prompt code paths
3. **Test wiring** at `web/app/(setup)/onboarding/done/__tests__/page.test.tsx`:
   - 6+ new tests pinning the celebration page behavior
4. **Wizard regression test** at `web/__tests__/onboarding/wizard.test.tsx`:
   - Pre-existing test asserts the OLD redirect target (e.g. `/workers/wkr_maya_1`)
   - Update strictly the assertion line(s): expect `/onboarding/done?worker=...&name=...`
   - Do NOT introduce any other shape change to that file

## Acceptance criteria

- [ ] `bash scripts/check-brief-freshness.sh rebase-fix-onboarding-add-done-page-with-meet-your-teammate-handoff` passes.
- [ ] `/onboarding/done?worker=<id>&name=<name>` renders with the eyebrow + hero + 2 CTAs.
- [ ] 6-second auto-advance fires when operator freezes; primary CTA cancels it.
- [ ] `sessionStorage` breadcrumb prevents re-fire on back-nav.
- [ ] `first-hire/page.tsx` submit routes through `/onboarding/done` (not directly to `/workers/<id>`).
- [ ] Vitest pin for `/onboarding/done/__tests__/page.test.tsx` passes (≥6 tests).
- [ ] Wizard regression test updated to assert the new redirect target.
- [ ] `pnpm typecheck` + `pnpm lint` clean.
- [ ] All new files under 500 LOC.

## May-touch

- `web/app/(setup)/onboarding/done/page.tsx` (NEW)
- `web/app/(setup)/onboarding/done/__tests__/page.test.tsx` (NEW)
- `web/app/(setup)/onboarding/first-hire/page.tsx` (REDIRECT LINE ONLY — be surgical)
- `web/__tests__/onboarding/wizard.test.tsx` (ASSERTION LINE ONLY)

## Out of scope

- Per-keystroke persistence of `firstHirePrompt` (already shipped via #2477).
- First-hire celebration timing / motion polish (this brief is the minimal celebration handoff).
- Touching the canonical-roster picker work (reverted via `58ea1c93f` — do not re-introduce).
- Renaming or restructuring the wizard chrome.

## Effort

~317 LOC across the new page + tests + 2 surgical line edits. ~1.5 hours.

## Source

- Original brief: `streams/fix-onboarding-add-done-page-with-meet-your-teammate-handoff.md`
- Closed PR: #2486 (closed 2026-05-29 as REAL-CONFLICT-NEEDS-REAUTHOR; first-hire/page.tsx has drifted significantly via 6 merged PRs; surgical line-edit re-application is cleaner than a rebase)
- UX audit: `docs/skycot/ux-audits/2026-05-27-0739/onboarding-hire-an-ai-teammate.md` F14
