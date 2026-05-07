# V1.4 — Skill signing CLI dependency prerequisite

**Owner**: codex. **Depends on**: none.
**Unblocks**: codex-skill-sign-cli.

> **NEW PREREQUISITE BRIEF (tick 269 by conductor)**: the original
> `codex-skill-sign-cli` brief assumed `ed25519-dalek` was a
> transitive workspace dep. It is not. This narrow brief adds it.
> Once merged, `codex-skill-sign-cli` unblocks naturally.

## Scope

Tiny dependency-add brief. Single deliverable: add `ed25519-dalek`
to the workspace and to `crates/argo-skill/Cargo.toml`. No code
changes.

## Approach

1. Add `ed25519-dalek = { version = "2", default-features = false, features = ["std", "rand_core"] }`
   to the workspace `[workspace.dependencies]` block in the root
   `Cargo.toml`.
2. Add `ed25519-dalek = { workspace = true }` to
   `crates/argo-skill/Cargo.toml` `[dependencies]`.
3. Run `cargo build -p argo-skill` to confirm the dep resolves.
4. Add a single trivial doctest or unit test in
   `crates/argo-skill/src/lib.rs` (or a new `src/sign_dep_smoke.rs`)
   that calls `ed25519_dalek::SigningKey::generate` to prove the
   feature flags are correct. Test should be ≤ 30 LOC.

## Files you may touch

- `Cargo.toml` (workspace) — narrowly: 1 line under
  `[workspace.dependencies]`
- `crates/argo-skill/Cargo.toml` — narrowly: 1 line under
  `[dependencies]`
- `crates/argo-skill/src/sign_dep_smoke.rs` (NEW — ≤ 30 LOC)
- `crates/argo-skill/src/lib.rs` — narrowly: 1 `mod` line if the
  smoke test lives in a sibling file

## Files you must NOT touch

- Any other crate's `Cargo.toml`
- Any signing logic (that lands in `codex-skill-sign-cli`)
- argo-skill public API

## Acceptance criteria

- [ ] `ed25519-dalek = { version = "2", ... }` in workspace
      `Cargo.toml`
- [ ] `crates/argo-skill/Cargo.toml` pulls it via `workspace = true`
- [ ] `cargo build -p argo-skill` succeeds
- [ ] One smoke test instantiates `SigningKey::generate` and
      confirms it produces a 32-byte verifying-key
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Per-crate dep allowlist (argo-skill) updated if needed —
      check `project_argo_dep_allowlists.md`
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `codex-skill-sign-cli-deps`
- PR title: `skill: add ed25519-dalek workspace dep (codex-skill-sign-cli-deps)`

## Notes for the implementer

- Brief should ship as a tiny PR (~30 LOC of net change).
- After merge, the dependent `codex-skill-sign-cli` brief flips
  BLOCKED → TODO and can pick up the actual signing logic.
- If the dep allowlist for argo-skill rejects `ed25519-dalek`,
  amend the allowlist as part of this PR (it's a load-bearing
  V1.4 dep, not a one-off).
