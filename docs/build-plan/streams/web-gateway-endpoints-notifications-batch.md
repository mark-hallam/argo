# web-gateway-endpoints-notifications-batch — implement 4 missing notifications endpoints

**Owner**: claude (Rust crate work — gateway side; codex acceptable too).
**Authored against**: origin/main `50e65e9f` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-gateway-endpoints-notifications-batch`.
**Priority**: P1 — wave-3 API-wiring audit found 17 missing gateway endpoints; the 4 notifications endpoints are entirely mock today, so the bell popover on every authed page is fake.
**Sizing**: ~400-700 LOC (4 handlers + storage + migration + tests).

## Why this brief exists

From the wave-3 API-wiring audit (`docs/skycot/audit/api-wiring/2026-05-20-wave-3-api-wiring.md`):

> Endpoints definitively missing from the gateway: 17. The big clusters:
> - All 4 `/api/v1/notifications/*` endpoints — bell popover is 100% mock.

The web app's `NotificationsBellTrigger` fetches from `/api/v1/notifications`
and silently falls back to seed when the gateway 404s. For demo this
is invisible (operator doesn't open the bell); for procurement, it's
a "this is fake data" tell.

## Endpoints to implement

1. `GET /api/v1/notifications` — list of notifications for the
   current principal (paginated; query params `?limit=&before=`).
   Returns envelope `{ items, has_more, cursor }`.
2. `POST /api/v1/notifications/<id>/dismiss` — mark a single
   notification as dismissed. Idempotent.
3. `POST /api/v1/notifications/dismiss-all` — mark all unread for
   the principal as dismissed.
4. `GET /api/v1/notifications/unread-count` — cheap counter for
   the bell badge.

## Storage

- New table via additive migration: `notifications` with
  `(id, tenant_id, principal_id, kind, body, created_at, read_at,
  dismissed_at, related_audit_id, severity)`.
- Indices on `(tenant_id, principal_id, dismissed_at)` and
  `(tenant_id, created_at desc)`.

## Steps

1. Verify next free migration slot (check `crates/argo-store/src/migrations/registry.rs`).
2. Add migration `NNN__notifications.sql` (additive).
3. Add `crates/argo-store/src/notifications.rs` with the CRUD ops.
4. Add gateway route handlers under `crates/argo-cli/src/handlers/gateway/routes_notifications.rs`.
5. Wire into the routes-dispatch in `routes.rs::dispatch_route_inner`.
6. Add unit tests in `routes_notifications_tests.rs`.
7. Cross-tenant negative test (principal_a cannot see principal_b's notifications).
8. Verify web side: `useUnreadNotificationCount` hook (`web/lib/hooks/`) now returns real data instead of mock fallback.

## may-touch

- New: `crates/argo-store/src/notifications.rs`
- New: migration file under `crates/argo-store/src/migrations/`
- `crates/argo-store/src/migrations/registry.rs` (register the new migration)
- New: `crates/argo-cli/src/handlers/gateway/routes_notifications.rs`
- New: `crates/argo-cli/src/handlers/gateway/routes_notifications_tests.rs`
- `crates/argo-cli/src/handlers/gateway/routes.rs` (dispatch wiring)
- `web/lib/api/notifications.ts` (only if envelope shape needs adjusting; default keep)

## must-NOT-touch

- Web-side mock fallback in `web/lib/api/notifications.ts` should stay as graceful degradation (do not delete).
- Other endpoint clusters (worker-skills, audit mutations, settings PATCH) — those are sibling briefs.

## Acceptance criteria

- [ ] All 4 endpoints return 200 on the running gateway with realistic data.
- [ ] Cross-tenant negative test passes: principal_a's bearer cannot fetch principal_b's notifications.
- [ ] Bell popover on the web shows live data when gateway is up (verified via `curl http://127.0.0.1:7400/api/v1/notifications`).
- [ ] `cargo clippy -p argo-store -p argo-cli --all-targets -- -D warnings` clean.
- [ ] Migration slot doesn't collide (check `scripts/check-migration-slots.sh`).
- [ ] At least 6 unit tests (CRUD + pagination + cross-tenant) + 1 integration test (full envelope).

## References

- Audit: `docs/skycot/audit/api-wiring/2026-05-20-wave-3-api-wiring.md`
- Pattern: `routes_team_list.rs` for envelope shape; `routes_audit.rs` for pagination
