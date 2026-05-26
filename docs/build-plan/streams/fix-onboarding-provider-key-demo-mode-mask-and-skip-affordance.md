# fix-onboarding-provider-key-demo-mode-mask-and-skip-affordance

**Owner**:
**Authored against**: origin/main `354de50a0c5fdf561125c7e931699fe94daa326d` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-onboarding-provider-key-demo-mode-mask-and-skip-affordance`.
**Source**: e2e-ux-audit T0739 — `docs/skycot/ux-audits/2026-05-27-0739/onboarding-hire-an-ai-teammate.md` F11.

🟡 **P2 — `/onboarding/connect-provider` accepts any string beginning `sk-ant-` and validates it green after a 400ms `setTimeout` in demo mode. A CTO-type viewer who types a real (or plausible-looking) key watches it "validate" with no Anthropic round-trip and no visible "we did NOT touch your key" affordance. The hard front-end shape check is the only gate; the UI doesn't tell the story of "this is a mock provider — paste anything".**

## Root cause

`web/lib/onboarding/use-provider-key.ts:56-59` in demo mode returns `{ validated: true, mockProvider: true }` after `setTimeout(400)` for any input passing the regex shape check at `web/app/(setup)/onboarding/connect-provider/page.tsx:102-108` (`/^sk-ant-/`). The success banner copy at lines 200-202 is honest ("Mock provider connected — workers run with simulated responses for the demo. Forwarding…") but it fires AFTER the user has already entered what may look like a real key. The input affordance during entry doesn't communicate that the field is mocked.

There IS a `Skip — use mock provider` button (already exists in the page; line approximate `connect-provider/page.tsx:217`). It just isn't surfaced prominently enough for a savvy user to notice before they paste a key.

This is the natural complement to the F1 hire-flow honesty fix: the connect-provider step already lands the green honest banner, but the entry-time affordance is still theatrical. Audit auditor flagged this specifically because the asymmetry between the two demo-mode messages reads as "they paint over the fake hire-flow because they understand demo-mocking, but the provider step pretends harder than it should".

## Findings this brief closes

- onboarding-hire-an-ai-teammate F11 [P2] — Provider key field accepts any `sk-ant-` string in demo mode with no validation feedback delta

## Fix recipe

1. **Detect demo mode in the connect-provider page** at `web/app/(setup)/onboarding/connect-provider/page.tsx`. Read `isDemoMode()` from `web/lib/skycot/demo-mode.ts`.
2. **Render an above-input demo-mode banner** when `isDemoMode()` is true:
   - Yellow/amber affordance (existing `Callout` component with `variant="info"` or similar).
   - Copy: "**Demo · mock provider** — paste anything starting with `sk-ant-`. Your real key stays on your machine; nothing is sent to Anthropic during demo recording. Skip this step to use the mock provider directly."
3. **Replace the input placeholder** in demo mode with `sk-ant-anything-works-here-in-demo`. The placeholder itself signals the field is mock.
4. **Promote the `Skip — use mock provider` button** to sit immediately under the input (not buried below the primary submit). Demo-mode default visual weight equal to or greater than the primary submit.
5. **Optional**: in demo mode, change the primary submit label from "Connect" to "Connect (mock)" so the button itself reads as honest.

## Acceptance criteria

- [ ] In demo mode, `/onboarding/connect-provider` renders an above-input banner explaining the field is mock.
- [ ] In demo mode, the input placeholder reads `sk-ant-anything-works-here-in-demo` (or operator-approved equivalent).
- [ ] The "Skip — use mock provider" button is visually equal-weighted with the primary CTA in demo mode.
- [ ] In non-demo mode, the page is unchanged (no regression).
- [ ] Vitest pin: when `isDemoMode()` returns true, the banner element is in the DOM.

## May-touch

- `web/app/(setup)/onboarding/connect-provider/page.tsx` (banner + placeholder + button weight)
- `web/components/skycot/Callout.tsx` (no change expected; just consume)
- `web/app/(setup)/onboarding/connect-provider/__tests__/page.test.tsx` (new pin)

## Out of scope

- Renaming the existing post-submit "Mock provider connected" success banner (it's already correct)
- The broader F1 hire-flow honesty (covered by `fix-demo-first-hire-discards-prompt-and-redirects-to-sample-worker.md`)
- The provider-key 500 friendly-error fix (covered by `fix-onboarding-provider-key-500-friendly-error.md`)
- Multi-provider copy (covered by `fix-onboarding-connect-provider-multi-provider-copy-or-options.md`)

## Effort

XS — ~40-60 LOC + pin. ~1 hour. P2 because the post-submit banner already lands honestly; this brief upgrades the pre-submit affordance to match.
