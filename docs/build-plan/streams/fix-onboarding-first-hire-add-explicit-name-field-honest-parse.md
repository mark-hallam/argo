# fix-onboarding-first-hire-add-explicit-name-field-honest-parse

**Owner**: claude.
**Authored against**: origin/main `b77864a000a9738c80f235fc2605d192197cd5c7` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-onboarding-first-hire-add-explicit-name-field-honest-parse`.
**Source**: e2e-ux-audit T0819 — `docs/skycot/ux-audits/2026-05-27-0819/onboarding-hire-an-ai-teammate.md` F13 [P2].

🟡 **P2 — Step 5 says "Plain English is fine. Give them a **name, a role, and what they should do this week**." — but the only input is a multi-line textarea. The user has to embed the name in prose. Combined with F4 (the gateway just takes the first capitalized word), this is the load-bearing failure mode: "She's called Maya and she runs the blog" → worker named "She's".**

## Root cause

`web/app/(setup)/onboarding/first-hire/page.tsx:96-118` renders a single textarea (`first-hire-prompt`). The copy promises "give them a name, a role, and what they should do this week" — but there is no separate name input. The user types prose; the gateway extracts the first capitalized word as the name (per F4, covered by `fix-onboarding-first-hire-name-parser-prefers-named-pattern`).

The honest fix is one of:
- **(a)** Add a separate `name` input above the description textarea. User owns the name; the description is for charter/persona.
- **(b)** Keep the single-input form but ship server-side LLM-parse for name extraction. Bigger swing; depends on Genesis being live.

Per memory `feedback_genesis_is_ai_teammate_onboarding_studio`, Genesis IS the AI Teammate Onboarding Studio — voice/text role + SOP-PDF/wiki/Slack uploads + comprehension verification. So option (b) is the long-term direction. But Genesis isn't fully wired today; F1 of the audit shows the demo-mode first-hire still short-circuits to a sample persona.

Option (a) is the immediate honest fix and is consistent with the operator's "ideas-are-the-moat / function-vs-dressing" principle — don't promise plain-English parsing we haven't shipped. Once Genesis is end-to-end, the explicit name field can be repurposed as a "or paste a description" affordance with name auto-extraction.

## Findings this brief closes

- onboarding-hire-an-ai-teammate F13 [P2] — First-hire form lacks an explicit "name" field; the prompt is the only input

## Fix recipe

1. **Add a name input.** In `web/app/(setup)/onboarding/first-hire/page.tsx`, render an `<input>` above the textarea with placeholder "Maya" (or rotating per-vertical placeholder: Marcus / Priya / Jamal / Nori). Required field. Label: "Their name".
2. **Wire name to the hire request.** Pass `name` as a separate field in the `firstHire({ name, description, verticalId })` request. Gateway-side, accept the name as-is when supplied; only fall back to substring-extraction when `name` is empty.
3. **Update copy.** Change "Plain English is fine. Give them a **name, a role, and what they should do this week**." to "Their **role** and **what they'll do** — plain English." (since "name" now has its own input).
4. **Persist to draft store.** The Zustand draft store should include the name. Step-5 page reads draft.name on mount; user can edit either input.
5. **Vitest pin** at `web/app/(setup)/onboarding/first-hire/__tests__/page.test.tsx`: assert the form has both inputs; assert submit sends `{name, description, verticalId}`.

## Acceptance criteria

- [ ] `/onboarding/first-hire` renders a name input above the description textarea.
- [ ] Name is required; submit is disabled until name is non-empty.
- [ ] Submitting sends `name` as a separate field in the API call.
- [ ] Description placeholder + label no longer claim "give them a name" — only role + tasks.
- [ ] Draft persists name across reloads (same Zustand store).
- [ ] Vitest pin asserts the new shape.

## May-touch

- `web/app/(setup)/onboarding/first-hire/page.tsx` (form + copy)
- `web/lib/onboarding/use-first-hire.ts` (request shape)
- `web/lib/onboarding/draft-store.ts` (or wherever the Zustand draft lives)
- Gateway-side `first_hire.rs` (accept optional `name` field; fall back to extraction if absent) — small change
- `web/app/(setup)/onboarding/first-hire/__tests__/page.test.tsx` (new vitest)

## Out of scope

- Genesis end-to-end LLM parse (long-term roadmap; covered indirectly by `fix-onboarding-first-hire-invoke-genesis-plan`)
- Gateway's substring name-extraction heuristic (covered by `fix-onboarding-first-hire-name-parser-prefers-named-pattern`)
- Demo-mode short-circuit (covered by `fix-demo-first-hire-discards-prompt-and-redirects-to-sample-worker`)

## Effort

S — ~50-80 LOC SPA + ~10-20 LOC gateway. ~1-2 hours. P2 because the demo MP4 narration says "we hired Maya" — if the user types "Maya, marketing intern" then today's parser DOES extract "Maya" correctly, so the demo records; this fix is for the broader product credibility (any non-leading-capitalized-name prose breaks).
