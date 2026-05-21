# web-hire-workerdraft-schema-mirror-only

**Authored against**: `origin/main` `8b216295` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-hire-workerdraft-schema-mirror-only`.
**Priority**: **P0 — wave-15 buyer-trial blocker #3 (phase 1 of 3).**

## Summary

Phase 1 of 3 split from `web-hire-workerdraft-schema-align` (superseded). Focused scope: ALIGN the SPA TypeScript `WorkerDraft` interface to the gateway-side Rust `WorkerDraft` struct + add a structural test that fails when they drift. No banner, no idempotency — just the schema mirror.

This brief alone closes the root cause of the silent fallback → "Mariah clones" bug. Phase 2 (`web-hire-loud-failure-banner`) and Phase 3 (`web-hire-idempotency-key`) follow.

## Context (from parent brief)

Read [docs/build-plan/streams/web-hire-workerdraft-schema-align.md](docs/build-plan/streams/web-hire-workerdraft-schema-align.md) for the full failure chain. tldr: gateway returns a `WorkerDraft` whose JSON shape doesn't match the SPA's TS interface; parse fails silently; SPA falls back to `STUB_QUESTIONS`; 5 hires get 5 identical drafts.

## Acceptance

- [ ] Schema delta audit in PR body: `pub struct WorkerDraft` fields from `crates/argo-genesis/src/` (or wherever it lives — verify) listed side-by-side with `interface WorkerDraft` from `web/lib/hire/draft-types.ts`. Every field name + type matched.
- [ ] `web/lib/hire/draft-types.ts` updated so every gateway-side field is present in the TS interface. Camel-case TS / snake-case JSON mapping at the boundary if needed.
- [ ] `web/lib/api/workers.ts` + `web/lib/hire/use-genesis-worker-draft.ts` updated to consume the real gateway response shape (deserialize correctly).
- [ ] `web/lib/hire/__tests__/workerdraft-schema-shape.test.ts` (new) — structural test that asserts every gateway-side field is present in the TS interface. Test reads the canonical Rust struct via a fixture (paste the field list as a const) and fails when the TS interface omits anything.
- [ ] Live-gateway path produces a real `WorkerDraft` (different inputs → different drafts). Verify via unit test mocking the gateway response with the new shape.
- [ ] No regression: `STUB_QUESTIONS` still loads when the gateway is unreachable (banner work belongs to Phase 2).
- [ ] `pnpm typecheck` + `pnpm lint` + `pnpm test --run` clean.
- [ ] Every modified file < 500 LOC.

## May touch

- `web/lib/hire/draft-types.ts` — field alignment
- `web/lib/hire/use-genesis-worker-draft.ts` — consume new shape
- `web/lib/api/workers.ts` — snake↔camel mapping at boundary
- `web/lib/hire/__tests__/workerdraft-schema-shape.test.ts` (new)

## Must NOT touch

- `web/lib/hire/genesis-stub.ts` (Phase 2 owns the silent-fallback deprecation)
- `crates/argo-cli/src/handlers/gateway/routes_workers_hire.rs` (Phase 3 owns the gateway idempotency work)
- `crates/argo-genesis/src/*` (the source-of-truth struct lives there; this brief mirrors it)
- Hire-flow UI files (Phase 2 owns Step 3 banner work)
- `crates/argo-store/migrations/*.sql` (no schema change needed)

## Sizing

200–300 LOC. Single-pass mirror + structural test.

## Notes

- Pre-flight: `rg -n 'pub struct WorkerDraft' crates/` to find canonical location.
- Pre-flight: `grep -n 'interface WorkerDraft' web/lib/hire/draft-types.ts` for current TS shape.
- This is the highest-leverage of the 3 phases — root cause fix.
- Phase 2 (banner) and Phase 3 (idempotency) are visible-improvement bites that pair on top once this lands.
