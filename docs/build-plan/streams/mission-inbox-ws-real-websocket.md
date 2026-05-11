# Mission load-bearing wire — `/inbox/ws` real WebSocket (replace meta-refresh sham)

**Owner**: claude. **Depends on**: existing `/inbox/ws` route stub (verified at `crates/argo-cli/src/handlers/gateway/routes_channels_webinbox.rs:29`); `/audit/stream` mirrors the same shape (`routes_audit_stream.rs`).
**Authored against**: origin/main `da6415b0` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-inbox-ws-real-websocket`.

> SOURCE: MISSION_BACKLOG.md Section 12 #261 — "Fix `/inbox/ws` to be real WebSocket." Today's "websocket" endpoint at `routes_channels_webinbox.rs:48` just serves an HTML stub with a 5-second meta-refresh `<script>` polling loop. Mission promise: a real WS upgrade so inbox events push to the operator within ~100ms instead of the 5s polling lag.

## Scope

1. **Replace `inbox_ws` handler** with a proper WebSocket upgrade:
   - Detect `Upgrade: websocket` header.
   - Compute the `Sec-WebSocket-Accept` (SHA1+base64 of the client key + magic GUID).
   - Send 101 Switching Protocols.
   - Loop: read frames, hold connection open, push event frames as approval/inbox events occur.

2. **Backend**: subscribe to existing inbox event channel (likely a `tokio::sync::broadcast` — verify via `grep -rn 'broadcast' crates/argo-cli/src/handlers/gateway/routes_channels_webinbox.rs`). Each broadcast event → one WS text frame `{"event":"<kind>","approval_id":"...",...}`.

3. **Auth**: per ledger amendment, `/inbox/ws` is bearer-authenticated. Verify the Authorization header BEFORE accepting the upgrade. 401 if missing/invalid; refuse the upgrade.

4. **Keepalive**: send a PING frame every 30s; if no PONG within 10s, close the connection.

5. **Backpressure**: if the broadcast subscriber lags (Lagged error), close the connection with code 1011 + reason "lagged". Client reconnects.

6. **Update existing inline `<script>` poll-loop** in `/inbox` to actually OPEN the WebSocket instead of just including the URL. Verify the existing meta-refresh fallback stays if WS fails to connect.

7. **Same fix for `/audit/stream`** (per `routes_audit_stream.rs` mirroring `/inbox/ws` shape).

This is NOT in scope: WSS / TLS termination (operator's reverse proxy handles it); per-client rate-limiting; multi-server scale-out (single-process broadcast is fine for v1); arbitrary client-server messages (server-push only).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes_channels_webinbox.rs` — narrowly scoped: replace `inbox_ws` body. File currently ≤ 200 LOC; stay under 400.
- `crates/argo-cli/src/handlers/gateway/routes_audit_stream.rs` — same WS-upgrade replacement.
- `crates/argo-cli/src/handlers/gateway/ws_upgrade.rs` (NEW, ≤ 250 LOC) — shared WS-upgrade helper: handshake, accept-key compute, frame read/write, keepalive.
- `crates/argo-cli/src/handlers/gateway/ws_upgrade_tests.rs` (NEW, sibling, ≤ 200 LOC) — 7 unit tests: accept-key-rfc6455-vector; bad-key-rejected; missing-upgrade-rejected; auth-missing-401; auth-bad-401; ping-pong-roundtrip; lagged-closes-1011.
- `crates/argo-cli/tests/inbox_ws_smoke.rs` (NEW, ≤ 200 LOC) — 1 integration smoke: boot gateway, open WS, trigger an inbox event, assert frame received within 100ms.
- `crates/argo-cli/Cargo.toml` — narrowly scoped: add `sha1` + `base64` IF not already present.

## Files you must NOT touch

- `crates/argo-cli/src/handlers/gateway/auth.rs` (or wherever bearer-auth lives) — reuse existing verifier
- The broadcast event source — reuse only
- `crates/argo-store/migrations/**` — no schema change
- Any non-gateway code

## Acceptance criteria

- [ ] `Upgrade: websocket` request gets 101 Switching Protocols
- [ ] `Sec-WebSocket-Accept` matches RFC 6455 vector (`dGhlIHNhbXBsZSBub25jZQ==` → `s3pPLMBiTxaQ9kYGzzhZRbK+xOo=`)
- [ ] Missing/bad bearer → 401, no upgrade
- [ ] Inbox events propagate to connected WS within 100ms
- [ ] PING/PONG keepalive at 30s interval
- [ ] Lagged subscriber → close code 1011 "lagged"
- [ ] Existing `<script>` poll-fallback still works if WS fails
- [ ] `/audit/stream` gets the same fix
- [ ] 7 unit tests + 1 integration smoke
- [ ] No new public API outside the listed exports
- [ ] `cargo build -p argo-cli` clean
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-inbox-ws-real-websocket`
- PR title: `gateway: real WebSocket for /inbox/ws + /audit/stream (mission-inbox-ws-real-websocket)`

## Notes for the implementer

- The WS upgrade handshake is well-specified in RFC 6455 §1.3. Use the published test vectors (`dGhlIHNhbXBsZSBub25jZQ==` → `s3pPLMBiTxaQ9kYGzzhZRbK+xOo=`) in your unit tests.
- For frame handling: a small inline implementation is fine — text + ping + pong + close are the only frame types you need. Don't pull in `tokio-tungstenite` unless the workspace already has it.
- The `tokio::sync::broadcast` may already exist for inbox events — REUSE rather than create a new channel. Verify with `grep -rn 'broadcast::channel' crates/argo-cli/src/handlers/gateway/`.
- The keepalive interval is configurable in CLI flag form for future ops; v1 hardcoded 30s is fine.
- Don't try to implement WS extensions (compression, custom protocols). Plain RFC 6455 text frames only.
