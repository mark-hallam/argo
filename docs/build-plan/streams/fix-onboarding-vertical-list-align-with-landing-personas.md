# fix-onboarding-vertical-list-align-with-landing-personas

**Owner**: claude or codex.
**Authored against**: origin/main `0653c478d` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-onboarding-vertical-list-align-with-landing-personas`.
**Source**: e2e-ux-audit Agent 1 finding F9 (`docs/skycot/ux-audits/2026-05-25-1100-onboarding-hire-an-ai-teammate.md`).

🟡 **P2 — onboarding only ships 5 verticals; landing hypes Mariah (Marketplace) + Priya (Recruiter); user can't hire those.**

## Root cause

`web/app/(setup)/onboarding/pick-vertical/page.tsx:22-58` ships 5 verticals: GTM Engineer / Marketing Intern / Sales SDR / Customer Support / Accounts Receivable.

Missing from the operator's narrative:
- **Marketplace seller (Mariah)** — featured on `/landing` art bubbles + customer stories
- **Recruiter (Priya)** — featured on `/landing` + the v1.0 demo runbook

A user hiring "Priya the recruiter" today picks "Sales SDR" or "Customer Support" — neither maps. The personas the landing page leverages aren't installable from the onboarding wizard, creating a story → product friction point.

## Findings this brief closes

- Agent 1 F9 [P2] — `/onboarding/pick-vertical` only ships 5 templates, no Marketplace seller / Recruiter

## Fix recipe

Pick (a) — direct mapping:

**(a) Add the two missing verticals**:
1. Add "Marketplace Seller" (Mariah's archetype) — vertical pack manifest, default duties, default skills.
2. Add "Recruiter" (Priya's archetype) — vertical pack manifest, default duties, default skills.
3. Surface both in `web/app/(setup)/onboarding/pick-vertical/page.tsx:22-58` alongside the existing 5.
4. Verify both verticals can be hired from the wizard end-to-end and the resulting worker has a real personality (per `fix-worker-personality-and-level-derive-from-real-data`).

**(b) Rename existing verticals to map**:
1. Rename "Sales SDR" → "Sales & Recruiting"
2. Rename "Customer Support" → "Support / Marketplace"
3. Doesn't actually fix the persona drift — rejected.

**(c) Drop Mariah/Priya from landing/customer copy until the verticals ship**:
1. Smaller scope but degrades the marketing surface — rejected.

Recommendation: ship (a). Two new vertical pack manifests + 2 lines added to the picker.

## Acceptance criteria

- [ ] `/onboarding/pick-vertical` shows 7 vertical templates including Marketplace Seller + Recruiter.
- [ ] Hiring through either path creates a worker with the matching vertical id.
- [ ] The new worker has real default duties, default skills, default personality (per `fix-worker-personality-and-level-derive-from-real-data`).
- [ ] Landing-page persona-bubble references (Mariah marketplace, Priya recruiter) now match an installable vertical.
- [ ] Smoke test: hire a Recruiter from the wizard, land on `/workers/<id>`, render without 500.

## May-touch

- `web/app/(setup)/onboarding/pick-vertical/page.tsx` (the 5 → 7 list)
- `crates/argo-vertical-pack/manifests/marketplace-seller.yaml` (NEW)
- `crates/argo-vertical-pack/manifests/recruiter.yaml` (NEW)
- Default skills + duties for each manifest.

## Out of scope

- Worker personality / level derive (separate brief: `fix-worker-personality-and-level-derive-from-real-data`)
- Onboarding wizard chrome refactor (separate brief: `fix-onboarding-hire-parallel-wizards-unify-or-document`)
- Landing-page persona copy (separate brief: `fix-landing-page-fictitious-customer-logos-and-testimonials`)

## Effort

~150-250 LOC across 2 manifests + the picker. ~2-3 hours.
