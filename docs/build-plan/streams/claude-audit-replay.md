# Claude — Audit-chain replay for incident investigation

**Owner**: claude. **Depends on**: nothing (read-only on
existing argo-audit data).

## Scope

`argo audit replay <session_id> [--from-seq N] [--until <iso8601>] [--format pretty|raw]`
walks the session's audit chain in seq order:
- `pretty` (default): one-line headlines per row
- `raw`: full JSON (NDJSON for piping)

Useful for incident postmortems + customer support.

## Files you may touch
- `crates/argo-audit/src/replay.rs` (NEW formatter)
- `crates/argo-audit/src/lib.rs` — narrowly: re-export
- BLANKET CLI ALLOWLIST applies (cli_subcommands.rs gets
  AuditCmd::Replay variant + handlers/audit_replay.rs)

## Acceptance criteria
- [ ] Rows printed in seq order
- [ ] `--from-seq` + `--until` filters work
- [ ] `pretty` truncates payloads to 80 chars
- [ ] `raw` outputs valid NDJSON
- [ ] Integration test seeds 5 rows + asserts ordered
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean
- [ ] Every authored file < 500 LOC

## Branch + PR
- Branch: `claude-audit-replay`
- PR title: `audit: argo audit replay for session incident investigation (claude-audit-replay)`
