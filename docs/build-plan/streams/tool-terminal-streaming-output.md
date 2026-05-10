# tool-terminal-streaming-output — wire `terminal` stdout/stderr to a progress callback

**Owner**: claude. **Depends on**: nothing (orthogonal to sandbox-backend and background-processes; can land in any order). **Tier B — CLI execution depth.**

**Authored against**: origin/main 7785708b on 2026-05-10.

## Why

`terminal` today calls `Command::output().await` and dumps stdout/stderr only
after the child exits. Long-running commands (test suites, builds, log tailing)
give the agent loop no signal until the wall-clock timeout or completion. A
streaming callback — same shape as the `forge optimize --stream` `ForgeEvent`
pattern (#280) — lets the gateway / TUI / agent loop see output as it arrives,
which both improves UX and lets the agent abort early when output indicates an
obvious failure mode. This is a half-day wiring change, not new infrastructure.

## Approach (concrete)

1. **Pre-work — verify callback shape.** Run
   `grep -rn "tool_progress_callback\|ProgressCallback\|ForgeEvent" crates/`.
   Initial scan shows the established pattern is `argo_forge::ForgeEvent`
   (`crates/argo-forge/src/forge.rs:107`). There is **no pre-existing
   `tool_progress_callback`**, so this brief either (a) adopts the
   `ForgeEvent`-style channel pattern locally inside `argo-tool`, or
   (b) introduces a new `ToolProgressEvent` enum in `argo-tool::tool` and
   wires an optional `tx: Option<mpsc::Sender<ToolProgressEvent>>` onto
   `ToolContext`. Pick **(b)** — `ForgeEvent` is forge-domain-specific and
   shouldn't bleed into the tool layer.
2. **NEW `ToolProgressEvent` enum** in `crates/argo-tool/src/tool.rs`:
   `Stdout(String)`, `Stderr(String)`, `Exited { exit_code: i32 }`. Chunks
   are line-buffered and capped at 4 KB per event so the channel can't
   memory-explode.
3. **Add `progress_tx: Option<mpsc::Sender<ToolProgressEvent>>` field on
   `ToolContext`** (Tokio `mpsc`). Default constructor leaves it `None`
   so all existing call sites compile unchanged.
4. **Refactor `terminal::run_command`**: when `ctx.progress_tx.is_some()`,
   spawn the child with `stdout(Stdio::piped()).stderr(Stdio::piped())`,
   read each pipe with `BufReader::lines()` in concurrent tasks, and send
   each line as a `ToolProgressEvent` while *also* appending to the
   in-memory accumulator (preserving the existing batch return semantics
   so callers without a `progress_tx` see no behavior change). When
   `progress_tx` is `None`, fall through to the existing `Command::output()`
   path unchanged.
5. **Backpressure rule**: send via `try_send`; on `Full`, drop the chunk
   and bump a per-call dropped-bytes counter that's logged on completion.
   Never block the child on a slow consumer.
6. Output cap (`MAX_OUTPUT_BYTES`, 64 KB / stream) still applies to the
   accumulator; the channel sees full output up to backpressure.
7. Add tests + rustdoc example showing how a caller subscribes to events.

## Files you may touch

- `crates/argo-tool/src/tools/terminal.rs` — primary target; currently 238 LOC, expected ~360 LOC; **must stay <500**
- `crates/argo-tool/src/tool.rs` — add `ToolProgressEvent` enum + optional `progress_tx` field on `ToolContext`; +~30 LOC
- `crates/argo-tool/src/lib.rs` — re-export `ToolProgressEvent`; +~1 LOC

## Files you must NOT touch

- Any other tool implementation — streaming is opt-in and `terminal`-only in this brief; broadening to `file_*`, `web_*`, `delegate_task` is a follow-up
- `crates/argo-runtime/src/` — runtime/dispatcher unchanged; callers thread the channel through `ToolContext` themselves
- `crates/argo-forge/src/forge.rs` — leave `ForgeEvent` alone; the new enum is tool-layer-specific
- `crates/argo-types/` — `ToolProgressEvent` lives in `argo-tool`, not `argo-types` (it's tool-layer transport, not a wire type)

## Acceptance criteria

- [ ] `ToolContext::new` signature unchanged; new field defaults to `None`
- [ ] `terminal` with `progress_tx = None` is byte-for-byte identical to current behavior (existing tests pass unchanged)
- [ ] `terminal` with `progress_tx = Some(tx)` emits per-line `Stdout`/`Stderr` events plus a final `Exited`
- [ ] Backpressure: `try_send` on full channel does not block child output
- [ ] No additional capability needed — streaming is a callback shape, not a new privilege; `Spawn::Subprocess` still suffices
- [ ] 3+ unit tests: streaming round-trip, backpressure-doesn't-deadlock, no-callback path unchanged
- [ ] 1 integration test: subscribe a `mpsc::Receiver`, run a multi-line cmd, assert events arrive in order before the dispatch future resolves
- [ ] Rustdoc on `ToolProgressEvent` and the new `ToolContext` field with a runnable example
- [ ] `cargo clippy -p argo-tool --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-tool` green
- [ ] `terminal.rs` stays under 500 LOC; no fn >80 LOC

## Branch + PR

- Branch: `tool-terminal-streaming-output`
- PR title: `Tier B — terminal — stream stdout/stderr via ToolContext progress channel`
