# fix-vertical-landing-cta-and-step-copy-worker-to-teammate

**Owner**: claude.
**Authored against**: origin/main `9c407cd20039adf25a986343b7c8cdba76650a52` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-vertical-landing-cta-and-step-copy-worker-to-teammate`.
**Source**: UX-AUDIT-T0015 synthesis (`docs/skycot/ux-audits/2026-05-27-0015/onboarding-hire-an-ai-teammate.md` F10 [P2] + `docs/skycot/ux-audits/2026-05-27-0015/whole-site-mvp-claims-coverage.md` F25 [P3]).

🟡 **P2 — The customer-facing vocabulary is "teammate" everywhere except (a) the primary CTA on `/for/[vertical]` pages, which says "Hire your first worker", and (b) two steps of `/landing` how-it-works that say "named worker (Mariah, Peta, …)". Carry-over from the 2026-05-26-2309 audit (F5) — STILL UNCHANGED on main.**

## Root cause

Operator-locked memory `project_vocabulary_duty_vs_schedule` is the load-bearing principle: in any SPA / hire wizard / marketing / docs / settings copy, customer-facing language is "teammate" / "schedule". Code/SQL/audit/brief filenames keep the legacy primitive name. Three customer-facing surfaces leak "worker" into copy the camera reads:

1. **`web/app/for/_components/VerticalLandingShell.tsx:600`** — the primary CTA on every `/for/<vertical>` page renders as **"Hire your first worker"**. The same CTA on `/landing` + `/auth/signup` + `/team` + `/compare` + customer-detail pages reads "Hire your first teammate". `/for/_data/verticals.ts` even uses "teammate" inside per-vertical headlines ("Hire your first sales teammate") — only the CTA copy is the outlier.

2. **`/landing` how-it-works step-02** at `web/app/landing/page.tsx` — "SKYCOT spins up a named **worker** (Mariah, Peta, Priya, Jamal, or Nori)…". Worker leaking into customer copy at the very moment the page is teaching the reader what a teammate is.

3. **`/landing` how-it-works step-04** — "audit-logged with **worker**, action, cost, latency".

Combined effect: a buyer scanning the marketing perimeter sees "teammate" 30+ times then "worker" 3 times on the most important moments (per-vertical primary CTA + the marketing page that defines the product noun).

## Findings this brief closes

- `docs/skycot/ux-audits/2026-05-27-0015/onboarding-hire-an-ai-teammate.md` F10 [P2] — Inconsistent primary CTA wording: "Hire your first teammate" everywhere except per-vertical landing pages
- `docs/skycot/ux-audits/2026-05-27-0015/whole-site-mvp-claims-coverage.md` F25 [P3] — Marketing pages mix "teammate" (modern) and "worker" (legacy code-canonical) in copy

## Fix recipe

1. `web/app/for/_components/VerticalLandingShell.tsx:600` — change "Hire your first worker" → "Hire your first teammate" (or use the per-vertical headline noun where available — `Hire your first ${vertical.label} teammate`).
2. `web/app/landing/page.tsx` — sweep how-it-works steps 2 and 4. Replace customer-visible "worker" with "teammate". Keep the named persona list intact.
3. After the targeted fixes, run a regression grep to make sure no other customer-visible surface still says "worker":
   ```bash
   rg -nE '\b[Ww]orker(s)?\b' web/app/landing web/app/for web/app/auth web/app/customers web/app/security web/app/changelog web/app/compare web/app/pricing | grep -v -E '(// |/\*|test|spec|__tests__)'
   ```
4. Document the operator-locked vocabulary at the top of `web/lib/skycot/vocabulary.ts` (or new file) and add a lint that flags new occurrences of customer-facing "worker" in `web/app/(marketing|landing|for|auth)/**`.

## Acceptance criteria

- [ ] `web/app/for/_components/VerticalLandingShell.tsx:600` reads "teammate" (or per-vertical "<vertical> teammate"), not "worker".
- [ ] `/landing` how-it-works steps 2 and 4 use "teammate".
- [ ] `rg -nE '\bworker(s)?\b' web/app/landing web/app/for web/app/auth web/app/customers web/app/security web/app/changelog web/app/compare web/app/pricing` returns zero customer-visible matches (lint-comments / code-symbol / test files are excluded).
- [ ] `pnpm --dir web exec tsc --noEmit` green.
- [ ] `pnpm --dir web exec vitest run` no regressions.
- [ ] (Optional) regression lint added that fails CI if any new occurrence appears.

## May-touch

- `web/app/for/_components/VerticalLandingShell.tsx` (~3 LOC at line 600)
- `web/app/landing/page.tsx` (~6 LOC across steps 2 + 4)
- Any other marketing-perimeter file the regression grep flags (low expected count)
- New `web/lib/__tests__/no-customer-worker-copy.test.ts` (~30 LOC, optional)

## Out of scope

- Code-side primitive renames (worker → teammate) — operator-decision, NOT in scope here
- The /workers route + nav-items "Workers" label — that's a separate breadcrumb-drift issue covered by `fix-workers-team-route-ambiguity-and-breadcrumb-drift`
- Internal-only surfaces (/dev, /api, settings) where "worker" is technically appropriate

## Effort

S. ~10 LOC + ~30 LOC test. ~30 min.
