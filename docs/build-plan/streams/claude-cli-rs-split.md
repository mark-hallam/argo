# V1.x — Architectural prep: split cli.rs

**Owner**: claude. **Depends on**: nothing (pure refactor).

## Scope

`crates/argo-cli/src/cli.rs` is at **509 LOC** — 9 lines over the
project's 500-LOC ceiling. This blocks any brief adding a new
top-level subcommand variant. **claude-installer-macos** hit this
ceiling and was marked BLOCKED tick183.

This is the 6th split in the pattern — proven approach:
- #851 (worker_actions split, tick89)
- #882 (cli_subcommands split, tick127)
- #909 (skill handlers split, tick181)
- #910 (routes_channels split, tick181)
- queued: claude-argo-store-lib-split (tick182)
- now: cli.rs split (this brief)

## Approach

Looking at cli.rs structure:
- ~80 lines of `pub use cli_<domain>_cmds::<X>Cmd;` re-exports
- The `Command` enum with ~30+ top-level variants (each ~3 lines)
- `Cli` struct + parse setup

Cleanest split:
- Keep `Cli` struct + `Command` enum dispatch + small wrapper logic
  in `cli.rs` (target ≤ 300 LOC)
- Move the `pub use` re-export block to `cli_reexports.rs`
  (or `cli_prelude.rs`) — pure pass-through, no logic
- The `Command` enum itself stays in cli.rs but variants reference
  types from `cli_reexports`

After:
- `cli.rs` ≤ 300 LOC (room for 5-10 new top-level variants)
- `cli_reexports.rs` ≤ 150 LOC (pure pass-through)
- Public API stable

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- `crates/argo-cli/src/cli.rs` — reduce to ≤ 300 LOC; keep
  public re-exports of `Command` + `Cli`
- `crates/argo-cli/src/cli_reexports.rs` (NEW, or
  `cli_prelude.rs` — refactorer's choice) — pure `pub use` block
- Any consumer that imports a type from `cli::<X>Cmd` —
  narrowly: change import path if a symbol moved (mechanical;
  should be zero edits if `pub use` re-exports are right)

## Acceptance criteria
- [ ] `cli.rs` ≤ 300 LOC
- [ ] New sibling file ≤ 150 LOC
- [ ] No public-API change (every existing call site still
      compiles unchanged)
- [ ] `cargo build --workspace` clean
- [ ] `cargo test --workspace` — same set of tests pass as
      before (no test deletions; no skips)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean
- [ ] Headroom for next 5-10 top-level subcommand additions
      before cli.rs approaches the 500 ceiling

## Branch + PR
- Branch: `claude-cli-rs-split`
- PR title: `refactor: split cli.rs under 500 LOC budget (claude-cli-rs-split)`
