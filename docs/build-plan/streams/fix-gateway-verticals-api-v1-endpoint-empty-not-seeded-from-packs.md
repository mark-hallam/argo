# fix-gateway-verticals-api-v1-endpoint-empty-not-seeded-from-packs

**Owner**: codex.
**Authored against**: origin/main `b77864a000a9738c80f235fc2605d192197cd5c7` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-gateway-verticals-api-v1-endpoint-empty-not-seeded-from-packs`.
**Source**: e2e-ux-audit T0819 — `docs/skycot/ux-audits/2026-05-27-0819/onboarding-hire-an-ai-teammate.md` F11 [P2]; cross-ref `whole-site-mvp-claims-coverage.md` Claim 3 [P1].

🟡 **P2 — `GET /api/v1/verticals` returns `{"verticals":[]}` even though the gateway has 33 verticals loaded under the legacy `/verticals` envelope (`{loaded:[...], errors:[], ok:true}`). The `/onboarding/pick-vertical` step calls `/api/v1/verticals`, gets empty, falls back to `FALLBACK_VERTICALS` (7 hardcoded). The hint text below the grid reads `v1 catalogue (offline · 7 verticals)` — telling the user the catalogue is "offline" when it's actually just unwired.**

## Root cause

The gateway exposes two different verticals routes:

1. **Legacy** `GET /verticals` → returns `{loaded: [...33 verticals...], errors: [], ok: true}` — works, has data
2. **Canonical** `GET /api/v1/verticals` → returns `{"verticals": []}` — empty envelope, no data

The SPA's `/onboarding/pick-vertical` page (`web/app/(setup)/onboarding/pick-vertical/page.tsx:210-215`) hits the canonical `/api/v1/verticals` endpoint, gets empty, and falls back to its 7-card hardcoded `FALLBACK_VERTICALS`. The skill counts on the fallback cards match the actual gateway install (`gtm-engineer: 8`, `marketing-intern: 5`, etc.) — so picking a card and clicking it does land real skills via `install-vertical`. But the catalogue list itself is a placeholder.

Two possible fixes server-side:
- **(a)** Make `/api/v1/verticals` read from the same source as `/verticals` (the on-disk packs in `crates/argo-gateway/src/verticals/` or wherever the pack manifest lives). Single source of truth.
- **(b)** Deprecate one of the two routes and have one alias the other. Simpler.

Cross-reference: this is the same root cluster as `fix-gateway-skills-and-verticals-boot-seed-tenant-resolution` — the skills endpoint has the same shape (boot-seed not landing for the tenant the SPA resolves). The verticals issue may be a separate code path (pack-loading-side) or may be the same path with a separate tenant-binding gap.

Evidence:
- `curl http://127.0.0.1:7400/verticals` → 33 loaded verticals
- `curl http://127.0.0.1:7400/api/v1/verticals` → `{"verticals":[]}`
- `/onboarding/pick-vertical` renders 7 hardcoded fallback cards with `(offline · 7 verticals)` hint

## Findings this brief closes

- onboarding-hire-an-ai-teammate F11 [P2] — Vertical catalogue is always empty from the gateway; the wizard always falls back to its hardcoded 7-card list

## Fix recipe

1. **Source-walk the two routes.** Find `crates/argo-cli/src/handlers/gateway/routes_verticals*.rs` (or equivalent). Identify which handler serves `/verticals` and which serves `/api/v1/verticals`.
2. **Decide canonical path.** If `fix-gateway-skills-and-verticals-boot-seed-tenant-resolution` is concurrently in flight, coordinate so both verticals endpoints converge on the same source (the on-disk pack manifest at boot, seeded into the tenant's vertical_catalog table or read live).
3. **Implement option (a) or (b)**:
   - (a) Make `/api/v1/verticals` read from the same pack-manifest source as `/verticals`. Same shape transformation, single source.
   - (b) Have `/api/v1/verticals` alias `/verticals` and return the same envelope (or transform `{loaded:[...]}` → `{verticals:[...]}`).
4. **Update SPA's fallback copy.** In `web/app/(setup)/onboarding/pick-vertical/page.tsx`, until this lands, change the hint text from `(offline · 7 verticals)` → `(7 starting templates)` so the user doesn't think something's broken. Drop the hint entirely when the gateway-side fix lands.
5. **Regression test** in `crates/argo-cli/tests/` asserting both `/verticals` and `/api/v1/verticals` return non-empty lists on cold boot.

## Acceptance criteria

- [ ] `curl http://127.0.0.1:7400/api/v1/verticals` returns ≥ 7 verticals.
- [ ] `/onboarding/pick-vertical` no longer falls back to `FALLBACK_VERTICALS` — renders real gateway-fed cards.
- [ ] The hint text `(offline · 7 verticals)` is removed or replaced once gateway data lands.
- [ ] Cargo test asserts non-empty response on cold boot.

## May-touch

- `crates/argo-cli/src/handlers/gateway/routes_verticals*.rs` (or wherever the verticals routes live)
- `crates/argo-cli/tests/gateway_verticals_test.rs` (NEW or extend)
- `web/app/(setup)/onboarding/pick-vertical/page.tsx` (fallback copy)

## Out of scope

- The skills boot-seed (covered by `fix-gateway-skills-and-verticals-boot-seed-tenant-resolution` — may overlap if same root cause)
- Adding new verticals to the pack manifest (separate effort)
- `/verticals` page (separate SPA route — different from `/onboarding/pick-vertical`)

## Effort

S-M — ~100-200 LOC across handler unification + tests + SPA copy. ~2-3 hours. P2 because the path works via fallback today; the "offline" copy is the only visible signal something is wrong.
