# V1.x — Tenant CLI extensions

**Owner**: codex. **Depends on**: H-tenant-cli (#839) merged.

## Scope

Extend `argo tenant` with three new operations operators
need for real-world tenant lifecycle:

- `argo tenant rename <id> --to <new-name>` — change a
  tenant's display name (id stays stable for FK integrity)
- `argo tenant archive <id>` — soft-delete: tenant survives
  for audit but is excluded from `argo tenant list` defaults
  + workers can't claim against archived tenants
- `argo tenant transfer <id> --owner <new-operator-email>`
  — change operator-of-record (audit-logged)

All three commands are administrative and approval-gated by
the workspace's policy.

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- `crates/argo-cli/src/handlers/tenant_rename.rs` (NEW)
- `crates/argo-cli/src/handlers/tenant_archive.rs` (NEW)
- `crates/argo-cli/src/handlers/tenant_transfer.rs` (NEW)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 3 pub
  mods
- `crates/argo-cli/src/cli_tenant_cmds.rs` — narrowly: 3
  new subcommand variants
- `crates/argo-store/migrations/<next>__tenant_archived.sql`
  (NEW — adds `archived_at`, `owner_email` cols if absent)
- `crates/argo-cli/tests/tenant_extensions_smoke.rs` (NEW)

## Acceptance criteria
- [ ] All 3 commands work end-to-end
- [ ] `archived` tenants excluded from `argo tenant list`
      unless `--include-archived` passed
- [ ] Transfer writes an audit row with old + new owner
- [ ] `--json` envelope (universal convention)
- [ ] 6 unit tests + 1 CLI smoke test
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean

## Branch + PR
- Branch: `codex-tenant-cli-extensions`
- PR title: `cli: argo tenant rename/archive/transfer (codex-tenant-cli-extensions)`
