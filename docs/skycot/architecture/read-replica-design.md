# Read-replica pattern for heavy audit queries

**Status:** design — no code in this commit.
**Brief:** [`mission-read-replica-pattern-design`](../../build-plan/streams/mission-read-replica-pattern-design.md).
**Source:** `MISSION_BACKLOG.md` Section 8 #240.
**Companion ADR:** [`adr-postgres-support-path.md`](../build-plan/streams/adr-postgres-support-path.md)
referenced where it constrains options.

This is the blueprint. Implementation lands across five follow-up
briefs (§6) once the path is agreed.

---

## 1. Why

Audit dashboards on large tenants run heavy queries that contend
with write traffic on the single shared SQLite pool. The current
`SessionStore` holds **one** `Pool<Sqlite>`
(`crates/argo-store/src/lib.rs:231-234`) opened in WAL mode with
`max_connections=8` by default (`pool_config::PoolConfig::from_env`,
`crates/argo-store/src/lib.rs:272-279`). Every gateway read and
every audit-pipeline write competes for the same eight handles.

WAL gives multi-reader / single-writer concurrency, so a hot reader
*does not* block a writer outright. What it does block is **acquire
of a free pooled handle**. Three observable symptoms on busy
tenants:

1. **Pool starvation.** A long-running `export_ndjson` scan
   (`crates/argo-audit/src/export.rs:60-85`, full-table sweep + LEFT
   JOIN, no LIMIT) pins one handle for the duration of the export.
   The 8-handle pool tolerates one such reader; two concurrent
   exports plus a gateway dashboard burst exhausts the pool and new
   acquires time out at 30s.
2. **Page-cache eviction.** Analytical scans push hot writer pages
   out of SQLite's page cache, slowing subsequent writes that now
   have to re-read from disk.
3. **Checkpoint stalls.** WAL grows during long readers because
   pages still referenced by an open snapshot cannot be checkpointed
   back to the main DB. Sustained heavy reads ⇒ unbounded WAL ⇒
   eventual `SQLITE_BUSY` on the next checkpoint.

Read-replicas solve all three by routing analytical reads off the
primary, at the cost of three new design problems: stale-replica
reads, replication lag, query routing.

Today's stop-gaps (the `RAW_FETCH_CAP=5_000` ceiling at
`crates/argo-cli/src/handlers/gateway/routes_audit_feed.rs:29`, the
10k cap in `audit_count_response`,
`crates/argo-cli/src/handlers/gateway/routes_audit.rs:80`) bound the
worst case but do not address the structural contention. They are
load-shedding, not architecture.

---

## 2. Current read patterns

Every audit-read path the gateway exposes today, categorised by
stale-tolerance. Stale-tolerance defines whether the query *can* be
served from a replica that lags the primary by N seconds without
returning a result the operator would consider wrong.

| # | Path | File:line | Pattern | Stale-tolerance |
|---|------|-----------|---------|-----------------|
| 1 | `argo_audit::export_ndjson` | `crates/argo-audit/src/export.rs:60` | Full-table `ORDER BY seq ASC` sweep, no LIMIT, LEFT JOIN session | **Analytical** — any-replica, ≤ 60s lag OK |
| 2 | `argo_audit::tail_rows_filtered` | `crates/argo-audit/src/export.rs:117` | Windowed `ORDER BY seq DESC LIMIT n` subquery + outer reorder | **Real-time** — primary-with-lag-header |
| 3 | `argo_audit::rows_for_session` | `crates/argo-audit/src/export.rs:179` | `WHERE session_id = ? ORDER BY seq ASC` | **Interactive** — primary-with-lag-header |
| 4 | `routes_audit_feed::list_response` (J14) | `crates/argo-cli/src/handlers/gateway/routes_audit_feed.rs:32` | Calls `tail_rows_filtered` with `RAW_FETCH_CAP=5_000` then in-memory filter | **Interactive** — primary-with-lag-header |
| 5 | `routes_audit::audit_count_response` | `crates/argo-cli/src/handlers/gateway/routes_audit.rs:51` | `tail_rows_filtered` cap 10_000 then `.len()` | **Analytical** — any-replica, ≤ 60s lag OK |
| 6 | `routes_audit_rollups::audit_by_session_response` | `crates/argo-cli/src/handlers/gateway/routes_audit_rollups.rs:22` | `GROUP BY session_id ORDER BY COUNT(*) DESC LIMIT n` | **Analytical** — any-replica, ≤ 60s lag OK |
| 7 | `routes_audit_rollups_tools::audit_by_tool_response` | `crates/argo-cli/src/handlers/gateway/routes_audit_rollups_tools.rs:20` | `json_extract($.tool)` + `GROUP BY tool` over `audit_record` | **Analytical** — any-replica, ≤ 60s lag OK |
| 8 | `routes_audit_stream::dispatch_audit_stream` | `crates/argo-cli/src/handlers/gateway/routes_audit_stream.rs:30` | Snapshot of most-recent N rows then live WS broadcast | **Real-time** — primary-only |
| 9 | `routes_audit_session::audit_session_verify_response` | `crates/argo-cli/src/handlers/gateway/routes_audit_session.rs:18` | Walks the per-session hash-chain, verifies linkage | **Real-time** — primary-only (sees torn write otherwise) |

**Stale-tolerance buckets:**

- **Real-time / primary-only.** Result must reflect every committed
  write up to the request's wall-clock moment. Hash-chain verify
  (#9) is the canonical case — a replica that lags by one row
  reports a phantom "break" at the tip. The live audit stream (#8)
  is the same case: the WS broadcast channel writes from the
  primary and the snapshot must align with the next live frame.
- **Interactive / primary-with-lag-header.** Operator-driven
  read on the *most recent* slice. Routed to the primary by default
  so the operator never sees a row vanish between two refreshes,
  but the response carries `X-Replica-Lag-Seconds` so a future
  client variant *could* opt into a replica explicitly (see §5).
  J14 feed (#4), per-session timeline (#3), tail-recent (#2) fit
  here.
- **Analytical / any-replica.** Aggregate, export, or rollup that
  is meaningful even when N seconds stale. Bucketed counts and
  full-table exports (#1, #5, #6, #7) fit here. These are the
  *biggest* offenders by run-time and the highest-value targets
  for replica routing.

The categorisation is per-endpoint, not per-call: the same SQL run
from CLI `argo audit show --session …` vs gateway
`/audit/sessions/<id>` is the same stale-tolerance regardless of
caller, because the stale-tolerance is a property of the
*operator's expectation*, not the transport.

---

## 3. Replication options

Two engines, three viable mechanisms. The choice depends on Tier
(per [adr-postgres-support-path](../build-plan/streams/mission-postgres-support-path-plan.md)).

### 3.1 SQLite — `litestream`

[`litestream.io`](https://litestream.io) is a sidecar that
continuously streams the WAL of a live SQLite database to
S3-compatible object storage and reconstructs read-only replicas
elsewhere from those WAL segments.

- **Topology**: primary process writes to local SQLite + WAL;
  litestream tails the WAL, ships segments to S3; replica processes
  poll S3, replay segments into their own local copy.
- **Lag**: typical 1–10s under steady write load; spikes to 30s on
  WAL checkpoint boundaries.
- **Read-side**: replica opens the local replicated file with WAL
  mode + `query_only=true`. Pool semantics unchanged.
- **Constraints**: replicas are append-only readers; multi-writer
  is **not** supported (litestream is single-primary). Replicas
  see committed writes only after litestream ships the segment
  *and* the replica polls.
- **Fit**: Tier 0 (v1) SQLite primary + ≥ 1 read-only replica for
  analytical traffic. Operator runs `litestream replicate` as a
  systemd unit alongside the gateway.

### 3.2 SQLite — `cr-sqlite`

[`cr-sqlite`](https://github.com/vlcn-io/cr-sqlite) extends SQLite
with CRDT-based multi-writer replication; each peer can write and
the changes converge.

- **Topology**: peer-to-peer; every node is read+write.
- **Lag**: same order as litestream (seconds), bidirectional.
- **Constraints**: every replicated table must be declared a CRDT;
  schemas need rewrite. Hash-chain semantics in `audit_record`
  (parent_hash / self_hash) are *not* CRDT-friendly — a CRDT merge
  could fork the chain. Migration cost is high; semantic mismatch
  is fatal for the chain.
- **Fit**: Rejected for v1. Re-examine only if we ever ship
  multi-writer SaaS on SQLite, which would require a chain redesign
  first.

### 3.3 PostgreSQL — native streaming replication

Once [Tier 1 of `adr-postgres-support-path`](../build-plan/streams/mission-postgres-support-path-plan.md)
lands, Postgres provides built-in streaming replication.

- **Topology**: primary + N read-replicas; replicas use WAL
  streaming over TCP, no sidecar.
- **Lag**: sub-second under steady write load.
- **Read-side**: replicas accept `SELECT` only; writes return
  `read-only transaction` error.
- **Constraints**: requires Tier 1 Postgres support (not yet
  scheduled). Behind a `--features postgres` build flag per the
  ADR. SaaS-only; self-hosted single-tenant operators keep SQLite.
- **Fit**: Tier 1+, when SaaS deployment moves to Postgres.
  Pre-positioned via the `ReadPool` abstraction so the migration is
  a config-only operator change.

### 3.4 Decision

- **Tier 0 (v1 today)**: `litestream` is the chosen path. Replicas
  serve analytical bucket #3 only.
- **Tier 1+ (when Postgres lands)**: native streaming replication.
  Same `ReadPool` abstraction, different `DATABASE_URL` per replica.
- **cr-sqlite**: rejected on hash-chain semantics.

---

## 4. Read-routing pattern

Today the store exposes one pool:

```rust
// crates/argo-store/src/lib.rs:231
pub struct SessionStore {
    pub(crate) pool: Pool<Sqlite>,
    pub(crate) messages: Arc<BatchedWriter<MessageItem>>,
}
```

Every call site reaches into `store.pool()`. The design adds a
second pool surface — the existing `pool` becomes the **write
pool**; a new `read_pool` field carries a fan-out of replica
connections:

```rust
pub struct SessionStore {
    pub(crate) write_pool: Pool<Sqlite>,
    pub(crate) read_pool: ReadPool,    // see below
    pub(crate) messages: Arc<BatchedWriter<MessageItem>>,
}

pub enum ReadPool {
    /// v1 default: replicas not configured; reads go to the primary
    /// pool. Backwards-compatible — every operator who has not
    /// configured `[store.read_replicas]` keeps current behavior.
    Primary(Pool<Sqlite>),
    /// One or more replica pools, round-robin. Empty replica list is
    /// rejected at config-load time.
    Replicas(Vec<Pool<Sqlite>>),
}
```

`SessionStore::pool()` stays — it returns the write pool. A new
`SessionStore::read_pool()` returns the right pool for a
**stale-tolerant** read. Call-site discipline is enforced at code
review (no clippy lint catches it); the §2 table is the
authoritative mapping.

### 4.1 Config surface

New `[store.read_replicas]` block in `argo.toml`:

```toml
[store.read_replicas]
enabled = false                     # default false — primary-only
paths = ["/var/lib/argo/replica1.db"]
max_connections_per_replica = 8
acquire_timeout_ms = 30_000
```

When `enabled = false` (the default), `ReadPool::Primary` is wired
and routing is a no-op. **No behavior change for any operator who
hasn't opted in.**

### 4.2 Routing rules

| Stale-tolerance | Pool | Header behavior |
|-----------------|------|-----------------|
| Real-time / primary-only | `write_pool` | No header |
| Interactive / primary-with-lag-header | `write_pool` | `X-Replica-Lag-Seconds: 0` |
| Analytical / any-replica | `read_pool` (Replicas if configured, else falls through to Primary) | `X-Replica-Lag-Seconds: <measured>` |

### 4.3 Why explicit, not transparent

A "smart router" that inspects SQL to decide pool affinity is
rejected: SQL inspection is brittle, the call sites already know
their semantics, and the §2 categorisation is small enough to
maintain by hand. Every audit query path declares its
stale-tolerance at the call site (a comment + the pool selection),
and reviewers enforce the categorisation in PR review.

---

## 5. Replication-lag awareness

### 5.1 The header

`X-Replica-Lag-Seconds: <integer>` on every gateway response that
served data from a replica or was eligible to. Computed as:

```
lag = now() - max(audit_record.created_at on the replica)
```

This is cheap (single indexed read on the replica) and correct
within ~1s. For Postgres Tier 1+, swap for
`pg_last_xact_replay_timestamp()` which is more precise.

When the read was served from the primary, the header value is
`0`. When the read was served from a replica that's > N seconds
behind a configurable threshold (`store.read_replicas.max_lag_s`,
default 60), the call **falls back** to the primary and emits an
audit row `audit_record(kind="replica_lag_fallback", payload={
configured_max_s, observed_s })`.

### 5.2 Why a header, not a field

Headers don't change response body shape. Existing clients
(`curl`, the SPA, internal tools) keep parsing the same JSON.
Header-aware clients (a future SPA fresh-data toggle) read the
header to decide whether to surface a "data is N seconds stale"
banner.

### 5.3 Falsifiable check

Operator can verify replica health via:

```bash
curl -i https://gateway/audit?range=1h | grep -i replica-lag
# X-Replica-Lag-Seconds: 3
```

A value > `max_lag_s` for sustained periods means a replica is
broken — operator alerts off this dimension.

### 5.4 Out of scope here

- Per-tenant lag thresholds (one global threshold in v1).
- Client-side staleness UX (a separate frontend brief).
- Replica health-checks beyond the lag header (a separate
  operability brief).

---

## 6. Migration sequence — five follow-up briefs

Implementation lands across five briefs, in order. Each one is a
single-PR scope; none rebuilds the core platform.

1. **`mission-store-readpool-split`** — Introduce `ReadPool` enum +
   `SessionStore::read_pool()` accessor. Both pools point at the
   same SQLite file in v1; *no behavior change*, pure plumbing.
   Adds `[store.read_replicas]` skeleton config (still
   `enabled=false` default). ~300 LOC. `crates/argo-store/**`.

2. **`mission-audit-readpool-routing`** — Migrate the §2
   *analytical* call sites (#1, #5, #6, #7) to
   `SessionStore::read_pool()`. Real-time and interactive paths
   stay on the write pool. Still no replicas wired — this brief
   *only* moves the call sites. Verifiable: clippy + audit unit
   tests pass unchanged. ~200 LOC.
   `crates/argo-cli/src/handlers/gateway/routes_audit*.rs`,
   `crates/argo-audit/src/export.rs`.

3. **`mission-litestream-replica-plumbing`** — Operator runbook + a
   sample `litestream.yml` + a startup-time wiring path so
   `[store.read_replicas].paths` opens read-only WAL pools against
   the replicated files. Adds `replica_lag_seconds()` helper that
   queries `MAX(audit_record.created_at)` on each replica. ~400
   LOC + `docs/skycot/runbook/litestream.md`.
   `crates/argo-store/**`, `docs/skycot/runbook/**`.

4. **`mission-replica-lag-header`** — Add `X-Replica-Lag-Seconds`
   to every gateway response on a read-pool path. Implements the
   §5.1 fallback when lag exceeds threshold. Requires the gateway
   response shape to gain a header dict (today it returns a
   2-tuple, see `routes_audit_feed.rs:78`); the shape change is
   the brief's main risk and the reason this is its own brief.
   ~500 LOC across `crates/argo-cli/src/handlers/gateway/**`.

5. **`mission-postgres-readreplica-flag`** — When Tier 1 of
   [adr-postgres-support-path](../build-plan/streams/mission-postgres-support-path-plan.md)
   lands, extend `[store.read_replicas].paths` to accept Postgres
   DSNs behind `--features postgres`. The `ReadPool::Replicas`
   variant generalises to `Vec<Pool<DB>>` over `DB: sqlx::Database`.
   Depends on `mission-postgres-support-path-plan` Tier 1
   shipping; *not scheduled* until that prerequisite advances
   past approved-in-principle. ~600 LOC.

Dependency order: 1 → 2 → 3 → 4. Brief 5 depends on Tier 1
Postgres, which is itself unscheduled.

---

## 7. Non-goals (in this brief)

- **No multi-writer replication.** cr-sqlite is rejected; we keep
  single-writer semantics throughout.
- **No automatic failover from primary to replica.** Replicas
  serve reads only; if the primary dies, the gateway dies too.
  Failover is a separate operational topic.
- **No per-tenant replica affinity.** All tenants share the same
  replica fleet in v1. Per-tenant routing is a Tier 2 concern
  alongside SaaS-Postgres.
- **No backfill of analytical reads from cold storage.** Replicas
  are warm replicas of *live* data, not archive readers.

---

## 8. Open questions for the operator

1. Does the v1 SQLite-litestream path go to S3 or to local disk
   on a sibling machine? Litestream supports both; the decision
   affects RPO (recovery-point objective) and operator cost.
2. Is one replica enough, or should we plan for ≥ 2 from day one?
   Two replicas adds redundancy but complicates the lag-header
   compute (which replica did we read from?).
3. Should `max_lag_s` be per-endpoint or one global threshold?
   Per-endpoint is the right long-term shape; one-global is fine
   for v1.

These questions are open by design — the answer should come from
the first SaaS-scale tenant's operator experience, not from
upfront design.

---

**End of design doc.** Implementation begins with
`mission-store-readpool-split` once this blueprint is approved.
