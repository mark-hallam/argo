# web-gateway-endpoints-audit-mutations-batch — implement 4 audit-mutation endpoints

**Owner**: claude (or codex).
**Authored against**: origin/main `50e65e9f` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-gateway-endpoints-audit-mutations-batch`.
**Priority**: P1 — wave-3 API-wiring audit found 4 audit-mutation endpoints unimplemented. Most critically: export silently fakes evidence bundles (data-integrity risk for procurement).
**Sizing**: ~600-1000 LOC (4 handlers + sandboxed-replay runner + export packaging + tests).

## Why this brief exists

From the wave-3 API-wiring audit:

> 4 audit-mutation routes: `POST /api/v1/audit/<id>/replay`,
> `POST /audit/export`, `POST /audit/export/preview`, `POST /audit`
> (audit-the-audit).
>
> **Audit export endpoints (P1 — silently fakes evidence bundles,
> data-integrity risk).**
> **Audit replay endpoint (P1 — deterministic mock verdict).**

For procurement-grade trust, fake exports are categorically worse
than missing exports — they imply the chain works when it doesn't.

## Endpoints to implement

1. `POST /api/v1/audit/<id>/replay` — sandboxed replay of an audit row. Re-runs the action against a frozen environment, returns a verdict object (same/differ/error + diff details).
2. `POST /api/v1/audit/export` — generate an evidence bundle for an audit range. Body `{ from, to, format: "json" | "csv" | "tar" }`. Returns a signed download URL.
3. `POST /api/v1/audit/export/preview` — count + sample of what would be exported (no actual file generated). Body same; returns `{ row_count, byte_estimate, sample: [...] }`.
4. `POST /api/v1/audit` — record an out-of-band audit entry (the "audit the audit" — e.g. operator inspected the log on date X). Body `{ kind, body, related_action_id }`.

## Steps

1. For replay: wire into the existing argo-replay crate (if present) or document why it must be re-implemented here.
2. For export: design the signed-URL flow — short-lived (5 min) HMAC-signed URL pointing to a server-generated file in a temp directory.
3. For audit-the-audit: just an insert into the audit chain with a distinguishing `kind`.
4. Add gateway handlers, route-dispatch wiring, tests.

## may-touch

- New: `crates/argo-cli/src/handlers/gateway/routes_audit_replay.rs`
- New: `crates/argo-cli/src/handlers/gateway/routes_audit_export.rs`
- New: `crates/argo-cli/src/handlers/gateway/routes_audit_record.rs`
- `crates/argo-replay/` (only if needed; preferred is reuse)
- `crates/argo-audit/` (export packaging)
- `crates/argo-cli/src/handlers/gateway/routes.rs`
- Test files
- `web/lib/api/audit-export.ts` + `replay.ts` (only if envelope changes)

## must-NOT-touch

- `crates/argo-audit/src/` hash-chain logic (immutable invariant)
- Other endpoint clusters

## Acceptance criteria

- [ ] All 4 endpoints return 200 with realistic data when called against a populated audit log.
- [ ] Replay: reruns deterministically — same input → same verdict; differs are described.
- [ ] Export: produces a real signed download with a valid (non-zero) audit-row payload; HMAC sig verifies.
- [ ] Export preview: returns accurate row count + byte estimate before file generation.
- [ ] Audit-the-audit row inserted into the chain, visible in /audit feed.
- [ ] `cargo clippy --workspace -- -D warnings` clean.
- [ ] At least 12 unit tests + 2 integration smokes.

## References

- Audit: `docs/skycot/audit/api-wiring/2026-05-20-wave-3-api-wiring.md`
- Existing: `crates/argo-audit/`, `crates/argo-replay/`
