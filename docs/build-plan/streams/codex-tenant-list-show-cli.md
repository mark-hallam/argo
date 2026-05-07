# V1.4 — Tenant inspection CLI

**Owner**: codex. **Depends on**: nothing (tenant primitives
already exist via E-identity #781 + tenant-archived migration).

## Scope

Today operators can archive tenants (#877) but can't easily
inspect what tenants exist or their metadata (worker count,
spend totals, archive state). Adds:

- `argo tenant list [--include-archived] [--json]` — chronological
  newest-first; columns: id, created_at, archived_at, worker_count,
  spend_today, spend_this_month
- `argo tenant show <id> [--json]` — single-tenant detail:
  metadata + roster summary + recent activity (5 most-recent
  audit rows) + per-channel manifest presence + classification
  rules in effect
- `argo tenant rename <id> <new-id> [--json]` — atomic id rename
  (cascades through worker_identity, spend_ledger, audit, etc.
  via existing FK constraints)

Mirrors the existing `argo worker list/show` (#837) +
`argo vertical list/show/describe` (#834) patterns.

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- `crates/argo-cli/src/handlers/tenant_list.rs` (NEW)
- `crates/argo-cli/src/handlers/tenant_show.rs` (NEW)
- `crates/argo-cli/src/handlers/tenant_rename.rs` (NEW)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 3 pub mods
- `crates/argo-cli/src/cli_tenant_cmds.rs` — narrowly: add
  `List`, `Show`, `Rename` variants if `Tenant` enum lives
  here; otherwise create sibling
- `crates/argo-store/src/tenant_query.rs` (NEW) — pure SQL
  helpers (list_tenants_with_stats, show_tenant, rename_tenant)
- `crates/argo-store/src/lib.rs` — narrowly: re-export
- `crates/argo-cli/tests/tenant_inspection_smoke.rs` (NEW)

## Acceptance criteria
- [ ] `argo tenant list` returns active + (with flag) archived
- [ ] `argo tenant show t-1` returns tenant detail JSON envelope
- [ ] `argo tenant rename t-1 t-1-renamed` cascades through all
      tables (verified via integration test that audit/worker/spend
      rows update transactionally)
- [ ] `--json` envelope universal across all 3 subcommands
- [ ] 5 unit tests + 1 integration test
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean

## Branch + PR
- Branch: `codex-tenant-list-show-cli`
- PR title: `tenant: argo tenant list/show/rename — operator inspection (codex-tenant-list-show-cli)`
