# Mission Phase 2 — GDPR data delete (right-to-erasure)

**Owner**: claude. **Depends on**: claude-compliance-gdpr-data-export (shares
the `TENANT_TABLES` discovery list — extract to a shared module).

## Scope

GDPR Article 17 grants data subjects the right to erasure of their personal
data. This brief adds:

```
argo compliance delete --tenant <id> --confirm "<id>" [--dry-run]
```

that cascades a delete across every tenant-scoped table for the supplied
`tenant_id`. The `--confirm <id>` arg MUST exactly match `--tenant`; mismatch
is a hard error (no foot-gun). `--dry-run` reports row counts that *would* be
deleted without performing any DELETE.

The audit row for the delete is itself preserved (audit is append-only and
hash-chained per #audit-log-immutability). Audit rows for the deleted tenant
get a special `tombstone` representation: actor/target stay, but other PII
fields are scrubbed to `<erased>` so the chain stays valid while honouring
erasure.

## Approach (concrete)

1. **Refactor** the `TENANT_TABLES` static from `compliance_export.rs` into
   `crates/argo-cli/src/handlers/compliance_tenant_tables.rs` (new shared
   module, ~60 LOC) so both export + delete reference the same list.
2. **NEW** `crates/argo-cli/src/handlers/compliance_delete.rs` (~280 LOC):
   - `pub async fn run(args: ComplianceDeleteArgs, store: &Store) -> ArgoResult<i32>`
   - Validates `args.confirm == args.tenant` — else exit 2 with clear error
   - Begins a transaction
   - `--dry-run`: SELECT COUNT(*) per table, prints summary, ROLLBACK
   - Real run: in delete-order (children first via FK), executes
     `DELETE FROM <t> WHERE <col> = ?`. The `audit` table gets special
     handling: instead of DELETE, UPDATE-redacts PII columns to `<erased>`
     while preserving id + ts + hash columns
   - Appends a final audit row
     `actor=<caller>, action=compliance.delete, target=<tenant_id>` BEFORE
     COMMIT
   - COMMIT
3. **Modify** `cli.rs` + `main.rs`:
   - New `Compliance::Delete` sub-subcommand (mirror Export)

## Manifest contract / wire format

CLI surface only. Output (text or `--format json`):

```
argo compliance delete --tenant t-42 --confirm t-42 --dry-run
audit:        12 rows (will be tombstoned)
messages:    340 rows
sessions:     22 rows
forge_runs:   18 rows
tenant_quota:  1 row
TOTAL:       393 rows (dry run — no changes)
```

## Files you may touch

- `crates/argo-cli/src/handlers/compliance_delete.rs` (NEW, ≤ 320 LOC)
- `crates/argo-cli/src/handlers/compliance_delete_tests.rs` (NEW, ≤ 250 LOC) —
  7 unit tests:
  - confirm mismatch → exit 2, no DELETEs
  - dry-run → counts correct, no DELETEs (verify rows still present)
  - real delete on single-row tenant → all child tables empty for that tenant
  - cross-tenant isolation (other tenant's rows untouched)
  - audit table: rows tombstoned not deleted, hash chain still verifies
  - audit row appended for the delete itself
  - tx rollback on mid-delete failure (inject failure, assert all-or-nothing)
- `crates/argo-cli/src/handlers/compliance_tenant_tables.rs` (NEW, ≤ 60 LOC) —
  shared `TENANT_TABLES` and `delete_order`
- `crates/argo-cli/src/handlers/compliance_export.rs` — narrowly: import the
  shared module instead of inlining the list (depends on the export brief
  landing first; if not yet, this brief inlines its own copy and the export
  brief refactors to share)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 2 pub mod lines
- `crates/argo-cli/src/cli.rs` — narrowly: `Compliance::Delete` subcommand
- `crates/argo-cli/src/main.rs` — narrowly: 1 dispatch arm
- `crates/argo-cli/tests/compliance_delete_smoke.rs` (NEW, ≤ 200 LOC) —
  1 integration test that seeds 2 tenants, deletes one, asserts isolation +
  audit-tombstone shape

## Files you must NOT touch

- Schema files / migrations — delete uses existing schema
- `argo-store::audit_chain` — delete uses tombstone UPDATE which preserves
  hash columns; chain verify still passes

## Acceptance criteria

- [ ] `--confirm` mismatch → exit 2, no rows touched
- [ ] `--dry-run` reports counts without modifying data
- [ ] Real run cascades across all tenant tables in a single tx
- [ ] Audit table rows tombstoned (PII columns → `<erased>`, hash chain valid)
- [ ] Audit row appended for the delete operation itself
- [ ] Cross-tenant isolation preserved
- [ ] 7 unit tests + 1 integration smoke
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-cli` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-compliance-gdpr-data-delete`
- PR title: `compliance: GDPR data delete with cascade + audit tombstone (claude-compliance-gdpr-data-delete)`

## Notes for the implementer

- The whole operation MUST be a single SQL transaction. If anything fails
  mid-cascade, rollback — partial deletes are not GDPR-compliant.
- Order DELETEs child-first to avoid FK violations. Verify FK relationships
  in migrations before settling on the order.
- Audit tombstone preserves the chain: `prev_hash`/`row_hash`/`id`/`ts` stay,
  PII columns flip to `<erased>`. The chain still verifies because the
  canonical encoding includes the new (erased) values, and we are recording
  a forward-tombstone, not rewriting history. Document this.
