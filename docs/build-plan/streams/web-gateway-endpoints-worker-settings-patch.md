# web-gateway-endpoints-worker-settings-patch — implement 4 worker-settings PATCH endpoints

**Owner**: claude (or codex).
**Authored against**: origin/main `50e65e9f` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-gateway-endpoints-worker-settings-patch`.
**Priority**: P2 — wave-3 audit found these endpoints "saved but doesn't persist". Wave-4 forms flagged these as the most-likely-clicked-on-camera silent-broken forms.
**Sizing**: ~400-700 LOC (4 PATCH handlers + storage column adds + tests).

## Why this brief exists

From the wave-3 API-wiring audit:

> All 4 `/api/v1/workers/<id>/settings/*` PATCH mutations.
> Worker settings PATCH mutations (P2 — "saved" but doesn't persist).

From the wave-4 forms audit:

> Worker Settings 5 sections (Channels / Quiet hours / Budget / Autonomy /
> Approval-policy) — `disabled={!isDirty}` only; in-flight saves are
> double-clickable; no success toast; no inline errors.

The web form pretends to save; the gateway has no endpoint to take
the save. After the form-validation brief lands client-side state,
this brief lands the server side.

## Endpoints to implement

1. `PATCH /api/v1/workers/<id>/settings/channels` — body `{ slack: bool, im: bool, email: bool, web_inbox: bool }`
2. `PATCH /api/v1/workers/<id>/settings/quiet-hours` — body `{ start_local: "HH:MM", end_local: "HH:MM", timezone: "IANA/zone" }`
3. `PATCH /api/v1/workers/<id>/settings/budget` — body `{ monthly_usd_cap: number, alert_at_pct: number }`
4. `PATCH /api/v1/workers/<id>/settings/autonomy` — body `{ level: "conservative" | "balanced" | "aggressive" }`

(The 5th section, Approval policy, may need a sibling brief if more involved.)

## Steps

1. Find existing worker-settings storage in `crates/argo-identity/` or `crates/argo-store/`.
2. Add columns/json blob for each section if missing (additive migration).
3. Add gateway handlers + tests + dispatch wire.
4. Each handler validates: caller owns the worker, body matches schema, response carries the new state.
5. Writes an audit row (so settings changes are tracked).

## may-touch

- `crates/argo-store/src/worker_settings.rs` (extend or create)
- Optional migration
- New: `crates/argo-cli/src/handlers/gateway/routes_worker_settings.rs`
- New: tests
- `crates/argo-cli/src/handlers/gateway/routes.rs`
- `web/lib/api/worker-settings.ts` (only if envelope shape changes)

## must-NOT-touch

- `web/app/(app)/workers/[workerId]/settings/page.tsx` — the form-validation sibling brief handles client side.

## Acceptance criteria

- [ ] All 4 PATCH endpoints round-trip: PATCH → GET returns the updated value.
- [ ] Audit row written for each change (visible in /audit with kind `worker_settings_changed`).
- [ ] Cross-tenant negative: PATCH `/workers/<other-tenant-id>/settings/...` returns 404.
- [ ] `cargo clippy --workspace -- -D warnings` clean.
- [ ] At least 8 unit tests.
- [ ] curl matrix: each endpoint accepts valid body, rejects invalid body (400), rejects cross-tenant (404).

## References

- Audit: `docs/skycot/audit/api-wiring/2026-05-20-wave-3-api-wiring.md`
- Audit: `docs/skycot/audit/forms/2026-05-20-wave-4-forms.md`
- Sibling: `web-form-validation-zod-sweep` (client side)
