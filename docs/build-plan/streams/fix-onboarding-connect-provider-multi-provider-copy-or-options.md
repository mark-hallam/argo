# fix-onboarding-connect-provider-multi-provider-copy-or-options

**Owner**: claude or codex.
**Authored against**: origin/main `d15461a3a` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-onboarding-connect-provider-multi-provider-copy-or-options`.
**Source**: e2e-ux-audit Agent (onboarding-hire-an-ai-teammate) finding F10 (`docs/skycot/ux-audits/2026-05-25-2309/onboarding-hire-an-ai-teammate.md`).

🟢 **P2 — Wizard locks the operator into Anthropic; `/landing` hero claims "provider routing" as a core capability. Marketing → product gap is visible.**

## Root cause

`/onboarding/connect-provider` only offers Anthropic ("sk-ant-..." input). The hero on `/landing` claims "provider routing" as a core capability, implying multi-provider choice.

Operator-locked decisions (per CLAUDE.md "v1.0 strategic decisions"): v1 ships `model=latest` single-provider — operator chose Anthropic-only for v1. The wizard reflects reality; the marketing copy promises a v2 capability.

P2 because the operator's decision is explicit and the gap is forward-looking. The brief is about closing the marketing-vs-product gap so a buyer reading the hero doesn't expect a multi-provider chooser in the wizard.

## Findings this brief closes

- onboarding-hire-an-ai-teammate F10 [P2] — Onboarding "Connect Anthropic" is provider-monolithic; landing claims "provider routing"

## Fix recipe

Two options; pick (a) for the smaller patch:

**(a) Soften the landing copy**:
1. Touch site: `web/app/landing/page.tsx` — the "provider routing" hero claim or features section.
2. Reword to one of:
   - "Anthropic Claude · multi-provider routing in v2"
   - "Claude-powered today · OpenAI, Bedrock, Vertex coming"
   - Drop "provider routing" from the hero and move it to a "Roadmap" section.
3. Audit all marketing pages (`/`, `/landing`, `/customers`, `/compare`) for similar overpromises on provider routing.

**(b) Add a smaller chooser to the wizard** (more work, ships v1 with stub options):
1. Touch site: `web/app/(setup)/onboarding/connect-provider/page.tsx`.
2. Show 3 cards: Anthropic (active), OpenAI (greyed `coming-soon`), Bedrock (greyed `coming-soon`).
3. Only Anthropic accepts input; the others render `"Available in v2"` tooltip.
4. Honest about scope without pretending.

Recommend (a) for the v1 demo lane. (b) is for v1.x once the second provider lands behind the manifest layer.

## Acceptance criteria

- [ ] `/landing` hero copy does NOT promise "multi-provider routing" as a v1 capability.
- [ ] Wizard connect-provider step stays Anthropic-only (no scope creep) OR shows greyed coming-soon cards.
- [ ] No regression on the existing "Skip — uses mock provider" affordance (the operator-stated honesty preservation).

## May-touch

- `web/app/landing/page.tsx`
- `web/app/(setup)/onboarding/connect-provider/page.tsx` (only if option b)
- Other marketing pages with provider-routing claims (`/`, `/customers`, `/compare`)

## Out of scope

- Building the second provider adapter (separate v1.x work)
- Provider-keys settings page completion (separate brief — `/settings/provider-keys` is "Coming soon" today)

## Effort

~15-50 LOC for option (a); ~80-150 LOC for option (b). ~30 min - 2 hours.
