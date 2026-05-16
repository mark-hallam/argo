# codex-span-naming-cleanup — Fix raw span literals (post-F-01) (Wave 1.5 P2)

**Owner**: codex. **Depends on**: F-01 MERGED (✅ #1732).
**Authored against**: origin/main `489e0580` on `2026-05-16`.

## Scope

P2. The re-activated `ci.yml` runs `scripts/check-span-naming.sh` which fails on raw `tracing::info_span!("...")` literals — convention is to use named constants from `argo-telemetry::span_names` for greppability.

Fix all callsites.

LOC: ~150 (mechanical).

## Files you may touch

- Files surfaced by `bash scripts/check-span-naming.sh 2>&1 | grep ':'` — typically in `work_loop.rs`, gateway route files, agent-runner.
- `crates/argo-telemetry/src/span_names.rs` (extend) — add any missing constant names.

## Acceptance criteria

- [ ] `bash scripts/check-span-naming.sh` exits 0.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] No span-literal in production code.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `codex-span-naming-cleanup`
- PR title: `chore: replace raw span literals with named constants (codex-span-naming-cleanup)`
