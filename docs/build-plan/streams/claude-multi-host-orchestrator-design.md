# V2.x — Multi-host orchestrator design + POC

**Owner**: claude. **Depends on**: H-orchestrator-persist (#806),
H-orchestrator-routes (#827) merged.

## Scope (deliberately bounded)

Today Argo runs single-host: conductor + gateway + sandbox
all on one machine. For "team of workers across multiple
operators / multiple Macs" we need a distributed topology.

This brief ships **design doc + 2-host POC**, NOT the full
distributed system.

1. **Design doc** at
   `docs/skycot/MULTI_HOST_ORCHESTRATOR.md` covering: (a)
   conductor topology (single primary vs multi-primary with
   leader election), (b) ledger sync (git-as-truth still
   works for builds; runtime needs different — likely
   sqlite WAL streaming OR S3-backed event log), (c)
   worker discovery (how does host A find workers on host B),
   (d) action dispatch (where does a session run when its
   worker is on a different host), (e) split-brain handling.
2. **2-host POC** — extend argo-gateway with a `--peer
   <host>` flag. When set, gateway joins a peer mesh:
   periodically fetches peer's session list + replicates
   audit records via a new `/peer/sync` endpoint. POC
   proves cross-host session visibility (operator on host A
   can see/inspect a worker running on host B).
3. **One smoke test** — spawns 2 gateways on 127.0.0.1
   ports, peers them, creates a session on gateway-1, asserts
   gateway-2 sees it.

Follow-up briefs (NOT in scope):
- Cross-host action dispatch
- Leader election for write authority
- Conflict resolution

## Files you may touch
- `docs/skycot/MULTI_HOST_ORCHESTRATOR.md` (NEW design doc)
- `crates/argo-gateway/src/peer.rs` (NEW)
- `crates/argo-gateway/src/routes_peer.rs` (NEW —
  /peer/sync endpoint)
- `crates/argo-gateway/src/lib.rs` — narrowly: register peer
  module + flag
- `crates/argo-gateway/tests/peer_smoke.rs` (NEW)
- BLANKET CLI ALLOWLIST applies for any CLI plumbing

## Acceptance criteria
- [ ] Design doc covers 5 architectural questions in scope
- [ ] PoC: 2 gateways peer + cross-host session visibility
- [ ] /peer/sync round-trip works (audit records replicate)
- [ ] Bearer auth between peers (shared secret)
- [ ] Graceful handling of peer-unreachable (no crashes)
- [ ] Smoke test passes
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean

## Branch + PR
- Branch: `claude-multi-host-orchestrator-design`
- PR title: `gateway: multi-host orchestrator design + 2-host POC (claude-multi-host-orchestrator-design)`
