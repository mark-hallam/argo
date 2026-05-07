# claude-cli-rs-split-v2 — split cli.rs (500 LOC) into per-command-family clap siblings

**Owner**: claude. **Depends on**: nothing. **Phase A architectural-prep — 10th split in proven pattern (#851/#882/#909/#910/#914/#915/#932/#935/routes-get-split).**

> **REWRITE NOTE**: a prior version of this brief targeted a 509-LOC overflow
> with a single re-export sibling. Since then cli.rs landed back at the 500
> ceiling exactly via the previous split, and another wave of CLI commands
> is again pressing against the budget. This rewrite uses the proven
> per-command-family split pattern from #882 (cli_subcommands) and applies
> it to the latest growth (session messages/title/clear-skills, forge
> runs abort-all, session replay --stream, etc.).

## Why

`crates/argo-cli/src/cli.rs` is at the **500-LOC ceiling exactly** (HARD RULE 6). Every future top-level CLI command addition is BLOCKED until this is split. Recent commands burned through the remaining headroom and the next clap variant addition will fail CI.

The file currently holds:
- Top-level `Cli` struct + `Commands` enum (the dispatch shape)
- ~30 per-command-family `*Args` / `*Subcommand` clap derive structs and enums inline (Session, Skill, Worker, Memory, Doctor, Forge, Gateway, etc.)
- A handful of `derive(ValueEnum)` helpers used by multiple subcommands

This brief mirrors the same surgery that `routes.rs` (#routes-get-split TODO) and `routes_dispatch_post.rs` (#932) used: extract each command family into a sibling clap module, leaving `cli.rs` as a thin re-export + the top-level enum.

## Approach (concrete)

**Pure refactor — zero behaviour change.** Every help string, every short/long flag, every default value preserved verbatim. Public API of `cli.rs` (the `Cli` struct + `Commands` enum + `parse()`) stays identical.

1. **NEW `cli_commands_session.rs`** (~120 LOC) — `SessionArgs` + `SessionSubcommand` + every variant struct (`SessionMessagesArgs`, `SessionTitleArgs`, `SessionClearSkillsArgs`, `SessionReplayArgs`, etc.). `pub` everything that `cli.rs` currently `pub`s.

2. **NEW `cli_commands_skill.rs`** (~80 LOC) — `SkillArgs` + `SkillSubcommand` + variant structs.

3. **NEW `cli_commands_worker.rs`** (~120 LOC) — `WorkerArgs` + `WorkerSubcommand` + variant structs (Create, List, Inspect, Logs, etc.).

4. **NEW `cli_commands_memory.rs`** (~80 LOC) — `MemoryArgs` + `MemorySubcommand` + variant structs.

5. **NEW `cli_commands_doctor.rs`** (~60 LOC) — `DoctorArgs` + variant structs.

6. **NEW `cli_commands_forge.rs`** (~100 LOC) — `ForgeArgs` + `ForgeSubcommand` + variant structs (Plan, Run, Optimize, RunsAbortAll, etc.).

7. **NEW `cli_commands_gateway.rs`** (~80 LOC) — `GatewayArgs` + `GatewaySubcommand`.

8. **Other families as needed** — `cli_commands_budget.rs`, `cli_commands_marketplace.rs`, `cli_commands_vertical.rs`, `cli_commands_identity.rs`. Carve only families large enough to matter; small families can stay co-located in `cli_commands_misc.rs`.

9. **Modify `cli.rs`** — keep only: top-level `Cli`, `Commands`, `pub use` re-exports of every extracted struct/enum, plus the `parse()` helper. Net delta should drop cli.rs from 500 LOC to ≤ 200 LOC.

10. **lib.rs**: add one `mod cli_commands_*;` line per new file (or fold into a single `mod cli_commands;` parent that re-exports — pick whichever yields cleaner imports).

## Files you may touch

- BLANKET CLI ALLOWLIST applies
- `crates/argo-cli/src/cli_commands_session.rs` (NEW, ≤ 200 LOC)
- `crates/argo-cli/src/cli_commands_skill.rs` (NEW, ≤ 150 LOC)
- `crates/argo-cli/src/cli_commands_worker.rs` (NEW, ≤ 200 LOC)
- `crates/argo-cli/src/cli_commands_memory.rs` (NEW, ≤ 150 LOC)
- `crates/argo-cli/src/cli_commands_doctor.rs` (NEW, ≤ 120 LOC)
- `crates/argo-cli/src/cli_commands_forge.rs` (NEW, ≤ 200 LOC)
- `crates/argo-cli/src/cli_commands_gateway.rs` (NEW, ≤ 150 LOC)
- `crates/argo-cli/src/cli_commands_misc.rs` (NEW, ≤ 200 LOC) — small families
- `crates/argo-cli/src/cli.rs` — narrowly: delete the family-arg structs, add `pub use` re-exports, keep top-level enum
- `crates/argo-cli/src/lib.rs` — narrowly: add `mod cli_commands_*;` decls

## Files you must NOT touch

- Any `handlers/*.rs` — handler dispatch keys off the `Commands` variants which stay shaped identically
- Any test file (help-smoke tests should pass unchanged — every clap variant is preserved)
- `argo-cli-help` golden files — no help-text change

## Acceptance criteria

- [ ] `cli.rs` drops to ≤ 200 LOC (was 500)
- [ ] Every new sibling ≤ 250 LOC
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-cli` clean
- [ ] Help-smoke golden tests pass unchanged (no help-text drift)
- [ ] Public API stable: `argo_cli::Cli`, `argo_cli::Commands`, every previously-public clap struct importable from `argo_cli` via re-export
- [ ] `gh pr view --json mergeable` returns MERGEABLE post-push

## Branch + PR

- Branch: `claude-cli-rs-split`
- PR title: `cli: split cli.rs into per-command-family clap siblings (claude-cli-rs-split)`

## Notes for the implementer

- This is the **10th split in the proven pattern**. Read `routes_dispatch_post.rs` (290 LOC) end-to-end first to internalise the shape.
- Watch for shared `derive(ValueEnum)` helpers — these likely belong in `cli_commands_misc.rs` (or a sibling `cli_value_enums.rs`) so multiple families can import them.
- Help-smoke tests are the safety net — if they pass unchanged, the refactor is correct.
- This unblocks every queued CLI feature brief that adds a new top-level command or subcommand variant. Conductor will queue follow-ups as TODO once this lands.
