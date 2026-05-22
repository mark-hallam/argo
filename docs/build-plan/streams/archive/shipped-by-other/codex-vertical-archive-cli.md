# V1.x — Vertical archive CLI

**Owner**: codex. **Depends on**: H-vertical-list-cli (#834)
merged. Symmetric to codex-tenant-cli-extensions.

## Scope

Operators sometimes ship a vertical, then want to retire it
without losing audit history (e.g. retired SDR vertical
replaced by a new one). Add archive/unarchive/delete:

- `argo vertical archive <id>` — soft-delete, vertical
  excluded from `argo vertical list` defaults; existing
  workers bound to it can still finish their work but no
  new sessions can bind
- `argo vertical unarchive <id>` — re-activate
- `argo vertical delete <id> [--force]` — hard delete (only
  allowed for archived verticals; preserves audit chain
  via redaction-not-deletion in audit payloads)

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- `crates/argo-cli/src/handlers/vertical_archive.rs` (NEW)
- `crates/argo-cli/src/handlers/vertical_unarchive.rs` (NEW)
- `crates/argo-cli/src/handlers/vertical_delete.rs` (NEW)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 3 pub mods
- `crates/argo-cli/src/cli_vertical_cmds.rs` — narrowly: 3
  new subcommand variants
- `crates/argo-store/migrations/<next>__vertical_archived.sql`
  (NEW — adds archived_at col)
- `crates/argo-cli/tests/vertical_archive_smoke.rs` (NEW)

## Acceptance criteria
- [ ] All 3 commands work end-to-end
- [ ] Archived verticals excluded from `argo vertical list`
      unless `--include-archived`
- [ ] `delete --force` rejected for non-archived verticals
- [ ] Bound-worker grace period honored (no immediate kill)
- [ ] `--json` envelope (universal convention)
- [ ] 6 unit tests + 1 CLI smoke test
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean

## Branch + PR
- Branch: `codex-vertical-archive-cli`
- PR title: `cli: argo vertical archive/unarchive/delete (codex-vertical-archive-cli)`
