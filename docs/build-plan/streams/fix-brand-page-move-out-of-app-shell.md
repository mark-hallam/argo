# fix-brand-page-move-out-of-app-shell

**Owner**: claude or codex.
**Authored against**: origin/main `9c407cd20039adf25a986343b7c8cdba76650a52` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-brand-page-move-out-of-app-shell`.
**Source**: UX-AUDIT-T0015 synthesis (`docs/skycot/ux-audits/2026-05-27-0015/whole-site-mvp-claims-coverage.md` F15 [P2]).

🟡 **P2 — `web/app/(app)/brand/page.tsx` (227 LOC, internal design-system reference: color tokens, typography samples, wordmark panel) lives under `(app)/` so it renders inside the customer app shell — sidebar nav present, demo banner overlay, demo-mode footer. A logged-in user can land here by typing the URL. Not linked from anywhere a customer would discover it, but it pollutes the customer route tree and surfaces internal design content as if it were a product feature.**

## Root cause

The brand reference page was authored as an in-app `(app)/brand/page.tsx` (probably for fast iteration when bringing the brand system live). The `(app)/` layout segment in Next.js wraps every child in the shared shell (`web/app/(app)/layout.tsx`) — sidebar nav, demo banner, command palette, app-chrome footer. So `/brand`:

- Shows the sidebar nav (Approvals / Team / Workers / Cohorts / …)
- Shows the demo banner
- Sits in the customer route tree alongside `/approvals`, `/audit`, etc.
- Is reachable via direct URL by any logged-in user (no auth gate beyond the shell's middleware)

It's not linked from anywhere customer-facing, but the URL is discoverable (Chrome address-bar autocompletes after one visit, search indices, etc.).

## Findings this brief closes

- `docs/skycot/ux-audits/2026-05-27-0015/whole-site-mvp-claims-coverage.md` F15 [P2] — `/brand` lives at `/(app)/brand/page.tsx` — exposed via direct URL inside the app shell

## Fix recipe

Three options; recommend (a).

**Option A — move to `_dev/brand` (preferred — fully off the customer route tree)**:

1. Move `web/app/(app)/brand/page.tsx` → `web/app/_dev/brand/page.tsx` (Next.js `_dev` underscore-prefixed segment is private — won't render as a route; Next.js will 404 the path).
2. If the operator still wants to view it, expose it via a dev-only env-var-gated route at `/dev/brand` that 404s in production.
3. Update any internal docs / READMEs that link to `/brand`.

**Option B — keep under `(app)` but add explicit dev-only gating + an "internal-only" badge**:

1. Wrap the page body with `if (process.env.NEXT_PUBLIC_SKYCOT_DEMO_MODE !== "1") return notFound();`.
2. Add a banner at the top: "Internal designer reference — not a product feature."
3. Optionally add a link from `/settings` → "Brand system reference" so it's discoverable to the team that needs it.

**Option C — move to `web/app/(setup)/brand/page.tsx`** — keeps it in a non-customer layout. Less clean than (a) since the (setup) shell is the onboarding flow, not a designer reference.

Pick (a). It's the load-bearing fix: the page is for internal designers, not customers; the customer route tree shouldn't carry internal references.

## Acceptance criteria

- [ ] `curl -s -o /dev/null -w '%{http_code}\n' http://localhost:3000/brand` returns 404 (or 200 only when `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` is set, via dev-gated `/dev/brand` if Option A includes the dev route).
- [ ] `web/app/(app)/brand/` directory is removed (or moved).
- [ ] The brand reference is reachable by an internal designer via the documented path (`/dev/brand` with env-var, or an internal README pointer).
- [ ] No new links to the old `/brand` URL from any customer-visible nav / footer / page.
- [ ] `pnpm --dir web exec tsc --noEmit` green.
- [ ] `pnpm --dir web exec vitest run` no regressions.

## May-touch

- `web/app/(app)/brand/page.tsx` → `web/app/_dev/brand/page.tsx` (file move, ~0 LOC change to the page itself)
- Any docs / READMEs that reference `/brand` (~5 LOC)
- Optional dev-gated `/dev/brand/page.tsx` (~20 LOC if Option A includes it)

## Out of scope

- Adding new brand-system content (this is a route move, not a content change)
- Updating `/settings` to link to the design system (operator-decisional; if added, separate brief)
- The /brand-reference page that already exists at a different URL (verify with `find web -name 'page.tsx' -path '*brand*'`)
- D3 brand-consistency pass (already shipped per memory `project_brand_consistency_pass`)

## Effort

XS. ~10 LOC for the move + docs cleanup. ~30 min.
