# mission-acp-server-multi-harness — ACP server per-harness capability filtering on handshake

**Owner**: codex. **Depends on**: nothing.
**Authored against**: origin/main `31d8e20e` on `2026-05-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-acp-server-multi-harness`.

## Scope

MISSION_BACKLOG Amendment 11. `Harness` enum + `HarnessCapabilities` struct shipped. Wire the server handshake: detect harness from user-agent / explicit declaration frame; respond with filtered capability set.

LOC: ~220.

## Files you may touch

- `crates/argo-acp/src/server.rs` — narrowly: `handle_client_handshake()` filtering. ≤100 modified lines.
- `crates/argo-acp/src/wire.rs` — narrowly: protocol-frame handling for harness-decl frame. ≤30 modified lines.
- Test sibling — per-harness unit tests + 1 mock-client integration. ≤120 LOC.

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

- Branch: `codex-mission-acp-server-multi-harness`
- PR title: `feat: ACP server per-harness capability filtering on handshake (mission-acp-server-multi-harness)`

## Notes for the implementer

- Priority: ACP extensibility.
- Authored as part of post-v1 polish backlog batch 2026-05-15.
