# Multi-host orchestrator — design + 2-host POC

**Status**: V2.x design doc + POC. Not production-ready replication.
**Scope**: visibility across 2+ hosts. Cross-host action dispatch,
leader election, and conflict resolution are explicit non-goals here.

## Why

Today Skycot (Argo) runs single-host: a conductor process,
HTTP gateway, sandbox, and SQLite store all live on one machine.
A "team of operators each running their own Mac" is the immediate
multi-host driver — operator A wants to see workers / sessions /
audit running on operator B's machine without copying SQLite files
around.

## The five questions

### (a) Conductor topology

Three plausible shapes; we adopt the first for V2.x and keep the
others on the roadmap.

1. **Single primary, read-only replicas (V2.x default).** Exactly
   one host owns write authority for each session. Every other
   host is a read-only mirror that pulls audit + session headers
   over `/peer/sync`. No leader election: the primary is the host
   the session was created on, and that ownership never moves.
   Failure mode is bounded — if the primary is down, replicas
   serve stale data; nothing is lost, no split-brain is possible.

2. **Multi-primary with leader election (deferred).** Per-session
   leadership rotates via a Raft-like protocol (likely `raft-rs`
   or a thinner `etcd`-style handoff). Required once we want
   cross-host action dispatch — the host that owns the worker
   process must also own the session writes — but introduces
   split-brain risk and ~3× the operational surface area.

3. **Stateless gateways + shared backend (deferred).** All
   gateways front a single shared SQLite or Postgres tier. Trades
   the distribution problem for an HA-storage problem. Reasonable
   for hosted Skycot Cloud, wrong shape for the operator-laptop
   default.

V2.x ships #1. Promotion to #2 is gated on a concrete cross-host
dispatch use case — currently absent.

### (b) Ledger sync

Two ledgers in play, with very different consistency needs.

- **Build ledger** (`docs/build-plan/BUILD_STATUS.md`): git-as-truth
  works fine. Every conductor and worker pulls/pushes against the
  same `origin/main`, the swarm protocol's HARD RULE 7
  (race-lost retry) handles non-fast-forwards. No change.

- **Runtime ledger** (`session`, `audit_record`, `message`,
  `tool_call` rows in SQLite): git is the wrong tool — millions
  of rows, append-mostly, 50ms-tail latency expectations. Three
  candidates:

  | Approach | Wire | Pros | Cons |
  |---|---|---|---|
  | **HTTP pull (V2.x POC)** | `GET /peer/sync` | trivial; reuses bearer auth; debuggable with curl | polling latency 5s default; per-record overhead |
  | SQLite WAL streaming | rsync / litestream | binary fidelity; near-realtime | requires shared filesystem or signed object store |
  | Append-only event log | S3-backed (deltastream / Iceberg) | single source of truth; replay-able | new infra dependency; not laptop-friendly |

  V2.x ships HTTP pull. Litestream-style WAL replication is the
  natural V2.y upgrade once the operator base outgrows polling
  (likely at ≥10 hosts or sub-second visibility requirements).

### (c) Worker discovery

The POC uses a static peer list: each gateway is launched with
`ARGO_GATEWAY_PEERS=http://host-a:7400,http://host-b:7400` (or a
repeatable `--peer` flag). Discovery is whatever the operator
configures; we deliberately do not auto-discover.

Future shapes considered and rejected for V2.x:

- **mDNS / Bonjour** — works on a single LAN, no good story for
  Tailscale / VPN / cross-Internet topology.
- **Centralized registry** (Consul / etcd) — solves discovery,
  reintroduces a shared-tier dependency.
- **Skycot Cloud directory** — viable but couples local operator
  use to a hosted control plane. Defer until Skycot Cloud exists.

Static config is the right primitive: it composes with whatever
discovery mechanism operators actually want (DNS round-robin,
hand-edited file, Tailscale ACLs) without forcing a choice.

### (d) Action dispatch

**V2.x answer: no cross-host dispatch.** A session runs entirely
on the host where it was created. Replicas can `GET /sessions/<id>`
to inspect, but `POST /sessions/<id>/runs` is rejected with HTTP
409 Conflict + a redirect-style header (`X-Argo-Primary:
http://host-a:7400`). The CLI follows the redirect transparently;
the operator sees the session run on its primary host.

V2.y will lift this once leader election lands. The 409+redirect
shape is forward-compatible: a future leader-elected mesh can
return the new leader's URL in the same header.

### (e) Split-brain handling

V2.x is split-brain-free **by construction** — single primary per
session, no automatic failover, no leadership rotation. The cost
is a hard durability boundary: if a primary dies with unflushed
writes, those writes are lost. Acceptable for V2.x because (a)
SQLite WAL fsync is on by default, narrowing the loss window to
the in-flight handler, and (b) operator workflows tolerate
restart-and-resume.

V2.y must address split-brain when leader election lands. The
plausible shape: per-session epochs tagged on every audit-record
write, conflict resolution via "highest epoch wins, lower-epoch
writes archived to a quarantine table." Out of scope here.

## POC: what V2.x ships

A 2-host visibility mesh, nothing more.

1. **Peer mesh client** (`peer.rs`). Reads
   `ARGO_GATEWAY_PEERS` (comma-separated URLs), launches a tokio
   background task that polls each peer every
   `ARGO_PEER_POLL_MS` (default 5000). Caches the most recent
   `/peer/sync` response per peer in a process-local map. Bearer
   auth via `ARGO_PEER_SECRET`.

2. **Sync endpoint** (`routes_peer.rs`).

   - `GET /peer/sync` — returns local recent sessions + audit
     records (limit 50 / 100). Bearer-gated by
     `ARGO_PEER_SECRET`.
   - `POST /peer/poke` — hint that the sender just produced new
     state; receiver schedules an out-of-band fast pull. No body.
     Bearer-gated.
   - `GET /peer/state` — debug/inspect: returns the cached
     snapshot of every configured peer plus last-sync timestamp
     and last-error string. NOT bearer-gated (local introspection
     only — the body never includes bearer tokens).

3. **Peer-sync hint**. When a session is created locally (`POST
   /sessions` etc.), the dispatcher emits a fire-and-forget
   `POST /peer/poke` to every configured peer. Faster than the
   periodic poll (typically <100ms vs 5s).

4. **Smoke test** (`tests/peer_smoke.rs`). Spawns two gateways
   on `127.0.0.1` ephemeral ports, configures them to peer with
   each other (poll interval 100ms), creates a session on g1,
   asserts g2's `/peer/state` surfaces it within ≤2s.

### Failure handling

- **Peer unreachable.** The poll loop logs once per failure
  transition, exponential-backs the next attempt up to a 60s
  ceiling, and clears the per-peer cached snapshot's "stale"
  field. No crash, no retry storm.
- **Bad / missing bearer secret.** `/peer/sync` returns 401. The
  poll loop records the error in the per-peer snapshot. The
  gateway as a whole is unaffected.
- **Schema drift.** The wire format is `serde_json` of
  `SessionHeader` and a thin `AuditWire` struct. Adding fields is
  forward-compatible (`#[serde(default)]`). Removing fields is a
  V2.y migration.
- **Clock skew.** `/peer/sync` includes a `local_micros` field
  in the response so the client can compute drift; nothing in
  V2.x acts on it. Future leader election will need monotonic-
  clock or Lamport-clock primitives anyway.

## Non-goals (recorded explicitly so V2.y briefs can pick them up)

- Cross-host action dispatch (the 409+redirect contract is
  defined; the runtime support is not).
- Leader election / Raft / leadership handoff.
- WAL streaming / Litestream integration.
- Conflict resolution / quarantine tables.
- Auto-discovery (mDNS, Tailscale ACLs, hosted directory).
- Cross-host RBAC propagation (assumed: every host shares the
  same role bindings out of band; future briefs will tighten).

## Wire reference

### `GET /peer/sync`

```jsonc
{
  "epoch_micros": 1715293812345678,
  "sessions": [
    {
      "id": "uuid",
      "tenant_id": "uuid",
      "workspace_id": "uuid",
      "model": "...",
      "provider": "...",
      "title": "...",
      "started_at": "2026-05-10T14:22:01Z",
      "ended_at": null,
      "vertical_id": null,
      "worker_id": null
    }
  ],
  "audit": [
    {
      "seq": 12345,
      "kind": "ToolCall",
      "payload_json": "...",
      "created_at": 1715293812345678
    }
  ]
}
```

### `POST /peer/poke`

Empty body. Receiver responds `200 OK` and schedules a fast pull.
Body of the response is `{"status":"poked"}` for symmetry with
the rest of the gateway's JSON envelope.

### `GET /peer/state`

```jsonc
{
  "self_url": "http://127.0.0.1:7400",
  "peers": [
    {
      "url": "http://host-b:7400",
      "last_sync_micros": 1715293812345678,
      "last_error": null,
      "session_count": 12,
      "audit_count": 87
    }
  ]
}
```

## Configuration

| Env var | Default | Purpose |
|---|---|---|
| `ARGO_GATEWAY_PEERS` | unset | Comma-separated peer URLs (`http://h1:7400,http://h2:7400`). Empty = mesh disabled. |
| `ARGO_PEER_SECRET` | unset | Shared bearer token for `/peer/sync` + `/peer/poke`. Required when `ARGO_GATEWAY_PEERS` is set. |
| `ARGO_PEER_POLL_MS` | 5000 | Periodic poll interval, in milliseconds. |
| `ARGO_PEER_TIMEOUT_MS` | 3000 | Per-request HTTP timeout for outbound peer calls. |
