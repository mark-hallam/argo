# rebase-fix-onboarding-provider-key-demo-mode-mask-and-skip-affordance

**Authored against:** origin/main `a5df90c41` on 2026-05-29 by claude deep-triage-agent-3.
**Validate before claim:** `bash scripts/check-brief-freshness.sh rebase-fix-onboarding-provider-key-demo-mode-mask-and-skip-affordance`

## What

Re-implement the original brief `streams/fix-onboarding-provider-key-demo-mode-mask-and-skip-affordance.md` against current `origin/main` HEAD.

The original PR #2472 (claude) was authored against an older `origin/main` and conflicts irretrievably with main's evolution on `web/app/(setup)/onboarding/connect-provider/page.tsx`:

- 5 merged PRs since #2472 opened: #2504 wizard polish, #2477 step-back persistence, #2578 Continue as primary CTA, #2551 dedupe per-step demo banners against global shell banner, #2534 shape-honest copy ("$0.001 test call" claim removed).

Current shape on main (verify with `cat web/app/(setup)/onboarding/connect-provider/page.tsx` before authoring):

- `demoMode` already gates a sub-text "Workers run with a mock provider…type any sk-ant- key" (lines 199-203).
- A separate `onboarding-provider-skip-warning` role=note block at lines 271-280 already says "Mock provider — workers run but responses are simulated. Skipping uses a mock provider until you add a real key in Settings · Providers."
- The "$0.001 test call" footer claim was REMOVED by #2534 — do NOT re-introduce it.
- The Continue button became the primary CTA via #2578 — preserve that ordering.
- Per-step demo banners are dedupe'd against the global shell banner by #2551 — any NEW above-input banner the rebase adds MUST NOT collide with that dedupe (use a distinct test-id and a different render condition so the global banner's dedupe logic still works).

The original brief's intent (UX-AUDIT-T0739MEL F11 P2): the entry-time affordance should signal mock-mode BEFORE submit so the buyer doesn't first see the truth in the success banner. After #2504+#2551 the demo affordance is now substantially better on main than at PR-open time. Re-evaluate whether the original brief's full ask is still needed, or if only the **placeholder swap** (`sk-ant-...` → `sk-ant-anything-works-here-in-demo`) is the surviving load-bearing piece.

## Acceptance (minimal — re-evaluate first)

The bare-minimum load-bearing pieces of the original brief:
1. **Placeholder swap** in demoMode: `placeholder="sk-ant-..."` → `placeholder="sk-ant-anything-works-here-in-demo"`. (Tiny, ~3 LOC. Almost certainly still load-bearing.)
2. **Skip-button visual promotion** (e.g. swap class from `.ob-cta-ghost` to a more visible variant when `demoMode`): re-evaluate against #2578 which made Continue the primary CTA — promoting Skip MAY collide with that decision; verify with operator if Skip should now be co-equal in demoMode.

Drop the original brief's amber above-input banner add if it would collide with the #2551 dedupe — the existing demoMode sub-text on lines 199-203 already serves the "tell the buyer it's mocked" job.

Closes UX-AUDIT-T0739MEL F11 (P2), or downgrade to PARTIALLY-FIXED via #2504+#2551+#2534 if the residual gap (placeholder + skip promotion) is too small to bother shipping.

## Sensible deviations

- Preserve #2534 "no $0.001 claim".
- Preserve #2578 Continue as primary CTA.
- Preserve #2551 demo-banner dedupe — don't add a new banner that re-introduces the duplicate.
- Preserve #2477 step-back persistence (don't clobber sessionStorage logic).

## May-touch

- `web/app/(setup)/onboarding/connect-provider/page.tsx`
- `web/app/(setup)/onboarding/connect-provider/__tests__/page.test.tsx`

## Verify

- `pnpm --dir web exec vitest run app/\(setup\)/onboarding/connect-provider`
- `pnpm --dir web typecheck`
