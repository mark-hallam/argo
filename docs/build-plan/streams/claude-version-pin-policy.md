# claude-version-pin-policy — workspace-level dep pinning audit + CI enforcement

**Owner**: claude. **Depends on**: nothing. **Phase B hygiene.**

## Why

Workspace deps should be pinned at the **workspace `Cargo.toml`** level (`[workspace.dependencies]`) and inherited per-crate via `<dep>.workspace = true`. Today this is mostly the case but **drift is creeping in**:

- A handful of crates declare per-crate versions for deps that *also* live in `[workspace.dependencies]` (latent inconsistency)
- New crates added by recent swarm PRs occasionally specify `tokio = "1"` (loose) when the workspace pins `tokio = "1.41"`
- Some dev-deps differ from the workspace pin in test crates

Result: `cargo build` resolves multiple versions of the same dep silently, increasing binary size + obscuring upgrade impact. The HARD RULE intent ("workspace pinning is canonical") is not enforced.

## Approach (concrete)

**Audit + enforcement script + CI check.** No production code change.

### Phase 1 — audit (read-only)

1. Walk every `crates/*/Cargo.toml`.
2. For each `[dependencies]` / `[dev-dependencies]` / `[build-dependencies]` table:
   - Note any dep with a literal version (e.g. `tokio = "1.41"`) instead of `tokio.workspace = true`
   - Note any dep with a path-only entry that isn't workspace (acceptable for intra-workspace crates)
3. Cross-check against `[workspace.dependencies]` in root `Cargo.toml`. Any literal-version dep that *also* exists in workspace is a drift candidate; rewrite to `dep.workspace = true`.
4. Any literal-version dep NOT in workspace is a candidate for promotion to workspace (if 2+ crates use it).

### Phase 2 — enforcement script

1. **NEW `scripts/check-version-pinning.sh`** (~100 LOC):
   - Parses every `crates/*/Cargo.toml` (use `cargo metadata --format-version 1 | jq` — the dep table is structured)
   - For each dep, asserts: if dep name is in `[workspace.dependencies]`, the per-crate entry must use `.workspace = true` (no literal version)
   - Exits non-zero on violation, printing `crate=X dep=Y violation=literal-version-in-presence-of-workspace-pin`

### Phase 3 — rewrite drift

1. For each violation found in Phase 1: narrowly rewrite the per-crate entry to `dep.workspace = true`.
2. Promote 2+-crate-shared deps to `[workspace.dependencies]` if not already there.

### Phase 4 — CI check

1. Add a step to existing CI that runs `scripts/check-version-pinning.sh`.

## Files you may touch

- `Cargo.toml` (root, workspace) — narrowly: promote 2+-crate-shared deps if any
- Every `crates/*/Cargo.toml` with a drift violation — narrowly: rewrite to `.workspace = true`
- `scripts/check-version-pinning.sh` (NEW, ≤ 150 LOC)
- `.github/workflows/*.yml` (existing CI workflow) — narrowly: 1 new step
- `docs/skycot/VERSION_PINNING.md` (NEW, ≤ 80 LOC) — 1-page contract

## Files you must NOT touch

- Any source code (this is purely build-config + scripts)
- Cargo.lock (let cargo regenerate)
- Any test file

## Acceptance criteria

- [ ] `scripts/check-version-pinning.sh` runs locally and exits 0 (no violations)
- [ ] CI runs the script on every PR
- [ ] `cargo build` produces no duplicate-version warnings for workspace-pinned deps
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` clean
- [ ] `docs/skycot/VERSION_PINNING.md` documents the contract for future contributors
- [ ] Every modified file ≤ 500 LOC
- [ ] `gh pr view --json mergeable` returns MERGEABLE post-push

## Branch + PR

- Branch: `claude-version-pin-policy`
- PR title: `deps: workspace-level version pinning audit + CI enforcement (claude-version-pin-policy)`

## Notes for the implementer

- `cargo metadata --format-version 1 | jq '.packages[] | {name, dependencies}'` gives you the structured view. Easier than parsing TOML directly.
- A few deps are legitimately per-crate-only (e.g. test-only `pretty_assertions` in 1 crate). Those are fine — the rule is "if it's in `[workspace.dependencies]`, the per-crate entry must inherit; otherwise per-crate is allowed".
- Path deps for intra-workspace crates (`argo-store = { path = "../argo-store" }`) should be `argo-store.workspace = true` if the workspace declares `argo-store = { path = "crates/argo-store" }`. Confirm the workspace style first.
- Don't bump any version as part of this brief — it's a hygiene pass, not an upgrade. File follow-ups for upgrades.
- Lock-file regeneration is expected. Don't fight it.
