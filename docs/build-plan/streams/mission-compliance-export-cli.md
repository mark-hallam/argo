# mission-compliance-export-cli — argo compliance export with date/tenant/format filters

**Owner**: codex. **Depends on**: nothing.
**Authored against**: origin/main `31d8e20e` on `2026-05-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-compliance-export-cli`.

## Scope

v1.1 production hardening. Extend `argo compliance` subcommand with `export` action: `--format json|csv|pdf`, `--start-date`, `--end-date`, `--tenant-id`, `--session-id`. Audit-4-grade: immutable, tamper-evident output with signature + checksum.

LOC: ~600.

## Files you may touch

- `crates/argo-compliance/src/export.rs` (NEW) — export query engine + format writers. ≤350 LOC.
- `crates/argo-cli/src/handlers/compliance.rs` — narrowly: wire export subcommand. ≤50 modified lines.
- `crates/argo-cli/src/cli_compliance_cmds.rs` — `Export` variant. ≤30 modified lines.
- Test sibling — round-trip + filter + checksum tests. ≤180 LOC.

## Files you must NOT touch

- Anything outside the may-touch list above.
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] Implementation matches scope above.
- [ ] All new tests pass.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean (Rust-side).
- [ ] No file > 500 LOC.
- [ ] BUILD_STATUS.md row updated with PR number.

## Branch + PR

- Branch: `codex-mission-compliance-export-cli`
- PR title: `feat: argo compliance export with date/tenant/format filters (mission-compliance-export-cli)`

## Notes for the implementer

- Priority: v1.1 governance.
- Authored as part of post-v1 polish backlog batch 2026-05-15.
