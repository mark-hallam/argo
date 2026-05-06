# Claude — Real-time audit stream over WebSocket

**Owner**: claude. **Depends on**: nothing (independent —
the audit crate already exists; this brief just exposes a
streaming view).

## Scope

The argo-audit crate writes audit rows synchronously today.
Operators querying `argo audit query` get a snapshot. For
v1 demo polish + production diagnostics, an operator
watching the workflow execute live wants to SEE rows
appear as they're written without polling.

This brief adds:
- `GET /audit/stream` WebSocket endpoint on the gateway
  that pushes every newly-written audit row as a JSON
  message
- Optional `?kind=action_emitted,auto_approved` query
  filter so the operator can subscribe to a subset
- Optional `?worker=<id>` filter for per-worker streams
- Server-side auth via the existing bearer-token cookie

## Files you may touch
- `crates/argo-cli/src/handlers/gateway/routes_audit.rs`
  (extend if exists, NEW if not)
- `crates/argo-cli/src/handlers/gateway/mod.rs` — narrowly:
  ONE router merge if needed
- `crates/argo-audit/src/sink.rs` — narrowly: extend with
  a tokio broadcast::Sender so the WebSocket route can
  subscribe without polling. Do NOT change the existing
  on-disk write path.
- `crates/argo-audit/src/lib.rs` — narrowly: re-export
  the broadcast handle

## Files you must NOT touch
- The audit row format / chain hash logic — append-only,
  fixed.
- Any other gateway route file.
- argo-cli main / startup (the broadcast channel
  initializes lazily on first subscription).

## Acceptance criteria
- [ ] WebSocket endpoint accepts connections + streams
      live audit rows
- [ ] Filter params (`kind=`, `worker=`) work + reject
      invalid values with 400 at handshake
- [ ] Auth: missing token → 401; invalid → 401; mismatched
      tenant → 403
- [ ] When 0 audit rows are written for 30s, the server
      sends a heartbeat ping (keeps mid-tier proxies from
      dropping the connection)
- [ ] Integration test that writes 5 audit rows + asserts
      a live subscriber received all 5 in order
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean
- [ ] Every authored file < 500 LOC

## Branch + PR
- Branch: `claude-realtime-audit-stream`
- PR title: `audit: WebSocket /audit/stream for live row push (claude-realtime-audit-stream)`

## Notes
- Look at `crates/argo-cli/src/handlers/gateway/routes_channels.rs`
  for the existing WebSocket pattern (D-webinbox uses
  `/inbox/ws` with the same auth). Mirror the shape.
- broadcast channel cap: 256 messages. If a slow
  subscriber falls behind, drop them with a clear close
  reason — never block the audit-write path.
- This is the v1 observability story for live demos.
  Pairs naturally with claude-skycot-runbook's
  "observability" section.
