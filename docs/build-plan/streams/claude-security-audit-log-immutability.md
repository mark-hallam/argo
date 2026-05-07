# Mission Phase 2 — Audit log immutability via hash chain

**Owner**: claude. **Depends on**: none (independent — extends existing audit table).

## Scope

Argo already has an `audit` table tracking actor, action, target, timestamp.
Today, a tenant admin (or attacker with DB write) could silently delete or
rewrite audit rows without detection. This brief makes the audit table
*tamper-evident* via a hash chain: each row stores `prev_hash` (sha256 of the
previous row's canonical encoding) and `row_hash` (sha256 of this row's
canonical encoding including `prev_hash`). A new `argo audit verify-chain`
command walks the chain end-to-end and reports the first divergence.

This does NOT prevent tampering — but it makes tampering *detectable*, which
is the SOC2 / GDPR audit-trail expectation.

## Approach (concrete)

1. **NEW** `crates/argo-store/src/audit_chain.rs` (~280 LOC):
   - `pub struct AuditChainRow { id, actor, action, target, ts, prev_hash, row_hash }`
   - `pub fn canonical_encode(row: &AuditChainRow) -> Vec<u8>` — deterministic
     CBOR or sorted-key JSON of `(id, actor, action, target, ts, prev_hash)`
   - `pub fn compute_row_hash(prev_hash: &[u8;32], row: &AuditChainRow) -> [u8;32]`
   - `pub async fn append(pool: &SqlitePool, row: NewAuditRow) -> ArgoResult<AuditChainRow>` —
     reads tail's `row_hash`, computes new row_hash, inserts inside a tx
   - `pub async fn verify_chain(pool: &SqlitePool, tenant: Option<&str>) -> ArgoResult<VerifyOutcome>`
     where `VerifyOutcome = { rows_checked, first_divergence: Option<RowId>, ok: bool }`
2. **Modify** `crates/argo-store/migrations/`:
   - New migration adds `prev_hash BLOB NOT NULL` and `row_hash BLOB NOT NULL`
     columns to `audit` (or whatever the existing audit table is named)
   - Backfill: existing rows get computed in id-order during migration so the
     chain is consistent from row 1
3. **Modify** all existing `audit::insert(...)` (or equivalent) call sites to
   route through `audit_chain::append` — narrowly, just the insert helper.
4. **NEW** CLI surface in `crates/argo-cli/src/handlers/audit_verify.rs`
   (~120 LOC):
   - `argo audit verify-chain [--tenant <id>] [--format json|text]`
   - Returns exit 0 if chain is intact, exit 1 + first-divergent-row-id if not

## Manifest contract / wire format

No wire change. New CLI subcommand only:

```
argo audit verify-chain --format json
{
  "rows_checked": 12345,
  "ok": true,
  "first_divergence": null
}
```

## Files you may touch

- `crates/argo-store/src/audit_chain.rs` (NEW, ≤ 320 LOC)
- `crates/argo-store/src/audit_chain_tests.rs` (NEW, ≤ 250 LOC) — 6 unit tests:
  - empty table → ok, 0 rows
  - single row append → row_hash present, prev_hash is zeroes
  - 100 row append → chain consistent, verify ok
  - tampered row (manual UPDATE) → verify reports divergence at correct id
  - deleted row → verify reports divergence
  - re-ordered rows → verify reports divergence
- `crates/argo-store/src/lib.rs` — narrowly: 1 pub mod line
- `crates/argo-store/migrations/<new>.sql` (NEW)
- `crates/argo-cli/src/handlers/audit_verify.rs` (NEW, ≤ 150 LOC)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 1 pub mod line
- `crates/argo-cli/src/cli.rs` — narrowly: 1 subcommand variant
- `crates/argo-cli/src/main.rs` — narrowly: 1 dispatch arm
- `crates/argo-cli/Cargo.toml` — only if a new dep is needed (sha2 already used)

## Files you must NOT touch

- Existing audit *read* paths — chain is verify-only, reads stay simple
- `argo-types` — `AuditChainRow` lives in `argo-store`

## Acceptance criteria

- [ ] Migration backfills `prev_hash` + `row_hash` for existing rows
- [ ] All audit inserts route through `audit_chain::append`
- [ ] `argo audit verify-chain` exits 0 on intact chain, 1 on tamper
- [ ] Tampered-row test detects divergence at the correct row id
- [ ] 6 unit tests + 1 CLI smoke test
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-store && cargo test -p argo-cli` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-security-audit-log-immutability`
- PR title: `audit: hash-chain audit table for tamper detection (claude-security-audit-log-immutability)`

## Notes for the implementer

- Use `sha2::Sha256` (already a workspace dep).
- Canonical encoding MUST be deterministic — pick CBOR via `ciborium` OR
  sorted-key JSON via `serde_json` with a sorted-map serializer; pick one
  and stick with it across append + verify.
- Chain genesis: `prev_hash = [0u8; 32]` for row 1.
- Migration backfill must hold a write lock and walk in id-order.
