# Mission Phase 2 — GDPR data export (right-to-access)

**Owner**: claude. **Depends on**: none (operates on existing tenant tables).

## Scope

GDPR Article 15 grants data subjects the right to access all personal data a
controller holds on them. Argo currently has tenant_quota, audit, messages,
sessions, forge runs scoped by `tenant_id`. This brief adds a new CLI:

```
argo compliance export --tenant <id> --format gdpr [--out <path>]
```

that emits a single JSON document (or zipped bundle if `--out` ends in `.zip`)
containing every row across every table that has the supplied `tenant_id`. The
`--format gdpr` flag selects the GDPR-aligned shape: a top-level
`{"subject": "<tenant_id>", "exported_at": "<rfc3339>", "tables": {...}}`
where each table is `{"schema": [...], "rows": [...]}`.

This is read-only and idempotent. It is the export half of the GDPR pair;
`compliance-gdpr-data-delete` is the erasure half.

## Approach (concrete)

1. **NEW** `crates/argo-cli/src/handlers/compliance_export.rs` (~280 LOC):
   - `pub async fn run(args: ComplianceExportArgs, store: &Store) -> ArgoResult<i32>`
   - Discovers tenant-scoped tables via a static `TENANT_TABLES: &[(&str,
     &str)]` mapping `(table_name, tenant_column)` — initial set:
     `audit`, `messages`, `sessions`, `forge_runs`, `tenant_quota`,
     `anomaly_events` (whatever exists; verify against migrations at impl time)
   - For each table: `SELECT * FROM <t> WHERE <col> = ?` streamed row-by-row
     into the output writer (avoid loading entire tables in memory)
   - Output shape:
     ```json
     {
       "subject": "<tenant_id>",
       "exported_at": "<rfc3339>",
       "argo_version": "<env CARGO_PKG_VERSION>",
       "tables": {
         "audit": { "schema": ["id","actor",...], "rows": [...] },
         "messages": { ... }
       }
     }
     ```
   - `--out` writes to file; absent → stdout
   - `--out *.zip` → writes a zip with one JSON entry per table (for large
     tenants); requires `zip` crate (workspace dep, verify before adding)
2. **Modify** `cli.rs` + `main.rs`:
   - New `Compliance` subcommand with `Export` sub-subcommand (mirror existing
     2-level subcommand patterns in the codebase)
3. **Audit hook**:
   - On successful export, append an audit row
     `actor=<caller>, action=compliance.export, target=<tenant_id>` so the
     export itself is logged

## Manifest contract / wire format

CLI surface only. JSON output is documented in the brief above; consumers
should treat additional table keys as forward-compatible (the export shape is
intentionally append-only).

## Files you may touch

- `crates/argo-cli/src/handlers/compliance_export.rs` (NEW, ≤ 320 LOC)
- `crates/argo-cli/src/handlers/compliance_export_tests.rs` (NEW, ≤ 250 LOC) —
  6 unit tests:
  - empty tenant → valid shell with empty rows arrays
  - single-row tenant → row appears in correct table
  - tenant isolation (other tenant's row is NOT in export)
  - `--out file.json` writes valid JSON
  - audit row appended on success
  - missing tenant id → exit non-zero with clear error
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 1 pub mod line
- `crates/argo-cli/src/cli.rs` — narrowly: `Compliance::Export` subcommand
- `crates/argo-cli/src/main.rs` — narrowly: 1 dispatch arm
- `crates/argo-cli/tests/compliance_export_smoke.rs` (NEW, ≤ 200 LOC) —
  1 integration test that seeds 2 tenants and asserts isolation

## Files you must NOT touch

- Schema files / migrations — export is read-only
- Other tenant-scoped tables' code — only the export reads them

## Acceptance criteria

- [ ] `argo compliance export --tenant <id> --format gdpr` emits valid JSON
- [ ] Cross-tenant isolation enforced (other tenants' rows absent)
- [ ] `--out <file>` writes correctly; stdout is the default
- [ ] Audit row appended on successful export
- [ ] `argo compliance export --help` shows the surface
- [ ] 6 unit tests + 1 integration smoke
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-cli` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-compliance-gdpr-data-export`
- PR title: `compliance: GDPR data export (claude-compliance-gdpr-data-export)`

## Notes for the implementer

- Discover the actual tenant-scoped tables from migrations before hardcoding
  `TENANT_TABLES`. If a table has tenant scoping via FK rather than a direct
  column, document and JOIN through the parent.
- Stream row-by-row using `sqlx::query(...)::fetch(...)` to avoid loading the
  entire table for large tenants.
- The export is the user's data — preserve raw values; do NOT redact PII here
  (that's the responsibility of the upstream PII layer at write time).
