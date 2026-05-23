# claude-workspace-cargo-dedupe-slack-entry

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `<latest>` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-workspace-cargo-dedupe-slack-entry`.

🟢 **P3 CARGO HYGIENE — slack listed twice in workspace.** Source: evaluator round-10 SHARED.md handoff 2026-05-23T10:55:00Z.

> "claude-workspace-cargo-dedupe-slack-entry (P3 — slack listed twice in workspace)"

The workspace `Cargo.toml`'s `[workspace.members]` array has `crates/argo-mcp-slack` listed twice. Cargo tolerates it (no error) but it's noise; resolver pays a tiny lookup cost on every workspace operation; future maintainers get confused.

## Scope

1. Open `Cargo.toml` at repo root.
2. Find `[workspace.members]` array (or sometimes `members = [...]`).
3. `rg 'argo-mcp-slack'` → confirm the dup. Remove the redundant line.
4. `cargo metadata --format-version 1 --no-deps` → verify resolution is unchanged (same crate, same package_id).
5. `bash scripts/workspace-clippy-fast-fail.sh` → confirm nothing broke.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `Cargo.toml` — single-line removal.
- `docs/build-plan/BUILD_STATUS.md` — standard row flip.

## Files you must NOT touch

- Anything else.

## Acceptance criteria

- [ ] One line removed from `Cargo.toml [workspace.members]`.
- [ ] `cargo metadata --no-deps` succeeds.
- [ ] `bash scripts/workspace-clippy-fast-fail.sh` clean.
- [ ] No other diff.
- [ ] BUILD_STATUS.md updated.

## Branch + PR

- Branch: `claude-workspace-cargo-dedupe-slack-entry`
- PR title: `chore(workspace): dedupe argo-mcp-slack entry in Cargo.toml (claude-workspace-cargo-dedupe-slack-entry)`

## Notes for the implementer

- ~1-3 LOC change. Fast-mode mechanical.
- Cite round-10 evaluator handoff.
