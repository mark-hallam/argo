# Mission load-bearing wire — Reconcile `Sandbox` naming collision

**Owner**: claude. **Depends on**: none.
**Authored against**: origin/main `2ca2d694` on `2026-05-13`.

> SOURCE: MISSION_BACKLOG #265. Two unrelated types share the name `Sandbox`: the trait `argo_sandbox::Sandbox` (command-execution seam) and the struct `argo_executor_computer_use::e2b::Sandbox` (E2B handle). Today every file that uses both has to write `use crate::e2b::Sandbox as E2bSandbox` — and a couple of places forget, causing import-shadowing footguns. Rename the E2B side to `E2bSandbox` at source. **Tier-Refactor (zero behaviour change).**

## Scope

Pure rename in `argo-executor-computer-use`:

1. Rename `pub struct Sandbox` in `crates/argo-executor-computer-use/src/e2b.rs` to `pub struct E2bSandbox`. (Currently around line 280; verify via `rg -n 'pub struct Sandbox\b' crates/argo-executor-computer-use/src/e2b.rs`.)
2. Update every in-crate reference — there are ~6–10 call sites. The convenience alias `use crate::e2b::Sandbox as E2bSandbox` already used in `sandbox_prewarm.rs` becomes `use crate::e2b::E2bSandbox` (or just `crate::e2b::E2bSandbox` inline).
3. **Re-export** `pub use e2b::E2bSandbox` from `lib.rs` if there's a public re-export today (`grep -n 'pub use .*e2b' crates/argo-executor-computer-use/src/lib.rs` to check). External callers in `argo-cli` etc. update accordingly.
4. Verify the new name shows zero collisions: `rg -n 'use argo_sandbox::Sandbox' crates/` and `rg -n 'argo_executor_computer_use::E2bSandbox' crates/` should both compile clean alongside.

NOT in scope: refactoring the underlying types; merging the two namespaces (they're genuinely unrelated — the trait is a command-runner abstraction, the struct is an E2B remote-VM handle); renaming `SandboxPool` / `SandboxPrewarmer` (those are pool concepts, OK as-is).

## Files you may touch

- `crates/argo-executor-computer-use/src/e2b.rs` (≤ 490 LOC after) — rename declaration + internal references
- `crates/argo-executor-computer-use/src/lib.rs` — re-export adjustment if needed
- `crates/argo-executor-computer-use/src/pool.rs` — references via `E2bSandbox` once renamed
- `crates/argo-executor-computer-use/src/sandbox_prewarm.rs` — drop the `as E2bSandbox` alias
- Any test file under `crates/argo-executor-computer-use/src/*_tests.rs` and `crates/argo-executor-computer-use/tests/` referencing the old name
- Any `argo-cli` consumer site — `rg -n 'argo_executor_computer_use::.*Sandbox\b' crates/argo-cli/` to find them

## Files you must NOT touch

- `crates/argo-sandbox/` — the trait keeps its name; that's the canonical `Sandbox`
- Any behaviour code — this brief is a name change only. Diff should be pure-rename (no logic edits)
- `Cargo.toml` / `Cargo.lock` — no dep changes

## Acceptance criteria

- [ ] `pub struct E2bSandbox` replaces `pub struct Sandbox` in e2b.rs
- [ ] All in-crate references updated; the `as E2bSandbox` alias deleted at every site
- [ ] Public re-export (if any) is `pub use e2b::E2bSandbox`
- [ ] External callers in `argo-cli` updated (verify with `rg`)
- [ ] `cargo build --workspace` clean
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] No new tests required (pure rename) — but existing tests pass unchanged
- [ ] All touched files < 500 LOC; BUILD_STATUS.md updated

## Branch + PR

- Branch: `claude-mission-sandbox-namespace-reconcile`
- PR title: `executor-computer-use: rename Sandbox → E2bSandbox to disambiguate from argo_sandbox::Sandbox (mission-sandbox-namespace-reconcile)`

## Notes

- IDE rename will land 90% of this; the last 10% is convenience aliases (`use ... as E2bSandbox`) that need manual removal — leaving them in keeps the trail of breadcrumbs that this rename was needed.
- The PR diff should be entirely line-by-line `Sandbox` → `E2bSandbox` substitutions inside the one crate + its dependents. If you find yourself editing function bodies or signatures beyond the rename, you've scope-crept — back out.
- Worker may discover that an external dep (e.g. `argo-tool` or a vertical template TOML) references the type name in a string identifier. Verify before / after with `rg -F 'Sandbox' verticals/ docs/ --files-with-matches`. String references in docs/manifests where the term refers to the E2B handle should be updated; the trait references stay as `Sandbox`.
