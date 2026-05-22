# web-audit-feed-platform-events

**Owner**: claude.
**Depends on**: none.
**Authored against**: `origin/main` `d2be56888` on `2026-05-22`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-audit-feed-platform-events`.

🟡 **P1 OPERATOR-TRUST GAP.** Source: evaluator bootstrap 2026-05-22T02:25 + auditor T03-39 §4 P1. `GET /api/v1/audit` is scoped to v67-era action-record dimensions and never surfaces `WorkerHired` / `WorkflowHeadChanged` / `WorkflowRolledBack` events, even though the handlers DO write those rows into `audit_record`. The audit feed visible to operators is missing the platform-level events that prove the swarm is doing what it says it's doing. **Closes mission claim 1 audit-feed gap** (per auditor coverage table).

## Scope

Extend `GET /api/v1/audit` so the response set UNIONs platform-event rows (`WorkerHired`, `WorkflowHeadChanged`, `WorkflowRolledBack`, and any other variants currently written to `audit_record` but absent from the feed query) into the existing action-record results, ordered by occurred-at descending.

Discover the full list of audit-record variants by:
1. Grep the workspace for `audit_record::insert*` / `AuditSink::record` / similar — every write call defines a variant that should be feedable.
2. Compare against the SELECT-side query in the gateway handler — anything written but not read is the gap.
3. List them all in the PR body before deciding scope. If the gap is >6 variants, split into a follow-up brief; otherwise land all in this PR.

The handler is in `crates/argo-gateway` (find it via `rg 'fn.*audit' crates/argo-gateway/src/` — likely `routes_audit.rs`). The Next.js client at `web/lib/api/` already calls the endpoint; only the server response shape extends. UI rendering for the new event types is **out of scope** for this brief (the audit page already handles unknown kinds gracefully — confirm before claiming and note any UI gap as a follow-up).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `crates/argo-gateway/src/routes_audit.rs` (or equivalent handler path — verify before claiming) — extend the SELECT/UNION.
- `crates/argo-gateway/src/routes_audit_tests.rs` (or equivalent) — add integration tests asserting platform-event variants appear in the feed.
- `crates/argo-store/src/audit_record.rs` (or equivalent SELECT-side query) — narrowly scoped: extend the variant filter.
- `crates/argo-types/src/audit.rs` — narrowly scoped: add any missing serde-tag variants needed by the feed response.
- `web/lib/api/audit.ts` (or equivalent client wrapper) — narrowly scoped: extend the response type if a new variant tag appears.
- `web/__tests__/audit-feed-platform-events.test.ts` (or co-located) — frontend test asserting client deserialization.

## Files you must NOT touch

- The audit-page UI components — rendering is out of scope (audit page handles unknown kinds gracefully today).
- The `audit_record` write path — this brief is read-side only.
- The `__Host-session` JWT shape or any auth layer.
- BUILD_STATUS.md outside the standard row flip.

## Acceptance criteria

- [ ] PR body lists every `audit_record` variant currently written but missing from the feed (table: variant | written-from-file | reason-it-was-missed).
- [ ] `GET /api/v1/audit` returns rows for at least `WorkerHired`, `WorkflowHeadChanged`, `WorkflowRolledBack`. Other variants UNIONed at worker discretion within scope.
- [ ] Integration test: hire a worker via `POST /api/v1/workers/hire`, then call `GET /api/v1/audit`, and the response includes a `WorkerHired`-tagged row with the new worker's id.
- [ ] Integration test: rollback a workflow, then call `GET /api/v1/audit`, and the response includes a `WorkflowRolledBack`-tagged row.
- [ ] Frontend test: client deserializer parses all new variants without throwing.
- [ ] Tenant isolation invariant holds: a worker hired in tenant A does NOT surface in tenant B's audit feed.
- [ ] `cargo clippy -p argo-gateway --all-targets -- -D warnings` clean.
- [ ] `cargo test -p argo-gateway routes_audit` passes (new tests included).
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean (final sanity).
- [ ] `pnpm typecheck` clean.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-web-audit-feed-platform-events`
- PR title: `gateway(audit): surface platform-event variants in /api/v1/audit (web-audit-feed-platform-events)`

## Notes for the implementer

- ~200 LOC expected. Most LOC is in tests; the SQL/UNION extension is ~30 LOC.
- The cross-tenant invariant test is **load-bearing**. Last week's `#1983` PR shipped a real cross-tenant data-exposure bug in audit detail; the conductor expects every audit-feed touch to include a fresh negative test (worker A in tenant T1 does NOT surface in tenant T2 feed).
- If the gateway query lives behind a SQL view or stored helper, follow the existing pattern — do not introduce a new DSL or query-builder abstraction for this brief.
- `WorkflowHeadChanged` may not exist as a variant yet — check `audit_record` shape first. If the variant is missing, the brief covers only what's actually written (`WorkerHired` + `WorkflowRolledBack` at minimum). Note any "expected variant doesn't exist yet" as a follow-up brief, not in scope here.
