# tool-terminal-background-processes — background processes + `process` lifecycle tool

**Owner**: claude. **Depends on**: nothing (independent of `tool-terminal-sandbox-backend`; can land in either order). **Tier B — CLI execution depth.**

**Authored against**: origin/main 7785708b on 2026-05-10.

## Why

Today every `terminal` call blocks until the child exits. An agent that wants to
`tail -f /var/log/foo` or run `npm run dev` and keep working has no path. Hermes
solves this with `process_registry.py` + a `process` lifecycle tool. We mirror
that: a `background: true` flag on `terminal` returns a `process_id` immediately,
and a NEW `process` tool exposes `list|status|tail|kill`. The capability split
keeps the danger contained — `Spawn::BackgroundProcess` is a strictly stronger
permission than the existing `Spawn::Subprocess`.

## Approach (concrete)

1. **NEW `crates/argo-tool/src/process_registry.rs`**: `pub struct
   ProcessRegistry { inner: Arc<RwLock<HashMap<ProcessId, ProcessHandle>>> }`.
   `ProcessHandle` holds the `tokio::process::Child`, ring-buffered stdout/stderr
   tails (~64 KB each, mirroring `MAX_OUTPUT_BYTES`), spawn metadata (command,
   started_at, pid, status). Methods: `spawn`, `list`, `status`, `tail`, `kill`,
   `reap_finished`. All operations `async`. Process IDs are UUIDv4 strings.
2. **NEW `SpawnKind::BackgroundProcess` variant** in `crates/argo-types/src/capability.rs`
   — small, additive. The variant is the *only* `argo-types` change. Do NOT
   touch any other capability code paths beyond exhaustive-match additions
   (HARD RULE 5 EXHAUSTIVE-MATCH amendment applies).
3. **`terminal.rs` extension**: add `background: Option<bool>` to `Args`. When
   `true`, `dispatch` requires `Spawn::BackgroundProcess`, calls
   `ProcessRegistry::spawn`, returns a one-line `ToolCallResult` containing the
   new `process_id` instead of stdout. The registry handle lives on
   `ToolContext` as a new optional field `background_processes: Option<Arc<ProcessRegistry>>`
   (graceful absence = "background unsupported in this context" error).
4. **NEW `crates/argo-tool/src/tools/process.rs`**: implements `Tool` with
   `name = "process"` and `action: enum { List, Status, Tail, Kill }` plus
   optional `pid: String`. `list` returns all live processes; `status`/`tail`/`kill`
   require `pid`. `tail` returns the most recent ~32 KB of buffered stdout +
   stderr. Required capability: `Spawn::BackgroundProcess`.
5. **Wire registration** in `crates/argo-tool/src/tools/mod.rs`: add `Process`
   to `builtin_tools` and `builtin_tools_with_memory`. Update the
   `builtin_tools_returns_eight_unique_names` test to expect 9 names. Update
   the doc comment to say "nine in-core tools".
6. **Allow `process` and background `terminal` in one vertical**: add an
   `engineering`-style allow stanza. Default verticals should NOT get
   `Spawn::BackgroundProcess` — if no current vertical is appropriate,
   document the decision in the PR body and propose a follow-up
   `engineering-vertical-template` brief.
7. Tests + rustdoc + ensure `process_registry.rs`, `process.rs`, and `terminal.rs`
   each stay under 500 LOC. Registry is the most likely to bloat; split the
   ring-buffer logic into a private `tail_buffer` helper module if needed.

## Files you may touch

- `crates/argo-tool/src/process_registry.rs` — **NEW**, expected ~280 LOC, **must stay <500**
- `crates/argo-tool/src/tools/process.rs` — **NEW**, expected ~220 LOC, **must stay <500**
- `crates/argo-tool/src/tools/terminal.rs` — add `background` flag; +~60 LOC
- `crates/argo-tool/src/tools/mod.rs` — register the new tool, +~10 LOC
- `crates/argo-tool/src/tool.rs` — add optional `background_processes` field on `ToolContext`, +~6 LOC
- `crates/argo-tool/src/lib.rs` — `pub mod process_registry;` + re-export, +~3 LOC
- `crates/argo-types/src/capability.rs` — add `SpawnKind::BackgroundProcess` variant + serde rename, +~3 LOC
- Match arms touching `SpawnKind::*` across the workspace — EXHAUSTIVE-MATCH amendment, ONLY the new arm
- One vertical manifest (or PR-body deferral note)

## Files you must NOT touch

- `crates/argo-runtime/src/` — no scheduler/dispatch changes; the registry plugs in via `ToolContext`
- `crates/argo-sandbox/` — orthogonal concern; sandbox + background can compose later
- Any other `tools/*.rs` file beyond `terminal.rs`, `process.rs`, `mod.rs`

## Acceptance criteria

- [ ] `terminal { command, background: true }` returns `process_id`, not stdout
- [ ] `process { action: "list" }` returns all live processes
- [ ] `process { action: "tail", pid }` returns last ~32 KB stdout+stderr
- [ ] `process { action: "kill", pid }` SIGTERMs and reaps within 2s; `SIGKILL` after grace timeout
- [ ] `Spawn::BackgroundProcess` is a distinct capability; covers itself but NOT `Spawn::Subprocess` and vice versa
- [ ] At least one vertical's manifest allows `process` (or PR body documents deferral)
- [ ] 3+ unit tests for `ProcessRegistry` (spawn/list/kill, tail truncation, reap)
- [ ] 3+ unit tests for `process` tool (each action)
- [ ] 1 integration test: spawn long-running cmd, list, tail, kill — round-trip
- [ ] Rustdoc on `ProcessRegistry`, `Process`, and the new `terminal.background` field with examples
- [ ] `cargo clippy -p argo-tool -p argo-types --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-tool -p argo-types` green
- [ ] No file >500 LOC; no fn >80 LOC

## Branch + PR

- Branch: `tool-terminal-background-processes`
- PR title: `Tier B — process registry + terminal background flag + process lifecycle tool`
