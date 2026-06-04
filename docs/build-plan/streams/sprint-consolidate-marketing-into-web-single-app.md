# sprint-consolidate-marketing-into-web-single-app

**Owner**: claude or codex (frontend + Vercel). **Windows lane B infra lane well-suited (it raised this + owns Vercel/DNS).**
**Operator decision 2026-06-04: ONE app at both domains — delete `marketing/`, serve skycot.com + app.skycot.com from `web/`.**
**Depends on**: pairs with `sprint-set-pro-price-49` (web/ becomes the single pricing source).
**Authored against**: origin/main `1260da9fb` on `2026-06-04`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh sprint-consolidate-marketing-into-web-single-app`.

## Why

There are two Next.js apps — `marketing/` (skycot.com apex, public site) + `web/`
(app.skycot.com, the product) — with FIVE overlapping route families
(pricing/about/contact/customers/changelog + `marketing/legal` vs `web/{privacy,terms,security,sla}`).
They silently drifted (the $49-vs-$29 pricing bug was the symptom). The operator chose **one
app at both domains** to kill the duplication at its root. Full options analysis + evidence:
`docs/skycot/audit/2026-06-04-marketing-vs-web-app-duplication.md`.

## What

Consolidate onto `web/` as the single front-end app serving BOTH domains:
1. **Reconcile the duplicate route families into `web/`** — `web/` is the actively-maintained
   one, so keep its versions as canonical; port any `marketing/`-only public content worth
   keeping (MDX blog/features/about copy) into `web/`. Ensure skycot.com's public pages
   (pricing/about/contact/customers/changelog/legal) all exist + work under `web/`.
2. **Serve both domains from `web/`** — `web/vercel.json` / Vercel project config so the apex
   `skycot.com` (+ `www`) AND `app.skycot.com` both route to `web/`. Public marketing routes at
   the apex, the authenticated product under `app.` (preserve the existing `(app)/` gating).
3. **Delete `marketing/`** (the app + its Vercel project + its README) once its content is ported.
4. **DNS / domain attach is the OPERATOR's step** — attaching skycot.com to web/'s Vercel
   project + the DNS records is operator/infra (document the exact steps in the PR + a runbook;
   do NOT attempt to change DNS or Vercel domains from code).

This is a sizable structural migration — do it carefully (don't lose public content; the public
site must keep working). Stage it: port-content → dual-domain config → verify → delete marketing/.

## May-touch
- `web/app/**` (port the marketing-only public routes/content), `web/vercel.json`, `web/next.config.ts` (domain/rewrite handling if needed)
- delete `marketing/**` (after porting) + `deploy/` references to the marketing Vercel project
- a `docs/skycot/deploy/` runbook for the operator's domain-attach + DNS steps
- `docs/build-plan/BUILD_STATUS.md` (ledger flip)

## Acceptance criteria
- [ ] All public routes that existed in `marketing/` (pricing/about/contact/customers/changelog/legal) exist + render under `web/` (no lost content); `pnpm build` of `web/` succeeds.
- [ ] `web/`'s Vercel config serves skycot.com (apex+www) + app.skycot.com; the `(app)/` auth gating is preserved (public pages stay public, product stays gated).
- [ ] `marketing/` is deleted (no orphaned references in deploy/ or docs); only `web/` remains.
- [ ] No duplicate route families remain (`rg` shows each public route in exactly one place — `web/`).
- [ ] PR body + a deploy runbook spell out the operator's remaining DNS/Vercel-domain-attach steps.
- [ ] `pnpm typecheck` + `pnpm lint` + `pnpm vitest run` GREEN.

## Source
Operator decision 2026-06-04 (AskUserQuestion: "one app at both domains"). Evidence + options: `docs/skycot/audit/2026-06-04-marketing-vs-web-app-duplication.md`.
