# V2.x — Multi-host orchestrator design + POC

**Owner**: claude. **Depends on**: H-orchestrator-persist (#806),
H-orchestrator-routes (#827) merged.

> **REWRITE NOTE (tick 269 by conductor)**: original brief pointed
> to `crates/argo-gateway/src/peer.rs` + `routes_peer.rs`, but the
> `argo-gateway` crate hosts chat-platform inbound adapters and
> contains NO HTTP server. The actual HTTP gateway server lives
> at `crates/argo-cli/src/handlers/gateway/`. This rewrite moves
> all paths under `crates/argo-cli/src/handlers/gateway/` and
> preserves the V2.x distributed-conductor intent.

## Scope (deliberately bounded)

Today Argo runs single-host: conductor + gateway + sandbox all on
one machine. For "team of workers across multiple operators /
multiple Macs" we need a distributed topology.

This brief ships **design doc + 2-host POC**, NOT the full
distributed system.

1. **Design doc** at
   `docs/skycot/MULTI_HOST_ORCHESTRATOR.md` covering: (a)
   conductor topology (single primary vs multi-primary with
   leader election), (b) ledger sync (git-as-truth still works
   for builds; runtime needs different — likely sqlite WAL
   streaming OR S3-backed event log), (c) worker discovery (how
   does host A find workers on host B), (d) action dispatch
   (where does a session run when its worker is on a different
   host), (e) split-brain handling.
2. **2-host POC** — extend the HTTP gateway with a `--peer
   <host>` flag (or `ARGO_GATEWAY_PEERS` env var). When set,
   gateway joins a peer mesh: periodically fetches peer's
   session list + replicates audit records via a new
   `/peer/sync` endpoint. POC proves cross-host session
   visibility (operator on host A can see/inspect a worker
   running on host B).
3. **One smoke test** — spawns 2 gateways on 127.0.0.1 ports,
   peers them, creates a session on gateway-1, asserts gateway-2
   sees it.

Follow-up briefs (NOT in scope):
- Cross-host action dispatch
- Leader election for write authority
- Conflict resolution

## Files you may touch

- `docs/skycot/MULTI_HOST_ORCHESTRATOR.md` (NEW design doc)
- `crates/argo-cli/src/handlers/gateway/peer.rs` (NEW — peer
  mesh client: periodic sync loop, bearer auth, peer registry;
  ≤ 480 LOC)
- `crates/argo-cli/src/handlers/gateway/routes_peer.rs` (NEW —
  HTTP handler for `/peer/sync`; ≤ 300 LOC)
- `crates/argo-cli/src/handlers/gateway/mod.rs` — narrowly: 2
  pub mod lines
- `crates/argo-cli/src/handlers/gateway/routes.rs` — narrowly:
  call `routes_peer::register(...)` to mount `/peer/*`
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_post.rs`
  — narrowly: emit a peer-sync hint when a session is created
  (so peers can pick it up faster than the periodic poll)
- `crates/argo-cli/tests/peer_smoke.rs` (NEW — 2-host loopback
  smoke; ≤ 280 LOC)
- BLANKET CLI ALLOWLIST applies for the `--peer <host>` flag
  plumbing through `cli.rs` / `main.rs` / `cli_subcommands.rs`

## Files you must NOT touch

- `crates/argo-gateway/` — that crate handles chat-platform
  inbound adapters (Slack/WhatsApp/etc.); peer mesh belongs on
  the HTTP gateway, NOT there
- argo-store schema — POC reuses existing audit/session tables;
  no migrations
- Any existing `routes_*.rs` file other than the narrow
  `routes.rs` + `routes_dispatch_post.rs` mount-point edits

## Acceptance criteria

- [ ] Design doc covers all 5 architectural questions in scope
- [ ] PoC: 2 gateways peer + cross-host session visibility
- [ ] `/peer/sync` round-trip works (audit records replicate
      bidirectionally)
- [ ] Bearer auth between peers (shared secret via
      `ARGO_PEER_SECRET` env var)
- [ ] Graceful handling of peer-unreachable (no crashes; log +
      backoff)
- [ ] Smoke test passes: 2 gateways on 127.0.0.1 → create
      session on g1 → g2 sees it within poll window
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-multi-host-orchestrator-design`
- PR title: `gateway: multi-host orchestrator design + 2-host POC (claude-multi-host-orchestrator-design)`

## Notes for the implementer

- The HTTP gateway's mount-point in `routes.rs` is the single
  source of route registration. Keep peer routes self-contained
  behind a `routes_peer::register(router)` helper so `routes.rs`
  only gets a 1-line addition (preserves the routes.rs LOC
  budget).
- The smoke test should use ephemeral ports (`:0` then read back
  the bound port) so it doesn't conflict with a real gateway.
- Bearer auth: shared-secret with constant-time comparison.
  Don't reach for JWT here — POC scope.
- Periodic poll interval: 5s default, override via env var.
