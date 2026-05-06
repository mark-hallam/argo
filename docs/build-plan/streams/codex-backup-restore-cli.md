# V1.x — Workspace backup + restore CLI

**Owner**: codex. **Depends on**: nothing.

## Scope

Today an operator's workspace state (sessions DB, audit DB,
memory rows, credentials, vertical manifests) lives in
`<workspace>/.argo/` with no backup story. If the directory
gets corrupted or accidentally deleted, the operator loses
everything.

Ship `argo backup` + `argo restore`:

- `argo backup create [--output <path>] [--include-credentials]`
  — creates a tarball at `<path>` (default
  `<workspace>/.argo/backups/<timestamp>.tar.gz`); skips
  credentials by default (security default; `--include-credentials`
  opt-in)
- `argo backup list` — list available backups
- `argo backup verify <path>` — sha256 + structural check
- `argo restore <path> [--into <workspace>] [--dry-run]` —
  unpack into workspace; refuses non-empty workspace unless
  `--into` specifies a fresh dir; `--dry-run` lists what
  would be restored

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- `crates/argo-cli/src/handlers/backup_*.rs` (4 NEW —
  create/list/verify/restore)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 4 pub mods
- `crates/argo-cli/src/cli_backup_cmds.rs` (NEW)
- `crates/argo-cli/src/cli_subcommands.rs` — narrowly: register
  `Backup` subcommand
- `crates/argo-cli/tests/backup_smoke.rs` (NEW)

## Acceptance criteria
- [ ] `backup create` produces a valid tar.gz of `.argo/`
- [ ] credentials.toml excluded by default; included with
      `--include-credentials`
- [ ] sha256 of tarball recorded in a `manifest.json` inside
- [ ] `restore` into a fresh dir works
- [ ] `restore` into a non-empty workspace refused (no
      data clobbering)
- [ ] `--dry-run` lists files without extracting
- [ ] `--json` envelope universal
- [ ] 6 unit tests + 1 CLI smoke (full round-trip)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean

## Branch + PR
- Branch: `codex-backup-restore-cli`
- PR title: `cli: argo backup + restore (codex-backup-restore-cli)`
