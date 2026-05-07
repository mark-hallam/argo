# claude-cli-rs-split-v2 — move Command enum body into a single sibling (rewrite v3)

**Owner**: claude. **Depends on**: nothing. **Phase A architectural-prep.**

> **REWRITE NOTE (tick379 by conductor)**: a prior version of this brief
> assumed `cli.rs` held many per-command-family `*Args` / `*Subcommand`
> structs to extract into per-family siblings. **3 prior tabs investigated
> and found the premise wrong**: per-family Subcommand enums are ALREADY
> extracted via `cli_reexports.rs` (~40 sub-modules). Actual `cli.rs`
> (500 LOC) is one monolithic `Command` enum body with 57+ inline
> variants (~442 LOC) that Rust's language semantics require to live in
> a single file/module. Rewrite v3 takes the worker-recommended **option
> (b) ship-the-80%** path: move the whole `Command` enum body to ONE
> sibling (~445 LOC) and let cli.rs become a ~30-LOC entry-point.

## Why

`crates/argo-cli/src/cli.rs` is at the **500-LOC ceiling exactly** (HARD RULE 6). Three Phase B worker briefs (`claude-worker-onboarding-cli`, `claude-worker-archive-rehydrate`, `claude-worker-introspection-cli`) are blocked because their CLI hooks need to add a single new variant to `Command` and that pushes cli.rs over 500. This brief makes that headroom exist.

## Approach (concrete) — option (b) ship-the-80%

1. **NEW `crates/argo-cli/src/cli_command_enum.rs`** (≤ 500 LOC): contains the entire `Command` enum body. Currently inline in cli.rs (~442 LOC). Move verbatim.
2. **Modify `crates/argo-cli/src/cli.rs`** (target: ≤ 50 LOC): keep the `Cli` struct (~12 LOC), the small `ToolsCmd` Subcommand enum that's already inline (~27 LOC), and `pub use cli_command_enum::Command;`. Drop everything else.
3. **No handlers/*.rs changes**: the `Command` variants stay shaped identically — handlers' pattern-matching keeps working unchanged.

The result:
- cli.rs: 500 LOC → ~50 LOC (450 LOC of headroom for ~10 future variant additions)
- cli_command_enum.rs: 0 LOC → ~445 LOC (55 LOC of variant headroom)
- HARD RULE 6 satisfied on both files
- Public API stable: `argo_cli::cli::Command` keeps working via the re-export

## Files you may touch

- `crates/argo-cli/src/cli.rs` — narrowly: drop the Command enum body, replace with `pub use cli_command_enum::Command;`. Target: ≤ 50 LOC.
- `crates/argo-cli/src/cli_command_enum.rs` (NEW, ≤ 500 LOC): the moved Command enum body. **Brief sibling-budget raised to ≤ 500 LOC** (vs the prior brief's ≤ 250 LOC) since the enum body is one monolithic block per Rust language semantics — splitting it across files would require touching every handler.
- `crates/argo-cli/src/lib.rs` (or main.rs) — narrowly: 1 `pub mod cli_command_enum;` decl if needed
- BLANKET CLI ALLOWLIST applies — cli.rs / main.rs / handlers/mod.rs / tests / Cargo.toml are all permitted

## Files you must NOT touch

- Any `crates/argo-cli/src/handlers/*.rs` — handler dispatch keys off the `Command` variants which stay shaped identically.
- Existing `cli_*.rs` siblings (cli_subcommands.rs, cli_reexports.rs, etc.) — those already-extracted files keep their current shapes.
- The existing per-family Subcommand enums (WorkerCmd, SkillCmd, etc.) — they live in their own siblings already; this brief doesn't reorganize them.

## Acceptance criteria

- [ ] `cli.rs` ≤ 50 LOC (was 500)
- [ ] `cli_command_enum.rs` ≤ 500 LOC (new file)
- [ ] `Command` re-exported via `pub use cli_command_enum::Command;` so external consumers (handlers, tests, doctests) keep compiling unchanged
- [ ] All existing tests pass without changes
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-cli` clean
- [ ] gh pr view --json mergeable returns MERGEABLE
- [ ] **CASCADE UNBLOCK**: confirm in PR body that this enables claude-worker-onboarding-cli, claude-worker-archive-rehydrate, claude-worker-introspection-cli to add their CLI variants without exceeding the 500 LOC ceiling.

## Branch + PR

- Branch: `claude-cli-rs-split-v2`
- PR title: `cli: move Command enum body into a single sibling (claude-cli-rs-split-v2)`

## Notes for the implementer

- This is a **2-step refactor**, not the multi-file split the prior brief attempted. Read the file once, copy 442 LOC into the new sibling, replace with `pub use`, update `lib.rs`/`main.rs` if needed.
- LOC accounting: cli.rs has Cli struct (~12) + ToolsCmd inline enum (~27) + Command body (~442) + imports/derives (~19) ≈ 500. After move: Cli + ToolsCmd + imports + `pub use` ≈ 50.
- Mission claim 1 (teammate): unblocks 3 Phase B claim-1 worker briefs once this lands. Conductor will flip those BLOCKED→TODO atomically post-merge.
- This rewrite (v3) is the result of 3 tabs investigating and converging on the same recommendation. The prior brief's "≤ 250 LOC sibling" was the misstep — Rust enum bodies are language-monolithic.
