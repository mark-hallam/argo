# web-hire-loud-failure-banner

**Authored against**: `origin/main` `8b216295` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-hire-loud-failure-banner`.
**Priority**: **P1 — wave-15 buyer-trial blocker #3 (phase 2 of 3).**

## Summary

Phase 2 of 3 split from `web-hire-workerdraft-schema-align` (superseded). When the gateway Genesis call fails (network / 5xx / parse error), surface an explicit "Generic questions only" warning banner with a Retry button on hire-flow Step 3. No more silent `STUB_QUESTIONS` fallback.

Pre-condition: **`web-hire-workerdraft-schema-mirror-only` MERGED** (the hook needs to return `{ source, error }`).

## Acceptance

- [ ] `web/app/(hire)/hire/step/3/page.tsx` (or actual Step 3 path — `rg -ln 'STUB_QUESTIONS' web/app/`) renders a warning Alert when the genesis call is using the stub fallback.
- [ ] Alert copy (plain language per operator preference): "Generic questions only — we couldn't reach Genesis. The worker we draft from your answers may be generic too. [Retry]"
- [ ] Retry button re-invokes the gateway call. If still failing, banner stays. If success, banner disappears.
- [ ] `web/lib/hire/use-genesis-worker-draft.ts` returns `{ source: 'gateway' | 'stub', error?: Error }` — UI gates the banner on `source === 'stub'`.
- [ ] No regression on the live-gateway success path (banner does NOT appear when gateway succeeds).
- [ ] Reduced-motion respected (no shake/pulse animation on the banner; static visual emphasis only).
- [ ] One Playwright test that mocks the gateway to return 500 → banner appears → Retry button is clickable.
- [ ] One unit test that asserts banner does NOT render when `source === 'gateway'`.
- [ ] `pnpm typecheck` + `pnpm lint` + `pnpm test --run` clean.

## May touch

- `web/app/(hire)/hire/step/3/page.tsx` (or actual Step 3 path)
- `web/lib/hire/use-genesis-worker-draft.ts` (extend to return `{ source, error }`)
- `web/components/skycot/hire/GenesisFallbackBanner.tsx` (new) or inline if cleaner
- New unit/Playwright test files

## Must NOT touch

- `web/lib/hire/draft-types.ts` (Phase 1 owns schema)
- `web/lib/hire/genesis-stub.ts` (keep as fallback safety net; this brief makes the fallback OBSERVABLE not removed)
- `crates/argo-cli/src/handlers/gateway/routes_workers_hire.rs` (Phase 3)
- Audit-kind variants (defer to optional sibling brief if needed)

## Sizing

150–250 LOC.

## Notes

- Pre-flight: confirm the existing hook's return shape via `cat web/lib/hire/use-genesis-worker-draft.ts`.
- Operator preference: plain-language copy. "Generic questions only" not "Genesis service unavailable".
- This is the visible-improvement bite — users immediately see when their input might be ignored.
