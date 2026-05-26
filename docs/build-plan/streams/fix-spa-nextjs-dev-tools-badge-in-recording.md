# fix-spa-nextjs-dev-tools-badge-in-recording

**Owner**: claude.
**Authored against**: origin/main `80e149df55f6` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-spa-nextjs-dev-tools-badge-in-recording`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-1028/scene-1-and-2-claim-1-and-2.md` F11 [P2].

🟡 **P2 — Next.js dev overlay renders a "1 Issue" red rounded chip bottom-left on every page in `next dev`. Permanently in frame during recording. Operator's pre-record checklist says "dev tools closed" but doesn't address the embedded overlay.**

## Root cause

`a051f_next_dist_compiled_next-devtools_index_*.js` (Next.js compiled dev overlay) loads in any `next dev` session regardless of `NEXT_PUBLIC_*` env flags. Visible bottom-left during recording.

## Findings this brief closes

- scene-1-and-2-claim-1-and-2 F11 [P2] — Next.js dev-tools "1 Issue" red chip visible during recording

## Fix recipe

Pick ONE.

**(a) Record against production build:**
1. Update `docs/skycot/demos/v1-mission-mvp-runbook.md` pre-record to `pnpm build && pnpm start` (production-mode Next.js — no dev overlay).
2. Smoke-test the prod build to confirm the runbook's flows still work (no `pnpm dev`-only features).

**(b) Hide the overlay in dev:**
1. Set `NEXT_PUBLIC_HIDE_DEV_OVERLAY=1` and add CSS rule `[data-nextjs-toast] { display: none !important; }` to a top-level layout style block. Brittle (Next.js could rename the data attribute) but cheap.

Prefer (a) — production-mode is also more representative of what the buyer would deploy.

## Acceptance criteria

- [ ] Demo recording flow uses production-mode `pnpm start` (or the overlay is CSS-hidden in dev).
- [ ] No "1 Issue" chip visible bottom-left on any in-app page.
- [ ] Runbook pre-record checklist updated.
- [ ] Smoke test confirms prod build serves all in-scope routes.

## May-touch

- `docs/skycot/demos/v1-mission-mvp-runbook.md` (pre-record)
- `web/app/(app)/layout.tsx` (CSS override if going with (b))
- `web/next.config.js` (verify prod build is happy)

## Out of scope

- Fixing the underlying "1 Issue" (it's a dev-time hot-reload warning, not a real bug)
- Custom dev tooling

## Effort

~5-15 LOC + runbook edit + smoke pass. ~30min.
