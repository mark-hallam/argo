# win-cli-code-executor-build-command-posix-syntax

**Owner**: windows-fleet — **P2 CROSS-PLATFORM — the code-executor bakes POSIX shell syntax (`ulimit`/`2>/dev/null`/`;`) into its command → fails under cmd.exe on native Windows.**
**Depends on**: none.
**Authored against**: origin/main `ce99d4e23` on `2026-06-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh win-cli-code-executor-build-command-posix-syntax`
**Fleet**: WINDOWS (BUILD_STATUS_WINDOWS.md). Build+verify on Windows; the FIX is cross-platform (unix path stays byte-identical) so the conductor ALSO re-verifies on unix before merge. Push branch, IN_REVIEW; conductor reviews+merges. **Do NOT self-merge.**
Source: docs/skycot/windows/PORTABILITY-AUDIT-2-2026-06-15.md — finding **3 of 7**.

## Problem (VERIFIED on origin/main `ce99d4e23`)

`crates/argo-cli/src/handlers/worker_actions_executor_code.rs:423` builds the
code-exec command string with **POSIX shell syntax**:

```rust
"ulimit -v {mem_kib} 2>/dev/null; {runner} {}"
```

- `ulimit` is a POSIX/bash builtin (no equivalent on cmd.exe),
- `2>/dev/null` is POSIX redirection (cmd.exe uses `2>NUL`),
- `;` is a POSIX command separator (cmd.exe uses `&`).

This is run via the shell (the same `bash -c` family the LocalSandbox/terminal
paths use — see #2944/#2948). On native Windows there is no bash by default, and
even under cmd.exe this string is invalid → the local code-executor fallback
(taken when `ARGO_E2B_TOKEN` is unset, `worker_actions_executor_code.rs:~258`)
fails. The memory-limit + stderr-suppression + sequencing are all POSIX-only.

## Fix

Make the build-command platform-aware (mirror the #2944/#2948 shell-selection
pattern + the LocalSandbox cfg-gate). Lowest-risk approach:
- `#[cfg(unix)]`: keep the exact current string (`ulimit -v … 2>/dev/null; …`) —
  byte-identical, no behavior change on macOS/Linux.
- `#[cfg(windows)]`: drop the unsupported `ulimit` (document that the memory cap
  is a no-op / best-effort on Windows — honest gap), use `2>NUL` + run the
  `{runner} {script}` directly (no `;`-sequencing needed once ulimit is gone),
  OR — consistent with `win-sandbox-localsandbox-bash-cfg-gate` (#2948, which
  made LocalSandbox return a clean error off-unix) — if the local code-executor
  is already effectively unix-only, return a clear `Err` on Windows ("local
  code-execution requires E2B on Windows; set ARGO_E2B_TOKEN") rather than a
  broken spawn. Pick whichever matches how #2948 left the sibling sandbox path
  (check it — do NOT introduce a divergent third behavior).
- A cross-platform unit test asserting the unix string is unchanged + the
  Windows arm produces a valid command (or the documented honest error).

Keep the file < 500 LOC; no `unwrap`/`expect` outside `#[cfg(test)]`.

## Verify (RUN it — NO THEATER)
- On Windows: `cargo build -p argo-cli` clean; `cargo test -p argo-cli` for the
  code-executor module green (incl. the new arm test); clippy `-D warnings` clean.
  If feasible, drive a trivial code-exec action on Windows and confirm it runs
  (or returns the honest "needs E2B" error) instead of a broken-command failure.
- Conductor re-verifies on unix: `cargo test -p argo-cli` (unix arm unchanged →
  byte-identical) + the ulimit-capped path still works on macOS/Linux.

## Out of scope
- Implementing a real Windows memory cap (Job Objects) — document the gap; a
  separate brief if wanted.

**LOC budget:** ~40–100 (cfg-split the command builder + a test).
