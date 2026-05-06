# Stream H (follow-up) — Orchestrator CLI subcommands

**Owner**: codex. **Depends on**: H-orchestrator-persist merged.

## Scope

Operator-facing CLI subcommands so a human can list + inspect
sessions and handoffs from the terminal without going through
the gateway. Mirrors the shape of `argo worker actions list /
show` from prior work.

Subcommands:
- `argo session start --tenant <id> --title <text>` →
  prints the new session id.
- `argo session list [--status active|resolved|cancelled|
  expired] [--limit N]` → table or JSON.
- `argo session show <session_id>` → details + participants +
  pending handoffs.
- `argo handoff list [--session <id>] [--status pending|
  acknowledged|settled|expired] [--worker <id>]` → table or
  JSON.
- `argo handoff show <handoff_id>` → details + payload preview.

Universal `--json` per existing CLI convention.

## Files you may touch
- `crates/argo-cli/src/cli_session_cmds.rs` (NEW — clap
  subcommand definitions for session + handoff)
- `crates/argo-cli/src/handlers/session.rs` (NEW — handler
  bodies that call `OrchestratorStore`)
- `crates/argo-cli/src/cli.rs` — narrowly scoped: register
  the two new subcommand families. Add ONLY the `Session(...)`
  and `Handoff(...)` arms; do NOT modify other arms.
- `crates/argo-cli/src/main.rs` — narrowly scoped: dispatch
  the two new arms to `handlers::session`. ~10 lines.
- `crates/argo-cli/src/handlers/mod.rs` — ONE line `pub mod
  session;`.
- `crates/argo-cli/Cargo.toml` — narrowly scoped: add
  `argo-orchestrator` path dep (workspace member already).

## Files you must NOT touch
- `crates/argo-orchestrator/**` — uses the persist API only;
  doesn't extend it.
- `crates/argo-cli/src/cli_*` other than the new file.
- `crates/argo-cli/src/handlers/gateway/**` — HTTP routes
  land in `H-orchestrator-routes`.

## Acceptance criteria
- [ ] All 5 subcommands wired + dispatch through clap.
- [ ] `--json` flag on every list/show subcommand.
- [ ] `argo session list --status active` filters correctly.
- [ ] `argo handoff list --session <id>` filters correctly.
- [ ] Integration test: spin up an in-memory store, insert a
      session + handoff, run each subcommand via the
      `assert_cmd` pattern, assert table + JSON output shapes.
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings`
      clean
- [ ] Every modified file < 500 LOC

## Branch + PR
- Branch: `codex-H-orchestrator-cli`
- PR title: `cli: argo session + handoff subcommands (H-orchestrator-cli)`

## Notes
- Mirror the table-rendering shape of
  `argo worker actions list` (look at
  `crates/argo-cli/src/handlers/worker_actions.rs` for the
  pattern). Use `comfy-table` if needed (already a workspace
  dep).
- Do NOT add a new dep just for this.
