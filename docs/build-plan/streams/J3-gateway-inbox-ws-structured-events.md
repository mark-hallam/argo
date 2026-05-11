# Stream J3 — Gateway `/inbox/ws` structured events (replaces reload signal with typed events)

**Owner**: claude. **Depends on**: J1 MERGED (events reference the same `ApprovalRow` shape).
**Authored against**: origin/main `bb9b6238` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh J3-gateway-inbox-ws-structured-events`.
**Tier tag**: PRODUCT_ROADMAP.md Track J brief 3 of 18 — unblocks frontend E1-f from cache.invalidate-on-any-event fallback to surgical cache updates.

## v1 posture vs enterprise target

| Concern | v1 implementation | Architectural target | Closes via |
|---|---|---|---|
| Auth | Query-param JWT (browser WebSocket limitation); existing bearer token shipped as JWT | Per-tenant signing key + key rotation history; verifier checks active-at-connection-time key | `mission-enterprise-ws-jwt-rotation` |
| Replay protection | Connection-time nonce + monotonic event sequence number; client drops out-of-order | Server-side replay-window enforcement with nonce de-dup | `mission-enterprise-ws-replay-protection` |
| Backwards compatibility | Existing reload-signal stays alive on a different message format ("hello world" reload still works); structured events on the same socket | Migrate legacy reload-signal consumers to structured events; deprecate the bare reload | `mission-enterprise-ws-legacy-deprecation` |
| Event types | 4 events: `approval.created` / `approval.decided` / `approval.undone` / `approval.expired` | Plus `approval.expired_warning` (30s before TTL); `approval.attention` (high-priority routing) | `mission-enterprise-ws-event-vocabulary` |
| Server-side fan-out | Per-tenant broadcast channel — every connection in tenant X receives X's events | Per-Principal filtering (only events the viewer's RBAC allows) | `mission-enterprise-ws-rbac-filter` |
| Quality-of-service | Best-effort; missed events on disconnect → client refetches on reconnect | At-least-once with server-side replay cursor for short windows | `mission-enterprise-ws-replay-cursor` |

v1 is acceptable: structured events are the
architectural primitive; v1 ships 4 standard types
that cover the daily-driver UX flows. Tenant
broadcast + client refetch-on-reconnect covers the
durability gap.

## Why

The existing `/inbox/ws` route (per
`crates/argo-cli/src/handlers/gateway/routes_channels_webinbox.rs:29`)
sends a generic "reload" signal — the HTML inbox
just re-fetches the entire page on any signal.
E1-f's frontend brief consumes structured events
(`approval.created`, etc.) for surgical cache
updates without full refetches. Without this brief,
E1-f falls back to cache.invalidate-on-any-event —
functional but inefficient.

For enterprise: large workspaces (1000+ approvals)
benefit substantially from surgical updates over
full refetches. v1 ships the event vocabulary; v1.5
hardens with replay-protection + RBAC filtering.

## Scope

- Extend `/inbox/ws` to emit structured JSON events
  alongside the existing reload signal.
- 4 event types in v1:
  - `approval.created` — new approval entered the
    inbox.
  - `approval.decided` — approve or refuse on an
    existing approval.
  - `approval.undone` — undo within window.
  - `approval.expired` — approval timed out
    without a decision.
- Per-tenant broadcast: each connection receives
  events for its tenant only.
- Connection auth via `?token=<JWT>` query param
  (browser WebSocket can't send headers).
- Monotonic per-connection `seq` number on each
  event for client-side ordering.
- Connection-time `hello` event with current
  protocol version + capability map.
- Backward compat: the existing reload-signal
  consumers (HTML inbox) keep working.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Shared hotspots:
  - `crates/argo-cli/src/handlers/gateway/routes_channels_webinbox.rs`
    (the WS route handler — narrowly extend).
  - `crates/argo-channels/src/webinbox.rs` (the
    state machine — add event-emission hooks).

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/ws_inbox_events.rs`
  — NEW. The structured-event protocol + emitter.
  ≤ 350 LOC.
- `crates/argo-cli/src/handlers/gateway/ws_inbox_events_tests.rs`
  — NEW. ≥ 10 tests. ≤ 400 LOC.
- `crates/argo-cli/src/handlers/gateway/routes_channels_webinbox.rs`
  — narrowly scoped: extend the WS handler to
  use the new emitter. Existing reload-signal
  behavior preserved. ≤ 250 LOC after.
- `crates/argo-channels/src/webinbox.rs` —
  narrowly scoped: add 4 hook calls to the
  existing state-transition functions
  (`on_approval_created`, `on_decision`,
  `on_undo`, `on_expired`). ≤ 350 LOC after.
- `crates/argo-cli/src/handlers/gateway/jwt.rs`
  — NEW. JWT decode + verify against the
  shipped public key. ≤ 200 LOC.
- `crates/argo-cli/src/handlers/gateway/jwt_tests.rs`
  — NEW. ≥ 6 tests. ≤ 200 LOC.

## Files you must NOT touch

- Other Track-J handler files.
- `crates/argo-runtime/**` — read-only.
- The approval state machine itself (just add
  hooks).

## Acceptance criteria

- [ ] WebSocket connection to
      `wss://<gateway>/inbox/ws?token=<JWT>`:
      - Validates JWT against the shipped public
        key.
      - Tenant-scopes the connection from JWT
        claims.
      - Returns 401 on invalid/expired/missing
        JWT.
- [ ] Connection-time `hello` event sent
      immediately on accept:
      ```json
      {
        "type": "hello",
        "protocol_version": "1.0",
        "capabilities": ["approval.created", "approval.decided", "approval.undone", "approval.expired"],
        "seq": 0
      }
      ```
- [ ] When a new approval is created server-side,
      ALL connections in the same tenant receive:
      ```json
      {
        "type": "approval.created",
        "seq": <n+1>,
        "ts": "<ISO>",
        "payload": <ApprovalRow JSON>
      }
      ```
- [ ] `approval.decided` event on decide; payload
      = the updated ApprovalRow.
- [ ] `approval.undone` event on undo; payload =
      the reverted ApprovalRow.
- [ ] `approval.expired` event on TTL expiry;
      payload = the expired ApprovalRow.
- [ ] Sequence numbers monotonically increment
      per-connection (server tracks).
- [ ] Existing reload-signal consumers (HTML
      inbox) continue to function:
      legacy text-frame `"reload"` messages still
      sent on state changes.
- [ ] Per-tenant isolation: no cross-tenant event
      leak (property test asserts).
- [ ] PII redaction applies to event payloads
      (via the J2 fence).
- [ ] 10 + 6 = 16 unit tests pass.
- [ ] `cargo build -p argo-cli` clean.
- [ ] `cargo clippy -p argo-cli --all-targets --
      -D warnings` clean.
- [ ] Every file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-J3-gateway-inbox-ws-structured-events`
- PR title: `gateway: /inbox/ws structured events (J3)`

## Notes for the implementer

- **JWT format.** Use the existing bearer-token
  contract (from E0) wrapped in a JWT:
  - Header: `alg: EdDSA, typ: JWT`
  - Payload: `sub: <principal_id>, tnt: <tenant_id>,
    iat: <unix>, exp: <unix+30m>`
  - Signature: Ed25519 over `<header>.<payload>`.

  Reuse the verify-cli's public-key infrastructure
  (E5-f) for consistency. Server signs the JWT on
  bearer-auth success; client passes via `?token=`.

- **Hello event MANDATORY** as the first message.
  Clients use it to negotiate protocol version. If
  the client's expected version differs from
  server's, client can disconnect + report. v1
  doesn't enforce version negotiation; the protocol
  is single-version "1.0".

- **Per-connection state.** Track `seq` per
  connection in a small struct. No shared state
  across connections.

- **Per-tenant broadcast.** Use tokio broadcast
  channel keyed by tenant_id. Each tenant has its
  own channel; new connection subscribes to its
  tenant's channel. Disconnection cleanly drops
  the subscription.

- **State-machine hooks.** Add 4 hook calls in
  `argo-channels::webinbox`:
  ```rust
  pub fn on_approval_created(...) {
      // existing logic
      events::broadcast(tenant_id, Event::ApprovalCreated {
          payload: row.clone(),
      });
  }
  ```
  Etc for decided, undone, expired.

- **Event JSON wire format** (locked):
  ```json
  {
    "type": "approval.created",
    "seq": 42,
    "ts": "2026-05-12T08:24:00Z",
    "payload": { /* ApprovalRow */ }
  }
  ```

- **Backwards-compat reload signal.** Continue
  sending the legacy text-frame
  `"reload"` message alongside structured events.
  Old consumers receive both; new consumers
  ignore the text frame and process the JSON.
  v1.5 deprecation brief removes the text frame
  once all consumers migrate.

- **PII redaction integration.** Apply
  `redact_approval_row(row, principal)` from J2
  BEFORE emitting the event payload to a specific
  connection. Note: different connections in the
  same tenant may have different `permissions`,
  so the broadcast emits raw payloads and each
  subscriber redacts on send. v1 may simplify by
  redacting per-tenant-default policy; v1.5 does
  per-connection per the hardening brief.

- **JWT key.** v1 ships the same Ed25519 key as
  the verify-cli (E5-f). Operators rotate the key
  via the v1.5 hardening brief.

- **Heartbeat.** Server sends a `ping` text frame
  every 30s; client must respond with `pong`
  within 10s or server force-closes. Reuses the
  WebSocket ping/pong feature; no JSON frame
  needed.

## Hardening briefs implied (Track H)

NOT blocking this PR:

- `mission-enterprise-ws-jwt-rotation` — per-
  tenant signing key with rotation history.
- `mission-enterprise-ws-replay-protection` —
  nonce + server-side replay-window.
- `mission-enterprise-ws-legacy-deprecation` —
  deprecate the bare reload-signal.
- `mission-enterprise-ws-event-vocabulary` —
  expand event types (warning / attention /
  routing).
- `mission-enterprise-ws-rbac-filter` — per-
  Principal event filtering.
- `mission-enterprise-ws-replay-cursor` —
  server-side replay cursor for short windows.
