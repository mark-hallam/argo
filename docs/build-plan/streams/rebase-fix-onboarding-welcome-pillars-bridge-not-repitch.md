# rebase-fix-onboarding-welcome-pillars-bridge-not-repitch

**Authored against:** origin/main `a5df90c41` on 2026-05-29 by claude deep-triage-agent-3.
**Validate before claim:** `bash scripts/check-brief-freshness.sh rebase-fix-onboarding-welcome-pillars-bridge-not-repitch`

## What

Re-implement the original brief `streams/fix-onboarding-welcome-pillars-bridge-not-repitch.md` against current `origin/main` HEAD.

The original PR #2533 (claude) was authored against an older `origin/main` and conflicts irretrievably with main's evolution on `web/app/(setup)/onboarding/welcome/page.tsx`:

- 4 merged PRs since #2533 opened: #2553 Genesis pillar reads as flagship (REWROTE the middle pillar text into the "Genesis · Onboarding Studio" flagship pitch — promoted, not removed); #2530 STEP N eyebrow alignment; #2466 singular "Hire your first teammate" hero + drop /genesis deep-link pillar; #2441 soften Phase 1 reality.

Current shape on main (verify with `cat web/app/(setup)/onboarding/welcome/page.tsx` before authoring):

- 3 pillars still present (lines 51-75): `Audit-first`, `Genesis · Onboarding Studio`, `Enterprise-grade`. The middle pillar was deliberately promoted by #2553 to be the flagship Genesis pitch — DELETING IT WOULD UNDO #2553.
- "Open Genesis studio →" ghost CTA added at the bottom by some recent PR (lines 90-95) — preserve.
- Hero is the singular "Hire your first teammate" per #2466 — preserve.

The original brief's intent (UX-AUDIT-T0119MEL F4, P2) was to bridge into doing rather than re-pitch /landing's value-prop strip. But #2553 explicitly decided the middle pillar IS the load-bearing Genesis pitch and should NOT be a value-prop repeat. The original brief is now PARTIALLY-SUPERSEDED.

## Re-evaluate intent before authoring

Before writing the rebase, the conductor/operator should decide:

- **Option (a) — Keep the 3 pillars (status-quo), close the brief as STALE-FIXED**: #2553 already addressed the "re-pitch" concern by re-pointing the middle pillar to Genesis (Skycot's distinguishing surface, not a recycled marketing claim). The remaining "Audit-first" + "Enterprise-grade" pillars ARE the load-bearing trust signals at this step of the funnel. Conclusion: drop the brief.
- **Option (b) — Replace ONLY the two repeat pillars (Audit-first + Enterprise-grade), keep the Genesis middle pillar**: a 2-bullet `Step N · time · task` preview replaces just the two echo pillars. The Genesis pillar stays. This is a smaller change than the original full-replacement and preserves #2553.

Recommend **Option (a)** (close as STALE-FIXED) unless the operator explicitly wants the bridge-into-doing affordance — the Genesis-flagship pillar IS the bridge.

## If Option (b) chosen — Acceptance

- `web/app/(setup)/onboarding/welcome/page.tsx`: keep the middle "Genesis · Onboarding Studio" pillar verbatim from #2553; replace ONLY the first (Audit-first) and last (Enterprise-grade) pillars with two preview tiles formatted as `Step 2 · Workspace · 30 sec` / `Step 4 · Vertical · 1 min` style.
- Vitest snapshot at `web/tests/onboarding-welcome.test.ts` + `web/components/auth/__tests__/WizardEditorialShell.onboarding.test.tsx`: update test-ids accordingly (`onboarding-pillar-audit-first` → `onboarding-step-preview-workspace`, etc.).
- Preserve #2530 STEP N eyebrow pattern.
- Preserve #2466 hero singular.
- Preserve the "Open Genesis studio →" ghost CTA.

Closes UX-AUDIT-T0119MEL F4 (P2) IF Option (b), or downgrade to STALE-FIXED-BY-#2553 if Option (a).

## May-touch

- `web/app/(setup)/onboarding/welcome/page.tsx`
- `web/components/auth/__tests__/WizardEditorialShell.onboarding.test.tsx`
- `web/tests/onboarding-welcome.test.ts`

## Verify

- `pnpm --dir web exec vitest run tests/onboarding-welcome.test.ts components/auth/__tests__/WizardEditorialShell.onboarding.test.tsx`
- `pnpm --dir web typecheck`
