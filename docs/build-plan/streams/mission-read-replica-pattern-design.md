# Mission load-bearing wire — Read-replica pattern for heavy audit queries (design)

**Owner**: claude. Design doc only.
**Authored against**: origin/main `1cca6a69` on `2026-05-12`.

> SOURCE: MISSION_BACKLOG.md Section 8 #240. Audit dashboards on large tenants run heavy queries that contend with write traffic. Read-replicas solve it but require careful design — stale-replica reads, replication lag, query routing.

## Scope

NEW `docs/skycot/architecture/read-replica-design.md` (≤ 500 LOC) covering:

1. **Why** — audit-heavy queries (E5 audit-timeline, J14 audit-feed, J15 audit-detail) lock writers on SQLite write-lock contention
2. **Read patterns** — categorize current read queries: real-time (recent N rows) vs analytical (historical aggregates) vs interactive (operator-driven E5 drill-in)
3. **Replication options**:
   - SQLite: WAL-mode + `litestream` or `cr-sqlite` for replicas
   - PostgreSQL (after `mission-postgres-support-path-plan`): native streaming replication
4. **Read-routing pattern** — `ReadPool` distinct from `WritePool`. Stale-tolerance per query: real-time = primary, analytical = any-replica, interactive = primary-with-lag-headers.
5. **Replication-lag awareness** — `X-Replica-Lag-Seconds` header on responses; client-side staleness handling
6. **Migration sequence** — 5 follow-up briefs that implement the design

No code in this brief.

## Files you may touch

- `docs/skycot/architecture/read-replica-design.md` (NEW, ≤ 500 LOC)
- `docs/skycot/RUNBOOK.md` — 1-line pointer

## Files you must NOT touch

- Any code

## Acceptance criteria

- [ ] Design doc covers 6 sections above
- [ ] Stale-tolerance categorization for ≥ 5 specific query paths (cite file:line)
- [ ] 5 follow-up briefs named with one-line scope each
- [ ] BUILD_STATUS.md updated

## Branch + PR

- Branch: `claude-mission-read-replica-pattern-design`
- PR title: `architecture: read-replica pattern design (mission-read-replica-pattern-design)`

## Notes

- litestream is the easy SQLite replication path (stream WAL to S3); cr-sqlite is heavier but supports multi-writer. Pick one per use case.
- The X-Replica-Lag-Seconds header is the operator-visible safety belt — clients that need fresh data can opt out of replicas explicitly.
- This brief is the BLUEPRINT. Implementation follows once the path is agreed upon.
