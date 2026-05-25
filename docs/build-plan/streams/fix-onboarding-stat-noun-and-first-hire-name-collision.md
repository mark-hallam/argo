# fix-onboarding-stat-noun-and-first-hire-name-collision

**Owner**: claude or codex.
**Authored against**: origin/main `d15461a3a` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-onboarding-stat-noun-and-first-hire-name-collision`.
**Source**: e2e-ux-audit Agent (onboarding-hire-an-ai-teammate) findings F13 + F15 (`docs/skycot/ux-audits/2026-05-25-2309/onboarding-hire-an-ai-teammate.md`).

🟢 **P2/P3 — Copy slip: `/landing` shows "2,847 TEAMS THIS MONTH"; wizard footer shows "2,847 COMPANIES THIS MONTH" — same number, different noun. First-hire wizard's example prompt names the worker "Maya" — collides with the seeded Maya in the demo tenant.**

## Root cause

**F13** (P2): Two surfaces in the same funnel display the same digit with different nouns:
- `/landing` hero strip: `2,847 TEAMS THIS MONTH`
- `/onboarding/_OnboardingShell.tsx` footer: `2,847 COMPANIES THIS MONTH`

Someone in the funnel will see "TEAMS" then "COMPANIES" with the exact same digit and notice. Copy slip, not a data issue.

**F15** (P3): `/onboarding/first-hire` placeholder + example text says "A marketing intern named Maya who drafts blog posts...". The gateway already has a worker named Maya (the seeded marketing-intern in the demo tenant). If the operator uses the example as-is, they create a duplicate Maya in the same tenant. Name-collision behaviour is unguarded — could be 409, could be silently allowed with two Mayas.

Combined as one brief because both are wizard-copy hygiene fixes.

## Findings this brief closes

- onboarding-hire-an-ai-teammate F13 [P2] — "2,847 COMPANIES THIS MONTH" vs "2,847 TEAMS THIS MONTH" — same number, different noun
- onboarding-hire-an-ai-teammate F15 [P3] — First-hire example prompt names the worker "Maya" — collides with existing seeded worker

## Fix recipe

**For F13**:
1. Touch sites:
   - `web/app/(setup)/onboarding/_OnboardingShell.tsx` (the footer stat)
   - `web/app/landing/page.tsx` (the hero strip)
2. Pick ONE noun. Per the SKYCOT proposition (named teammates, not workflows), **"TEAMS"** is the right pick.
3. Change the wizard footer string to `"2,847 TEAMS THIS MONTH"` so it matches `/landing`.
4. While there, audit any other surface that displays the same `2,847` number — they should all use "TEAMS".

**For F15**:
1. Touch site: `web/app/(setup)/onboarding/first-hire/page.tsx:11` — the `EXAMPLE_PROMPT` constant.
2. Change the example name from "Maya" to a name NOT in the canonical demo roster. Pick from the roster's outside-set: "Marcus", "Hana", "Theo", "Iris", "Ezra" — any first name that isn't Maya / Mariah / Peta / Priya / Jamal / Nori / Patrick.
3. **Bonus** — server-side: have the gateway return a `{ "error": "name_taken", "suggestions": ["Marcus", "Hana"] }` error envelope on duplicate names in the same tenant, and have the wizard render it inline. Treats the symptom of any name collision, not just Maya. P3 carrots for the brief if effort allows.

**Pin**:
- `web/app/(setup)/onboarding/_OnboardingShell.test.tsx`: assert footer string contains "TEAMS", not "COMPANIES".
- `web/app/(setup)/onboarding/first-hire/page.test.tsx`: assert `EXAMPLE_PROMPT` does NOT contain "Maya" (or any canonical roster name).

## Acceptance criteria

- [ ] Wizard footer reads "2,847 TEAMS THIS MONTH".
- [ ] `/landing` hero strip reads "2,847 TEAMS THIS MONTH" (already does — verify no regression).
- [ ] First-hire example prompt does NOT name the worker "Maya" or any other name in the canonical demo roster.
- [ ] Optional: gateway returns `name_taken` error on duplicate-name hire + wizard renders it.
- [ ] Two test pins pass.

## May-touch

- `web/app/(setup)/onboarding/_OnboardingShell.tsx`
- `web/app/(setup)/onboarding/first-hire/page.tsx`
- `web/app/landing/page.tsx` (verify only)
- Test files (NEW or extend)
- `crates/argo-cli/src/handlers/gateway/routes_post_writes_onboarding.rs` (optional, for the duplicate-name error envelope)

## Out of scope

- Wider stat-derivation work (the `2,847` is itself dressing-side; not in scope)
- Vertical-list alignment (separate brief `fix-onboarding-vertical-list-align-with-landing-personas`)

## Effort

~20-40 LOC + 2 test pins. ~30 min (without the gateway error envelope; +1h with it).
