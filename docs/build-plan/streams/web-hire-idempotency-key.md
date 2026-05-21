# web-hire-idempotency-key

**Authored against**: `origin/main` `8b216295` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-hire-idempotency-key`.
**Priority**: **P2 — wave-15 buyer-trial blocker #3 (phase 3 of 3).**

## Summary

Phase 3 of 3 split from `web-hire-workerdraft-schema-align` (superseded). Client generates a UUID v7 per hire-flow lifecycle, sends it as `idempotency_key` in the gateway hire POST body. Gateway de-dupes against existing rows in a 1-hour window. Prevents duplicate-hire storms from double-clicks or page reloads.

Pre-condition: **`web-hire-workerdraft-schema-mirror-only` MERGED** (the request envelope is settled).

## Acceptance

- [ ] Client generates a UUID v7 on Step 1 first-load + persists it across Step 2–6 in the draft state. Use `crypto.randomUUID()` + a v7 wrapper, OR `uuid/v7` if already in `web/package.json`.
- [ ] Hire submit POST body includes `idempotency_key`. Gateway response on duplicate: `200 OK` body `{ worker_id, duplicate_of, message: "Already hired (idempotent)" }`.
- [ ] Gateway side (`crates/argo-cli/src/handlers/gateway/routes_workers_hire.rs`): de-dupe against existing rows keyed on `(tenant_id, idempotency_key, created_at > now - 1h)`. Window configurable via `ARGO_HIRE_IDEMPOTENCY_WINDOW_SECS` env var (default `3600`).
- [ ] Gateway integration test: same idempotency_key submitted twice → exactly 1 worker row in DB. Second response includes `duplicate_of`.
- [ ] Negative test: different idempotency_key → 2 distinct rows.
- [ ] Negative test: same idempotency_key from tenant_b accessing tenant_a → 403 / 404 (cross-tenant isolation, per the canonical pattern).
- [ ] `pnpm typecheck` + `pnpm lint` + `pnpm test --run` clean.
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean.

## May touch

- `web/lib/api/workers.ts` (add `idempotency_key` to POST body)
- `web/lib/hire/use-hire-draft.ts` (or wherever the hire-flow lifecycle state lives — verify with `rg -ln 'useHireDraft\|useHireFlow' web/lib/`)
- `crates/argo-cli/src/handlers/gateway/routes_workers_hire.rs` (accept + dedupe)
- `crates/argo-cli/src/handlers/gateway/routes_workers_hire_tests.rs` (3 new tests)

## Must NOT touch

- `web/lib/hire/draft-types.ts` (Phase 1 settled)
- `web/lib/hire/genesis-stub.ts` (Phase 2 settled)
- Step 3 banner UI (Phase 2 settled)
- `crates/argo-store/migrations/*.sql` (no migration needed; idempotency_key lives in audit row payload or in the workers table if there's an existing field — verify first)

## Sizing

200–300 LOC.

## Notes

- Pre-flight: `grep '"uuid"' web/package.json` — if already present, use `uuid/v7`. Otherwise `crypto.randomUUID()` + v7 generator is fine.
- The 5-teammate batch hire is the killer test — opens 5 separate hire flows, each gets a unique idempotency_key, none collapse to a single row.
- v1.0 demo benefit: prevents the "I clicked submit twice and got 2 of the same worker" embarrassment.
