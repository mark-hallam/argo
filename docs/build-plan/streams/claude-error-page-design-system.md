# claude-error-page-design-system — branded 404/500/maintenance pages (Wave 4 P2)

**Owner**: claude. **Depends on**: `claude-brand-identity-system` MERGED.
**Authored against**: origin/main `054d8b0a` on `2026-05-16`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-error-page-design-system`.
**Source**: §4 weekend plan Wave 4 — brand discipline applies to error states.

## Mission alignment

- **Reliability** (CLAUDE.md priority #1) — error pages are honest signal, not generic stack traces.
- **Commercial usability** (#5) — "Jonny-Ive design magic" extends to *every* surface including failure modes.

## Scope

P2. Three error surfaces, each rendered in both the marketing site (`skycot.com`) and the SPA (`app.skycot.com`):

1. **404 Not Found** — page-not-found. Useful: nearest navigation suggestions + search hint + back link.
2. **500 Internal Server Error** — generic failure. Useful: incident-status link + reload + support email.
3. **Maintenance / Planned downtime** — banner-overlay rendered when `ARGO_MAINTENANCE_MODE=1`; respects existing API for setting maintenance window.

Brand discipline (per `claude-brand-identity-system`):
- Monochrome base + one warm accent only.
- Inter Display for the headline number (404, 500), Inter for body.
- Generous white-space; max 65ch text width; centred composition.
- A single illustration motif — e.g. minimal geometric line-art — NOT stock astronaut/dog.
- Joyful, not flippant. Copy tone: calm + competent + helpful.

LOC: ~200.

## Files you may touch

**SPA (app.skycot.com):**
- `web/app/not-found.tsx` (existing — replace placeholder) — 404 page. ≤80 LOC.
- `web/app/error.tsx` (NEW, App Router error boundary) — 500 page. ≤80 LOC.
- `web/app/maintenance/page.tsx` (NEW) — explicit `/maintenance` route. ≤40 LOC.
- `web/components/error/ErrorShell.tsx` (NEW) — shared layout. ≤80 LOC.
- `web/components/error/ErrorIllustration.tsx` (NEW) — inline SVG motif. ≤60 LOC.

**Marketing (skycot.com):**
- `marketing/app/not-found.tsx` (NEW) — marketing 404. ≤60 LOC.
- `marketing/app/error.tsx` (NEW) — marketing 500. ≤60 LOC.

**Backend:**
- `crates/argo-cli/src/handlers/gateway/maintenance_middleware.rs` (NEW) — emits `503 Service Unavailable` with `Retry-After` header + JSON body. ≤50 LOC.
- `crates/argo-cli/src/handlers/gateway/boot.rs` — mount middleware gated on env var. ≤8 modified LOC.

**Tests:**
- `web/__tests__/error-pages.test.tsx` (NEW) — render + accessibility (axe). ≤60 LOC.

## Files you must NOT touch

- The brand tokens themselves (`web/styles/tokens.css` or equivalent) — consume only.
- Real error-handling logic anywhere — this brief is presentation only.
- Sentry / OTel wiring — that's `codex-sentry-otel-error-reporting`.

## Acceptance criteria

- [ ] `/this-route-does-not-exist` → 404 page matches brand discipline.
- [ ] Server-side throw → 500 page matches brand discipline; reload button + support email + status-page link rendered.
- [ ] `ARGO_MAINTENANCE_MODE=1` → all routes return 503 + render maintenance page with operator-configured ETA.
- [ ] Single illustration motif per surface — no stock imagery, no rainbow gradients.
- [ ] Copy passes "helpful + calm + competent" tone check (verify against `docs/skycot/brand/voice-and-tone.md`).
- [ ] Lighthouse accessibility ≥ 95 for all three pages (text-contrast, semantic landmarks).
- [ ] All pages render with no console errors / hydration mismatches.
- [ ] Marketing + SPA share visual language (different layouts, same tokens).
- [ ] `pnpm typecheck && pnpm test && pnpm lint` clean for web + marketing.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `claude-error-page-design-system`
- Title: `web: branded 404/500/maintenance pages (claude-error-page-design-system)`
- Worker verification: `pnpm typecheck && pnpm test` per SWARM_PROTOCOL.md.

## Hidden constraints

- **Next.js App Router error boundaries are CLIENT components** — they receive `error` and `reset` props. Do not attempt to render server-side data; only the static shell + retry button.
- **Maintenance middleware order matters** — must mount AFTER health-check route (so liveness probe still works) but BEFORE everything else.
- **No external image hosts** — illustration is inline SVG to avoid asset-loader dependencies and CSP issues.
- **Marketing + SPA both render** — they share tokens but NOT components; do not import across the Next.js project boundary.
