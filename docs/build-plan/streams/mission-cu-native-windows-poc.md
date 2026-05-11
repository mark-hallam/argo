# mission-cu-native-windows-poc — native Windows computer-use companion crate (mirror of #941 macOS / Linux POCs)

**Owner**: claude. **Depends on**: nothing — standalone crate, mirrors `argo-executor-computer-use-macos` (#941) and `argo-executor-computer-use-linux` (`claude-cu-native-linux-poc`). Dispatch wiring is a follow-up brief (cf. `mission-cu-native-macos-executor-dispatch` #955).
**Authored against**: origin/main `70e4b020` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-cu-native-windows-poc`.

## Why

Mission claim 5 says "real-world execution = any task a human can do on a PC. Workers must have BOTH MCP AND computer use." macOS is shipped (#941 / #955). Linux POC is in-flight. **Windows desktop workers have no native CU path today** — they fall through to E2B (a Linux microVM, not the user's actual Windows desktop). This brief ships the Windows-side companion so a Windows operator can run native CU against their own machine without an E2B round-trip. This closes the third leg of "any PC the user owns".

## Scope

NEW crate `crates/argo-executor-computer-use-windows` modeled exactly on `argo-executor-computer-use-macos` and `argo-executor-computer-use-linux`. Same public surface (`WindowsComputerUse::screenshot()`, `::click(x, y)`, `::key(vk)`, `::type(text)`, `::session(opts) -> Session`). Subprocess-based primitives using **PowerShell built-ins only** (no external installers required — parity with macOS's built-in `screencapture` / `osascript`):

- **screenshot** → `powershell.exe -NoProfile -Command "Add-Type -AssemblyName System.Drawing,System.Windows.Forms; ..."` capturing the primary screen to `<out.png>`
- **mouse** → `powershell.exe` P/Invoke into `user32.dll` (`SetCursorPos`, `mouse_event`) wrapped in a small inline script
- **keyboard** → `powershell.exe` `[System.Windows.Forms.SendKeys]::SendWait(...)` for text; `keybd_event` P/Invoke for non-printable keys
- **window enum / focus** → `powershell.exe Get-Process | Where-Object MainWindowTitle ...` + `user32!SetForegroundWindow`

The crate must compile cleanly on macOS + Linux (deps optional, runtime probes report `Unsupported`) so the workspace stays cross-platform.

## Files you may touch

- `crates/argo-executor-computer-use-windows/**` (NEW crate, ≤ 1800 LOC across files)
  - `Cargo.toml` — minimal deps: `serde`, `tokio`, `argo-types`, `argo-error`, `tracing`. No new workspace deps. **Do NOT pull in the `windows` crate or `enigo` or `screenshots`** — keep subprocess-only for v1 parity with mac/Linux POCs.
  - `src/lib.rs` (≤ 80 LOC) — module decls + re-exports
  - `src/screenshot.rs` (≤ 220 LOC) — PowerShell-built-in screenshot wrapper, error path when not Windows
  - `src/input.rs` (≤ 280 LOC) — PowerShell mouse + keyboard wrappers, virtual-key code table
  - `src/window.rs` (≤ 200 LOC) — window enumeration + focus via PowerShell
  - `src/session.rs` (≤ 300 LOC) — orchestration + step log (match macOS/Linux session.rs structure 1:1)
  - `src/probe.rs` (≤ 120 LOC) — runtime probe for PowerShell ≥ 5.1 presence; report `Unsupported` cleanly on mac/Linux
  - `src/error.rs` (≤ 100 LOC) — local error enum + `From` to `ArgoError`
  - sibling `_tests.rs` per non-trivial file (≤ 250 LOC each); skip Windows-only paths via `#[cfg(target_os = "windows")]`
- `Cargo.toml` (workspace root) — narrowly: 1 `members =` line addition
- `README.md` inside the new crate — ~40 LOC, lists PowerShell requirement (built-in on Windows 7+) and notes "no installers needed"

## Files you must NOT touch

- `argo-executor-computer-use-macos` — separate crate; do NOT modify
- `argo-executor-computer-use-linux` — separate crate; do NOT modify
- `argo-executor-computer-use` (E2B variant) — separate
- `worker_actions_executor.rs` / `ExecutorKind` — dispatch wiring is a sibling brief; this one is a standalone POC crate
- `argo-vertical/src/manifest.rs` — no schema change

## Acceptance criteria

- [ ] New crate `argo-executor-computer-use-windows` builds clean on Windows + macOS + Linux (latter two with runtime probe = `Unsupported`)
- [ ] Public API surface matches `argo-executor-computer-use-macos` 1:1 (screenshot, click, key, type, session)
- [ ] On a Windows dev box, the `session` example takes a screenshot, clicks (100, 100), types "hello", and writes a JSON step log
- [ ] Unit tests for each subprocess wrapper with PowerShell mocked via PATH stub (or via a `PowerShellRunner` trait the test substitutes)
- [ ] Probe returns `Supported` only when `powershell.exe` is on PATH AND `$PSVersionTable.PSVersion.Major -ge 5`; `Unsupported(reason)` otherwise
- [ ] `cargo build --workspace && cargo clippy -p argo-executor-computer-use-windows -- -D warnings && cargo test -p argo-executor-computer-use-windows` clean
- [ ] Every file < 500 LOC
- [ ] README documents the security model: PowerShell runs un-elevated; no AMSI bypass; explicit note that this executor cannot escape the user's normal permissions
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-cu-native-windows-poc`
- PR title: `executor(cu): native Windows computer-use POC crate (mission-cu-native-windows-poc)`

## Notes for the implementer

- Read the macOS crate (#941) AND the Linux POC end-to-end before starting. Your file layout and trait shapes should mirror them exactly so a future generic `NativeComputerUse` trait can subsume all three.
- PowerShell virtual-key codes are documented at MSDN `Microsoft.PowerShell.Commands.SendKeys` — keep the mapping in `input.rs` table-driven for testability.
- `SendKeys::SendWait` mishandles `+`, `^`, `%`, `(`, `)`, `{`, `}` — escape them per MSDN rules. Test this explicitly.
- The PowerShell process startup cost is ~150ms. The macOS `osascript` cost is similar; do not pre-emptively cache a long-lived process for v1 — match the mac/Linux POC patterns exactly. A persistent PowerShell session is a follow-up optimization.
- Headless CI: a Windows runner without an interactive session will fail screenshot; the runtime probe must catch this and report `Unsupported(reason="no interactive session")`.
- Dispatch wiring (ExecutorKind variant + adapter file) is a separate follow-up brief mirroring `mission-cu-native-macos-executor-dispatch`. Do NOT add it here.
- **Security**: do not write `Set-ExecutionPolicy` calls or `-ExecutionPolicy Bypass` flags. Use `-NoProfile -NonInteractive -Command` and inline scripts only. Lock this in a unit test that greps the produced command lines.
