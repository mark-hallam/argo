# mission-model-latest-resolution — Daemon refresh loop for provider 'model=latest' resolution

**Owner**: codex. **Depends on**: nothing.
**Authored against**: origin/main `31d8e20e` on `2026-05-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-model-latest-resolution`.

## Scope

MISSION_BACKLOG Amendment 8. `discovery.rs` filter+select logic is shipped. Add the daemon-side refresh scheduler that polls `[provider.discovery.refresh_interval_hours]` (default 24h), caches, and applies `resolve_latest()` on next model request.

LOC: ~180.

## Files you may touch

- `crates/argo-provider/src/transport.rs` (or discovery.rs sibling) — refresh scheduler. ≤80 modified lines.
- `crates/argo-config/src/sections_provider.rs` (NEW) — `refresh_interval_hours` parsing. ≤20 modified lines.
- Test sibling — mock-endpoint cache test. ≤80 LOC.

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

- Branch: `codex-mission-model-latest-resolution`
- PR title: `feat: Daemon refresh loop for provider 'model=latest' resolution (mission-model-latest-resolution)`

## Notes for the implementer

- Priority: provider UX.
- Authored as part of post-v1 polish backlog batch 2026-05-15.
