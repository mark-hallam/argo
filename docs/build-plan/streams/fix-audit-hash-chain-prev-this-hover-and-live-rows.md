# fix-audit-hash-chain-prev-this-hover-and-live-rows

**Owner**: claude.
**Authored against**: origin/main `0653c478d` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-audit-hash-chain-prev-this-hover-and-live-rows`.
**Source**: e2e-ux-audit Agent 4 findings F7 + F8 (`docs/skycot/ux-audits/2026-05-25-1100-scene-6-claim-6-trust-and-approvals.md`).

🟡 **P1 — Scene 6 hash-chain "centerpiece" affordance does not exist; live rows have no hash field. SOC-2-conscious viewer perception risk.**

## Root cause

Two coupled defects:

1. **No prev-hash chain edge anywhere.** `web/components/skycot/audit/ActivityRow.tsx:166-195` `HashCell` shows only the row's own hash on hover (`title="${hash} — Click to copy"`). There is no `prevHash` field on `AuditRow` (`web/lib/types/audit.ts`) and nothing renders a chain edge. The Scene 6 brief calls the hash-chain hover "Scene 6's centerpiece" — that affordance is missing.

2. **The seeded hash isn't actually a chain.** `web/lib/sample-audit.ts:149` computes each row's hash as `rowHash(worker.id|verb|timestamp)` — eight FNV-1a passes on per-row content. Rows do NOT depend on previous rows. The file comment at `:159` even acknowledges: "matches the visual length of a SHA-256 digest so the demo MP4 reads as forensic-grade". This is the canonical dressing-pretending-to-be-function example.

3. **Live audit rows have no `hash` field at all.** `HashCell` only renders when `activity.hash` is truthy; live `/api/v1/audit` rows include NO `hash` field (Agent-4 curl-verified). The Etherscan-style aesthetic the audit page promises is fixture-only — on real data, the entire hash column disappears.

Per the function-vs-dressing rule: if the demo camera lingers on the hash column AND narrates "every row is hash-chained to the previous", this is function-side theatre and P0. The audit page's Etherscan-style register IS positioned as forensic evidence.

## Findings this brief closes

- Agent 4 F7 [P1] — Hash-chain "prev → this" hover affordance does not exist (centerpiece beat missing)
- Agent 4 F8 [P1] — Live audit rows from the gateway have no `hash` field at all

## Fix recipe

This is a coordinated SPA + gateway fix:

**Gateway-side (real hash chain)**:
1. Add a real `hash` + `prev_hash` field to every audit row. Compute as `sha256(prev_hash || row_canonical_json)` at insert time (or at read time if the audit table doesn't store it yet).
2. Pin the genesis row's `prev_hash` to a known sentinel (e.g. 64 zeros).
3. Surface both in `GET /api/v1/audit` and `GET /api/v1/audit/{id}`.
4. Add a `/api/v1/audit/{id}/verify-chain` endpoint that re-walks and returns `{ ok: true, broken_at: null }` or the breakage point. (Optional — could land in a follow-on brief.)

**SPA-side (render the chain)**:
1. Add `prev_hash: string` to `AuditRow` type in `web/lib/types/audit.ts`.
2. Update `HashCell` (`web/components/skycot/audit/ActivityRow.tsx:166-195`) to show a "prev → this" hover affordance: on hover, a small tooltip renders `{prevHash.slice(0,8)}…  →  {hash.slice(0,8)}…` with a click-to-verify button (calls the verify endpoint, if it exists).
3. Update `web/lib/sample-audit.ts:149` to compute a REAL chain (each row's hash includes the prior row's hash in the FNV input) — even fixture mode should chain honestly. Use the same hash format as the gateway (sha256 — drop FNV).
4. When `activity.hash` is missing, render an explicit "Pending hash" pill rather than silently omitting the column.

## Acceptance criteria

- [ ] Live `GET /api/v1/audit` returns `hash` + `prev_hash` on every row.
- [ ] Live `GET /api/v1/audit/{id}` returns `hash` + `prev_hash`.
- [ ] curl-test: `prev_hash` of row N+1 equals `hash` of row N.
- [ ] SPA `HashCell` shows a "prev → this" hover affordance on real data.
- [ ] `sample-audit.ts` builds a real chain (rows depend on previous rows' hash).
- [ ] Component + integration test pinning the chain integrity.
- [ ] Removed the "matches the visual length of a SHA-256 digest so the demo MP4 reads as forensic-grade" comment — the comment is the smoking gun for the dressing-side bug.

## May-touch

- `crates/argo-cli/src/handlers/gateway/routes_audit*.rs`
- `crates/argo-store/src/audit*.rs` (schema additive — if `prev_hash` column doesn't exist)
- `crates/argo-store/src/migrations/<next>.sql` (additive migration per `docs/skycot/operator/migrations.md`)
- `web/lib/types/audit.ts`
- `web/components/skycot/audit/ActivityRow.tsx` (HashCell)
- `web/lib/sample-audit.ts` (real chain)
- Tests for both layers.

## Out of scope

- Audit list/detail endpoint ID mismatch (separate brief: `fix-audit-list-detail-endpoint-id-mismatch`, already authored)
- Audit row "unknown" worker fixture (separate brief: `fix-audit-unknown-worker-and-hipaa-fixture-rows`)
- Redaction banner narration (P3, not load-bearing)

## Effort

~150-300 LOC across crates + web + migration. ~2-3 hours.
