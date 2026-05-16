# codex-rustfmt-broadside — Workspace-wide rustfmt cleanup (post-F-01) (Wave 1.5 P1)

**Owner**: codex. **Depends on**: F-01 `codex-ci-workflows-reactivate` MERGED (✅ #1732).
**Authored against**: origin/main `489e0580` on `2026-05-16`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-rustfmt-broadside`.

## Mission alignment

- **CLAUDE.md hygiene** — re-activated `ci.yml` has a `fmt` job that will fail on workspace-wide pre-existing fmt drift. Must be fixed for CI to be green.

## Scope

P1. Run `cargo fmt --all` across the workspace. Single broad-touch commit. No semantic changes; pure formatting.

LOC: estimated 5000-15000 lines of pure fmt churn (no logic change).

## Files you may touch

- All `.rs` files in `crates/**` — but only formatting changes.
- `rustfmt.toml` (NEW or extend) — narrowly: pin formatting rules to match operator preferences. ≤30 LOC.

## Files you must NOT touch

- Non-Rust files.
- Any logic.
- Any test outcomes.

## Acceptance criteria

- [ ] `cargo fmt --all -- --check` returns 0 (clean).
- [ ] `cargo test --workspace` passes (no test broke).
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] PR body explicitly states this is fmt-only; reviewer can scan diff with `git diff -w` to verify no semantic change.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `codex-rustfmt-broadside`
- PR title: `chore: workspace-wide rustfmt cleanup (codex-rustfmt-broadside)`

## Notes

- Run `cargo fmt --all` then commit.
- Some files may have fmt-skipping pragmas (`#[rustfmt::skip]`). Preserve those.
- If `rustfmt.toml` doesn't exist, create one with the operator's preferences (default config or 4-space indent + max_width=100).
