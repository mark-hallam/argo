# M11-eval-self-healing-fixtures - Track M (M-Eval), Phase 2, Tier 2

**Owner**: claude. **Depends on**: M9 MERGED.
**Authored against**: origin/main `5a5d3970` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh M11-eval-self-healing-fixtures`.

## Scope

When a fixture fails for a reason the LLM judges environmental (UI changed, selector moved, external API renamed a field), the eval runner proposes a fixture update. Operator confirms via UI (M10 dashboard). Mirrors UiPaths self-healing tests feature.

LOC: ~700. Branch: `claude-M11-eval-self-healing-fixtures`. PR title: `feat(eval): self-healing fixture proposals (M11)`.

## Files you may touch

- NEW crates/argo-eval/src/self_healing.rs
- NEW crates/argo-eval/src/self_healing_tests.rs

## Files you must NOT touch

- M9/M10 (consume APIs).
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] Synthetic environmental failure produces a fixture-update proposal.
- [ ] Operator approval applies the update.
- [ ] Logical failures NOT proposed for self-heal.
- [ ] 5 unit tests.
- [ ] No file > 500 LOC.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean (Rust-side).
- [ ] `pnpm typecheck` clean (web-side).
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-M11-eval-self-healing-fixtures`
- PR title: `feat(eval): self-healing fixture proposals (M11)`

## Notes for the implementer

- See `streams/M0-enterprise-parity-track-overview.md` for the authoritative spec for this brief.
- Mission-claim alignment: claim 4 (adaptive fixtures).
