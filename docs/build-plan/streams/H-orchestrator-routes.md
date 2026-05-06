# Stream H (follow-up) — Gateway routes for sessions / handoffs

**Owner**: codex. **Depends on**: H-orchestrator-persist merged.

## Scope

HTTP endpoints on the Argo gateway so the existing web inbox
(D-webinbox, already on main) can render live session +
handoff state without polling the CLI.

Endpoints:
- `GET    /sessions` — list (filter by status, tenant, limit)
- `POST   /sessions` — create `{ tenant_id, title }`
- `GET    /sessions/<id>` — full detail incl. participants +
  pending handoffs
- `PATCH  /sessions/<id>/status` — `{ status: "resolved" |
  "cancelled" }`
- `GET    /handoffs` — list (filter by session, worker,
  status)
- `GET    /handoffs/<id>` — detail
- `PATCH  /handoffs/<id>/status` — `{ status: "acknowledged"
  | "settled" }`

Auth via the existing bearer-token cookie/header used by
D-webinbox (`ARGO_WEBINBOX_BEARER_TOKEN` env at the gateway
side; same value the inbox UI already sends).

## Files you may touch
- `crates/argo-cli/src/handlers/gateway/routes_orchestrator.rs`
  (NEW)
- `crates/argo-cli/src/handlers/gateway/mod.rs` — narrowly
  scoped: ONE `pub mod routes_orchestrator;` + ONE
  `routes.merge(routes_orchestrator::router(...))` line.

## Files you must NOT touch
- Any other `routes_*.rs` in the gateway directory.
- The bearer-token auth middleware (reuse it; don't fork).
- `crates/argo-orchestrator/**` — call the existing persist
  API; don't extend it.

## Acceptance criteria
- [ ] All 7 endpoints implemented + return JSON bodies that
      round-trip via `serde_json` from the type's `Serialize`
      derive.
- [ ] List endpoints accept query params for filtering;
      unknown params return 400 not 200-with-no-filter.
- [ ] PATCH endpoints reject lifecycle backslides (e.g.
      Resolved → Active) with 409 + a body explaining why.
- [ ] Auth: missing/invalid token returns 401; valid token
      with mismatched tenant returns 403.
- [ ] Integration test: spin up a test gateway with an
      in-memory store, hit each endpoint, assert status
      codes + body shapes.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean
- [ ] Every authored file < 500 LOC

## Branch + PR
- Branch: `codex-H-orchestrator-routes`
- PR title: `gateway: HTTP routes for sessions + handoffs (H-orchestrator-routes)`

## Notes
- Mirror the shape of
  `crates/argo-cli/src/handlers/gateway/routes_channels.rs`
  for axum router boilerplate, error handling, JSON envelopes.
- 409 on lifecycle violations matches the rest of the
  gateway's existing convention.
