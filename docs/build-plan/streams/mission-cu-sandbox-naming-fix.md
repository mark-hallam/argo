# Mission load-bearing wire — disambiguate `Sandbox` type aliases at import sites

**Owner**: claude. **Depends on**: none (pure import-alias refactor).
**Authored against**: origin/main `cb0c0a90` on `2026-05-10`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-cu-sandbox-naming-fix`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 5 item #98 ("Resolve Sandbox naming hazard"). Three distinct `Sandbox` types coexist in the workspace: `argo_executor_computer_use::e2b::Sandbox` (E2B remote VM), `argo_sandbox::Sandbox` (local code-execution sandbox trait), and `argo_executor_filesystem::Sandbox` (filesystem isolation primitive). Today every import site reaches for `Sandbox` plain — readers in unfamiliar code can't tell which one they're looking at without scrolling to the import block. Mission Claim 5 ("real-world execution") risks confusing test scaffolding when an unrelated bug touches the wrong sandbox layer.

## Scope

Apply `as` aliases at every import site so the type's role is obvious in the code body, not buried in the imports:

- `argo_executor_computer_use::e2b::Sandbox` → import as `E2bSandbox`
- `argo_sandbox::Sandbox` → keep as `Sandbox` (it's the canonical workspace seam)
- `argo_executor_filesystem::Sandbox` → import as `FsSandbox`

This is a pure refactor: no public API change, no behavior change. The struct names themselves stay — only the *use* sites get the alias. Downstream rustdoc references that link to `[Sandbox]` continue to resolve via fully-qualified paths.

This is NOT in scope: renaming the structs themselves (would break public API and require coordinated downstream updates); introducing a unifying trait; collapsing the three sandbox types into one (they're genuinely different abstractions); modifying any sandbox implementation logic.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `crates/argo-executor-computer-use/src/pool.rs` — `use crate::e2b::{E2bClient, Sandbox};` → `use crate::e2b::{E2bClient, Sandbox as E2bSandbox};` and update body references.
- `crates/argo-executor-computer-use/src/executor.rs` — same pattern.
- `crates/argo-executor-computer-use/src/e2b.rs` — narrowly scoped: rustdoc on `pub struct Sandbox` carries a `// alias as E2bSandbox at use sites` comment. NO struct rename.
- `crates/argo-cli/src/handlers/worker_actions_executor_stubs_filesystem.rs` — `use argo_executor_filesystem::{ops as fs_ops, FsOpKind, Sandbox};` → `... Sandbox as FsSandbox` + body refs.
- `crates/argo-cli/src/handlers/worker_actions_executor_code.rs` — `argo_sandbox::Sandbox` stays as `Sandbox` (canonical); NO change needed UNLESS the same file also imports from another sandbox module (verify in first 5 minutes).
- `crates/argo-tool/src/tools/terminal.rs` — `argo_sandbox::Sandbox` stays as `Sandbox`; verify and confirm no other sandbox import collides.
- `crates/argo-tool/src/tools/code_execute.rs` — same as terminal.rs.
- Any test files that import `Sandbox` from the renamed crates need matching alias updates.

## Files you must NOT touch

- Any sandbox implementation file (the structs themselves, their impl blocks, their tests of behavior)
- `crates/argo-store/migrations/**` — no schema change
- Any vertical manifest
- Any documentation file outside the inline rustdoc comment in `e2b.rs`

## Acceptance criteria

- [ ] Every `use ...e2b::Sandbox` in the workspace is aliased as `E2bSandbox` at the use site
- [ ] Every `use argo_executor_filesystem::Sandbox` is aliased as `FsSandbox`
- [ ] `argo_sandbox::Sandbox` stays as plain `Sandbox` everywhere (canonical workspace seam)
- [ ] No `pub struct Sandbox` is renamed in any source file (public API stable)
- [ ] Body references to the aliased types are updated consistently (a body still calling `Sandbox` after the alias rename is a compile error caught by the worker)
- [ ] One inline rustdoc comment added to `e2b.rs`'s `pub struct Sandbox` noting the alias convention
- [ ] No public API change observable from outside `crates/argo-executor-computer-use` and `crates/argo-executor-filesystem`
- [ ] `cargo build --workspace` clean
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-executor-computer-use -p argo-executor-filesystem -p argo-tool -p argo-cli` passes (skip pre-existing flakes per KNOWN_FLAKES.md)
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-cu-sandbox-naming-fix`
- PR title: `refactor: alias Sandbox imports as E2bSandbox / FsSandbox at use sites (mission-cu-sandbox-naming-fix)`

## Notes for the implementer

- Use `rg -n 'use.*::Sandbox' crates/` to enumerate every import site at the start. The brief lists the known sites but recompiling against current main is the source of truth.
- This is a "find/replace + verify" PR. The interesting part is making sure body references are also updated — the compiler will catch missed ones via `error: cannot find type Sandbox in this scope` (or via wrong-type errors when one alias was forgotten and a different `Sandbox` resolved).
- If you find a place where multiple sandbox types are imported AND used in the same function, that's an ideal example to call out in the PR body — it's exactly the readability win this refactor exists for.
- Don't add new public API. The brief is a no-functional-change refactor; if you find yourself adding a method, you've drifted scope.
- Test files: aliases apply consistently. A test that uses `Sandbox` directly after the import-side alias change won't compile.
