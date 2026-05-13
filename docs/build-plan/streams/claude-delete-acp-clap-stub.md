# claude-delete-acp-clap-stub — delete unreachable `Command::Acp` clap variant

**Owner**: claude. **Depends on**: nothing.
**Authored against**: origin/main `6e3e31d8` on `2026-05-14`.

> Audit cluster 5 found `Command::Acp { cmd: AcpCmd }` declared in `cli_command_enum.rs:54` + `cli_subcommands.rs:143` `pub enum AcpCmd { Serve }`. `main.rs::dispatch` has NO `Command::Acp` arm — falls through to the `Some(other) => exit(2, "not yet wired")` path. `argo acp serve` always errors. The `argo-acp` crate exists but `argo-cli/Cargo.toml` has no entry.

## Scope

Pure delete: remove the `Command::Acp` clap variant + `AcpCmd` enum + any leftover references. Keep the `argo-acp` crate (different concern — out of scope here; may become Phase 3 wire-or-delete).

## Files you may touch
- `crates/argo-cli/src/cli_command_enum.rs` — remove `Command::Acp` arm. ≤5 LOC.
- `crates/argo-cli/src/cli_subcommands.rs` — remove `AcpCmd` enum. ≤10 LOC.
- `crates/argo-cli/src/main.rs::dispatch` — remove any acp-related fall-through. ≤5 LOC.

## Acceptance
- [ ] `argo acp` subcommand no longer in `argo --help`.
- [ ] `cargo build` clean.
- [ ] Workspace clippy clean.

## Branch
- `claude-delete-acp-clap-stub`
