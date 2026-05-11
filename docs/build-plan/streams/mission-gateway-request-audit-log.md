# Mission load-bearing wire — Gateway request audit log

**Owner**: claude. **Depends on**: existing `argo-audit::AuditSink` + `argo-audit::Event` + the gateway `boot.rs` request dispatcher (verify path via `find crates -name boot.rs | xargs grep -l 'handle_connection\|gateway'`).
**Authored against**: origin/main `bd7fd450` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-gateway-request-audit-log`.

> SOURCE: MISSION_BACKLOG.md Section 1 #22 ("Add request audit log entry per HTTP request"). **Tier M (compliance).** Today the gateway processes thousands of requests per day with zero audit trail per request — the audit sink only records app-level domain events (skill_proposed, action_dispatched, etc.). For SOC-2 / ISO-27001 attestation we need one audit row per HTTP request, including: timestamp, principal (or `anon`), source IP, method, path, status code, latency, request body hash (NOT body), and a request-id correlation key. Operators get a per-request "who did what when" trail without the cost of full body capture.

## Scope

NEW gateway middleware `crates/argo-cli/src/handlers/gateway/request_audit.rs` (≤ 280 LOC):

```rust
pub struct RequestAuditEntry {
    pub request_id: Uuid,                       // generated at request-receive
    pub received_at: DateTime<Utc>,
    pub principal: Option<String>,              // bearer-resolved or None
    pub source_ip: IpAddr,                      // X-Forwarded-For honored if present
    pub method: String,                         // "GET" / "POST" / etc.
    pub path: String,                           // normalized, query stripped
    pub status_code: u16,                       // filled at response-emit
    pub latency_ms: u32,                        // request-receive → response-emit
    pub request_body_sha256: Option<[u8; 32]>,  // None for GETs and 0-byte POSTs
    pub response_body_len: u32,                 // bytes emitted
    pub user_agent: Option<String>,             // truncated to 200 chars
}

pub async fn record(sink: &AuditSink, entry: RequestAuditEntry) -> ArgoResult<()>;
```

NEW migration `vNN.rs` (verify next-free slot first; as of 2026-05-12 the live tail is v58 / SQL 62 after v57 routing-skillmatch (#1356) and v58 operator-risk-overrides (#1361) shipped — pick v59+ and SQL slot 63+) + raw SQL adding `gateway_request_audit` table:

```sql
CREATE TABLE gateway_request_audit (
    request_id BLOB PRIMARY KEY NOT NULL,
    received_at INTEGER NOT NULL,
    principal TEXT,                  -- NULL for anon
    source_ip TEXT NOT NULL,
    method TEXT NOT NULL,
    path TEXT NOT NULL,
    status_code INTEGER NOT NULL,
    latency_ms INTEGER NOT NULL,
    request_body_sha256 BLOB,        -- NULL for GETs / empty body
    response_body_len INTEGER NOT NULL,
    user_agent TEXT
);
CREATE INDEX idx_gateway_request_audit_received_at ON gateway_request_audit(received_at);
CREATE INDEX idx_gateway_request_audit_principal ON gateway_request_audit(principal) WHERE principal IS NOT NULL;
CREATE INDEX idx_gateway_request_audit_status ON gateway_request_audit(status_code);
```

Wire one ENTER + one EMIT call in `boot.rs::handle_connection`:
- ENTER: stamp `request_id`, `received_at`, parse principal from bearer (re-use `routes_auth::resolve_principal`), capture source-IP (TcpStream::peer_addr + X-Forwarded-For if `config.gateway.behind_proxy = true`), method + path, hash body if POST.
- EMIT: after the response writer flushes, fill `status_code`, `latency_ms`, `response_body_len`. Spawn the `record` call on a `tokio::spawn` so writer-loop isn't blocked on storage.

NEW `argo gateway audit` CLI (`crates/argo-cli/src/handlers/gateway_audit.rs`):
- `argo gateway audit list --since <when> --principal <id> --status <code> --json` (paginated, default last 100)
- `argo gateway audit show <request_id> --json`

NEW `[gateway.request_audit]` config block in `crates/argo-config/src/sections.rs`:
- `enabled: bool = true` (default on; operators opt-out for high-volume read-only deployments)
- `behind_proxy: bool = false` (honor X-Forwarded-For only when true)
- `body_hash_for_methods: Vec<String> = ["POST", "PUT", "PATCH"]` (don't hash GET bodies — they're always empty)
- `user_agent_max_len: u32 = 200`

This is NOT in scope: full body capture (separate brief for verbose-mode); response-body hash (would require buffering the writer); audit row purge / retention (separate ops brief); request-replay from audit log (separate forensics brief); GDPR principal-deletion (separate); integration with the existing app-level audit chain hash (separate — request audit is a SEPARATE table, not appended to the merkle-style chain).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/request_audit.rs` (NEW, ≤ 280 LOC)
- `crates/argo-cli/src/handlers/gateway/request_audit_tests.rs` (NEW, sibling, ≤ 250 LOC) — 7 unit tests
- `crates/argo-cli/src/handlers/gateway/boot.rs` — narrowly scoped: ADD request_audit ENTER + EMIT hooks (≤ 30 LOC delta)
- `crates/argo-cli/src/handlers/gateway/mod.rs` — narrowly scoped: register `pub mod request_audit;`
- `crates/argo-cli/src/handlers/gateway_audit.rs` (NEW, ≤ 180 LOC) — `list` + `show` handlers
- `crates/argo-cli/src/handlers/mod.rs` — narrowly scoped: register handler
- `crates/argo-cli/src/cli_gateway_cmds.rs` — narrowly scoped: ADD `Audit { list / show }` arm to `GatewayCmd`
- `crates/argo-cli/src/handlers/gateway.rs` — narrowly scoped: dispatch new `GatewayCmd::Audit`
- `crates/argo-store/src/gateway_request_audit.rs` (NEW, ≤ 220 LOC) — `record / list / get` primitives + DTO
- `crates/argo-store/src/gateway_request_audit_tests.rs` (NEW, sibling, ≤ 180 LOC) — 5 unit tests
- `crates/argo-store/src/lib.rs` — narrowly scoped: register `pub mod gateway_request_audit;`
- `crates/argo-store/src/migrations/vNN.rs` (NEW, verify next-free slot, ≤ 60 LOC) — typed migration
- `crates/argo-store/migrations/<NN>__gateway_request_audit.sql` (NEW, ≤ 30 LOC) — raw SQL parallel
- `crates/argo-store/src/migrations/mod.rs` — narrowly scoped: register new migration
- `crates/argo-config/src/sections.rs` — narrowly scoped: ADD `[gateway.request_audit]` section
- `crates/argo-config/src/defaults.rs` — narrowly scoped: ADD default values
- `crates/argo-cli/tests/gateway_request_audit_smoke.rs` (NEW, ≤ 150 LOC) — integration smoke: boot gateway, hit `/health` + `/providers`, query `argo gateway audit list`, assert 2 rows present with the right shape

## Files you must NOT touch

- `crates/argo-audit/**` — request-audit is a SEPARATE table, not part of the app-level merkle chain
- Any other crate
- `crates/argo-store/migrations/**` slots ≤ 60 (those are in-flight)

## Acceptance criteria

- [ ] Every HTTP request through the gateway emits exactly one audit row, regardless of route
- [ ] Audit rows include all 11 columns; principal=NULL for anon GETs to `/health`/`/livez`/`/readyz`/`/providers`
- [ ] X-Forwarded-For honored when `[gateway.request_audit].behind_proxy = true`, ignored otherwise
- [ ] Request body sha256 populated for POSTs with non-empty body; NULL for GETs
- [ ] User-Agent truncated to ≤ 200 chars
- [ ] `argo gateway audit list --since "1h ago" --json` returns paginated JSON with the right shape
- [ ] `argo gateway audit show <request_id> --json` returns a single row
- [ ] Audit write is async (`tokio::spawn`) — writer-loop isn't blocked on storage I/O. Verify in a unit test that the response is emitted before the audit row is committed.
- [ ] `[gateway.request_audit].enabled = false` disables all rows AND the audit middleware (zero overhead in disabled mode — verify via a unit test that no record() call happens)
- [ ] 7 unit tests on request_audit + 5 unit tests on store-side primitives + 1 integration smoke
- [ ] `cargo clippy -p argo-store -p argo-config -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-store -p argo-config -p argo-cli` passes
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-gateway-request-audit-log`
- PR title: `gateway: per-HTTP-request audit log (compliance) (mission-gateway-request-audit-log)`

## Notes for the implementer

- Mirror the `mission-per-tenant-rate-limiting` (#1338) pattern: middleware in `boot.rs::handle_connection` AFTER per-IP gate + per-tenant gate, BEFORE route dispatch. The ENTER hook lives early; the EMIT hook lives in the response-write path.
- `request_id` should be a v4 UUID (matches existing UUID convention in argo-types).
- `source_ip` precedence: if `behind_proxy = true` AND `X-Forwarded-For` is present, use the leftmost (originating) IP; else use TcpStream::peer_addr.
- Body hash: sha256 of the raw bytes BEFORE deserialization. For multipart uploads ≥ 1 MiB, hash only the first 1 MiB (avoid streaming-cost on uploads). Document this in the module docs.
- The async-spawn pattern: `tokio::spawn(async move { request_audit::record(&sink, entry).await })` — if record() errors, log at `tracing::warn!` but do NOT bubble back to the response path. The audit row is best-effort; the request response is load-bearing.
- Test the "audit row written after response emitted" invariant by using a slow `MockAuditSink` that blocks for 200ms; the response should still emit in under 50ms.
- Migration slot: as of 2026-05-12 the live tail is v58 / raw slot 62 (v56 per-tenant-rate-limiting + v57 routing-skillmatch + v58 operator-risk-overrides all shipped). Pick v59 / raw slot 63. Verify before claim with `ls crates/argo-store/migrations/ | sort -V | tail -10` and `ls crates/argo-store/src/migrations/v*.rs | sort -V | tail -10` and pick the next-free if numbers have shifted further (mission-audit-chain-watchdog is a parallel brief that may also claim a slot in this range — coordinate via ordering).
- `enabled = false` should be a true zero-overhead path: don't allocate the entry, don't hash the body, don't enter the middleware. The `boot.rs` hook should `if !config.gateway.request_audit.enabled { /* skip */ }` before any work.
- `gateway_request_audit_smoke.rs` should match the established style of `crates/argo-cli/tests/gateway_tenant_rate_limit_smoke.rs` — set `ANTHROPIC_API_KEY=dummy` to bypass startup_check, boot gateway, hit endpoints, query the audit table directly via the store or via the CLI.
