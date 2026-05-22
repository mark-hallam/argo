# Stream H — `argo tenant` management

**Owner**: codex. **Depends on**: E-identity (#781) merged.

## Scope

- `argo tenant list` → tenants known to the workspace
- `argo tenant info <id>` → workers + sessions + recent
  activity for one tenant
- `argo tenant create <name>` → register a new tenant

`--json` everywhere.

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- `crates/argo-cli/src/cli_tenant_cmds.rs` (NEW)
- `crates/argo-cli/src/handlers/tenant.rs` (NEW)
- `crates/argo-cli/tests/tenant.rs` (NEW)

## Acceptance criteria
- [ ] All 3 subcommands wired
- [ ] `info <id>` shows worker count + session count + last
      action timestamp
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean
- [ ] Every authored file < 500 LOC

## Branch + PR
- Branch: `codex-H-tenant-cli`
- PR title: `cli: argo tenant list/info/create (H-tenant-cli)`
