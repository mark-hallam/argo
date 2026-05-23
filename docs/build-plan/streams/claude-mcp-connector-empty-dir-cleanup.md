# claude-mcp-connector-empty-dir-cleanup

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `<latest>` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-mcp-connector-empty-dir-cleanup`.

🟢 **P3 REPO HYGIENE — 7 empty MCP connector dirs left from prior deletes.** Source: evaluator round-10 SHARED.md handoff 2026-05-23T10:55:00Z.

> "claude-mcp-connector-empty-dir-cleanup (P3 — 7 empty dirs from prior deletes)"

Per v1.0 strategy lock (2026-05-14): 4 consumer/redundant MCP connectors were deleted (`signal`, `telegram`, `mailgun`, `resend`). The Cargo.toml entries + crate code went, but the empty `crates/argo-mcp-<name>/` directories were left behind. Repo noise + grep false-positives.

## Scope

1. `find crates/argo-mcp-* -type d -empty` → list candidates.
2. Cross-check against `git log --diff-filter=D -- crates/argo-mcp-<name>/` to verify each was deleted, not in-progress.
3. `git rm -r` each empty dir.
4. Verify no Cargo.toml reference remains to any of the deleted names.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `crates/argo-mcp-<empty-dir>/` — remove via `git rm -r`.
- `Cargo.toml` — narrowly scoped: remove any leftover dead refs.
- `docs/build-plan/BUILD_STATUS.md` — standard row flip.

## Files you must NOT touch

- Any non-empty MCP connector crate.
- Anything outside `crates/argo-mcp-*/`.

## Acceptance criteria

- [ ] All 7 empty connector dirs removed (or specific count discovered during pre-flight).
- [ ] No stale Cargo.toml ref to any removed connector.
- [ ] `cargo metadata --no-deps` succeeds.
- [ ] `bash scripts/workspace-clippy-fast-fail.sh` clean.
- [ ] BUILD_STATUS.md updated.

## Branch + PR

- Branch: `claude-mcp-connector-empty-dir-cleanup`
- PR title: `chore(mcp): remove 7 empty connector dirs left from v1-strategy deletes (claude-mcp-connector-empty-dir-cleanup)`

## Notes for the implementer

- ~7 `git rm -r` ops + any dead Cargo.toml line removal.
- **Don't delete dirs that look empty but have hidden files** — `ls -la` first.
- Cite round-10 evaluator handoff + v1.0 strategy lock (operator memory 2026-05-14).
