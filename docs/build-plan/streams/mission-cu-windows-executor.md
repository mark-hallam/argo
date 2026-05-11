# Mission load-bearing wire — Native Windows computer-use executor

**Owner**: claude. **Depends on**: SHIPPED `crates/argo-executor-computer-use-macos/` (canonical native CU executor pattern); existing `crates/argo-executor-computer-use/` (cross-platform CU loop + dispatch types); existing `mission-cu-native-linux-executor-dispatch` brief (parallel platform; coordinate test fixtures).
**Authored against**: origin/main `ea8b4406` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-cu-windows-executor`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 5 #101 ("Build native Windows computer-use executor"). **Tier A (executor).** Today macOS native CU is SHIPPED, linux dispatch is queued. Windows is the missing leg of the three-OS native-CU triad. This brief lands the Windows executor crate following the macOS structure 1:1 — Win32 + UIAutomation for accessibility-tree introspection, BitBlt + GDI+ for screenshots, SendInput for mouse/keyboard injection, preflight + a11y probe + screenshot + executor loop.

## Scope

NEW `crates/argo-executor-computer-use-windows/` crate (≤ 1850 LOC across 9 files — mirroring the macOS crate 1:1):

- `crates/argo-executor-computer-use-windows/Cargo.toml` (NEW, ≤ 45 LOC) — pulls in `windows` (Win32_UI_Accessibility, Win32_UI_Input_KeyboardAndMouse, Win32_Graphics_Gdi, Win32_System_Threading, Win32_System_Com), `image`, `tokio`, `tracing`, `thiserror`, `serde`. Dev-deps `argo-types` `argo-channels`.
- `crates/argo-executor-computer-use-windows/src/lib.rs` (NEW, ≤ 70 LOC) — re-exports + rustdoc with usage example.
- `crates/argo-executor-computer-use-windows/src/executor.rs` (NEW, ≤ 300 LOC) — main `WindowsExecutor` struct; implements the existing `Executor` trait from `argo-executor-computer-use`. Step-loop dispatch to mouse / keyboard / screenshot / a11y operations.
- `crates/argo-executor-computer-use-windows/src/a11y.rs` (NEW, ≤ 130 LOC) — UIAutomation tree traversal. `find_element_by_role + name`, `find_element_by_automation_id`, `bounding_rect`. Returns `A11yElement { role, name, automation_id, rect, role_count_for_dedupe }`.
- `crates/argo-executor-computer-use-windows/src/screenshot.rs` (NEW, ≤ 220 LOC) — full-desktop BitBlt → PNG via the `image` crate. Multi-monitor enumeration via `EnumDisplayMonitors`. Returns `Vec<MonitorScreenshot { monitor_id, image_bytes, scale_factor }>`.
- `crates/argo-executor-computer-use-windows/src/config.rs` (NEW, ≤ 175 LOC) — `WindowsCuConfig { allow_send_keys: bool, screenshot_kind: Full | PerMonitor, click_settle_ms: u64, key_delay_ms: u64, dpi_aware: bool }`. Reads from `[executor.computer_use.windows]` config section if present (additive — does NOT collide with the macOS section).
- `crates/argo-executor-computer-use-windows/src/error.rs` (NEW, ≤ 170 LOC) — `WindowsCuError` enum: `Win32Error { hresult, ctx }`, `A11yLookupFailed`, `ScreenshotFailed`, `InputBlocked (UAC / secure-desktop)`, `MonitorEnumerationFailed`, etc. `From<windows::core::Error>` impl.
- `crates/argo-executor-computer-use-windows/src/preflight.rs` (NEW, ≤ 360 LOC) — Windows-specific preflight checks: UAC consent status, secure-desktop detection (a known input-blocked state), DPI awareness, display-scaling sanity, a11y service ("Microsoft UI Automation Service") running, monitor count, primary-monitor identified. Returns `WindowsPreflightReport`.
- `crates/argo-executor-computer-use-windows/src/events.rs` (NEW, ≤ 400 LOC) — input-event emission via `SendInput`: mouse move (absolute via `MOUSEEVENTF_ABSOLUTE`), mouse click (left/right/middle/double), wheel scroll, key press/release/combo, type-string. Handles dead-time + settle-ms.
- `crates/argo-executor-computer-use-windows/src/notes.rs` (NEW, ≤ 110 LOC) — Windows-platform notes: "no DPI override w/o `dpi_aware=true`", "secure-desktop blocks SendInput silently — preflight detects", "UAC-elevated processes can't inject into non-elevated UIs and vice-versa". 

NEW unit tests inline via `#[cfg(test)]` modules (5 per non-trivial file ≈ 25 unit tests total, ≤ 100 LOC per test-module). Each test that touches Win32 API should be `#[cfg(target_os = "windows")]` gated — non-Windows CI runs do `cargo check` only.

NEW dispatch wiring (mirrors the existing macOS pattern):
- EXTEND `crates/argo-executor-computer-use/src/executor.rs` (≤ 25 LOC delta) — register the Windows backend via `#[cfg(target_os = "windows")]` branch in the dispatch factory.
- EXTEND `Cargo.toml` (workspace) — add `argo-executor-computer-use-windows` to `members` (build-gated `[target.'cfg(target_os = "windows")']`).

NEW operator runbook `docs/skycot/runbook/cu-windows.md` (≤ 220 LOC):
- Windows-specific preflight checks (UAC / secure-desktop / DPI / a11y)
- How to enable UIAutomation programmatic access (operator setting)
- Multi-monitor coordinate model (top-left of primary = origin; secondary monitors offset positive/negative)
- DPI-awareness expectation (per-monitor v2)
- Known limitations: cannot inject into elevated UIs from non-elevated; cannot inject during secure-desktop (UAC prompt / lock-screen / Ctrl-Alt-Del)
- Failure-class triage: "preflight passed but click does nothing" → check secure-desktop state; "screenshot is blank" → check sleep / lock state

NO Rust changes outside the new crate + the small dispatch-wire delta in `argo-executor-computer-use`. NO migration. NO config-schema breakage (additive section only).

This is NOT in scope: Linux native executor (separate `mission-cu-native-linux-executor-dispatch` brief); the per-OS dispatch HARNESS / browser-CU loop (already shipped in `argo-executor-computer-use`); Win32-side OCR (delegate to existing OCR pipeline); Windows-specific clipboard manipulation beyond what's already in the generic CU loop; UWP / WinUI XAML automation (the v1 targets Win32 + classic WinForms / WPF surfaces); Citrix / RDP indirect-input passthrough (out-of-scope; that's a deployment concern); WSL bridge (out-of-scope; WSL ≠ Windows desktop for CU purposes).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-executor-computer-use-windows/Cargo.toml` (NEW, ≤ 45 LOC)
- `crates/argo-executor-computer-use-windows/src/lib.rs` (NEW, ≤ 70 LOC)
- `crates/argo-executor-computer-use-windows/src/executor.rs` (NEW, ≤ 300 LOC)
- `crates/argo-executor-computer-use-windows/src/a11y.rs` (NEW, ≤ 130 LOC)
- `crates/argo-executor-computer-use-windows/src/screenshot.rs` (NEW, ≤ 220 LOC)
- `crates/argo-executor-computer-use-windows/src/config.rs` (NEW, ≤ 175 LOC)
- `crates/argo-executor-computer-use-windows/src/error.rs` (NEW, ≤ 170 LOC)
- `crates/argo-executor-computer-use-windows/src/preflight.rs` (NEW, ≤ 360 LOC)
- `crates/argo-executor-computer-use-windows/src/events.rs` (NEW, ≤ 400 LOC)
- `crates/argo-executor-computer-use-windows/src/notes.rs` (NEW, ≤ 110 LOC)
- `crates/argo-executor-computer-use/src/executor.rs` (EXTEND, ≤ 25 LOC delta — register Windows backend via cfg gate)
- `Cargo.toml` (workspace EXTEND — add Windows crate to `members` w/ target-cfg gate)
- `docs/skycot/runbook/cu-windows.md` (NEW, ≤ 220 LOC)

## Files you must NOT touch

- `crates/argo-executor-computer-use-macos/**` (macOS executor stays)
- `crates/argo-executor-computer-use/src/executor.rs` beyond the ≤ 25 LOC dispatch-register delta (keep the cross-platform loop intact)
- `crates/argo-store/migrations/**` (no migration)

## Acceptance criteria

- [ ] `cargo check -p argo-executor-computer-use-windows --target x86_64-pc-windows-msvc` succeeds (non-Windows CI runs `cargo check` only)
- [ ] On a Windows test host, `cargo test -p argo-executor-computer-use-windows` passes all 25 unit tests
- [ ] `WindowsExecutor` implements the existing `Executor` trait without trait-changes
- [ ] `WindowsPreflightReport` covers: UAC / secure-desktop / DPI / a11y service / monitor count / primary-monitor-identified
- [ ] Multi-monitor screenshot returns one image per monitor with correct scale-factor metadata
- [ ] Dispatch factory routes to Windows backend when `target_os = "windows"`
- [ ] All new files < 500 LOC; per-file ≤ briefed budgets
- [ ] No changes to macOS executor or cross-platform CU loop beyond dispatch register
- [ ] Runbook covers all 6 Windows-specific failure modes (UAC / secure-desktop / DPI / a11y service / multi-monitor / elevated-vs-non-elevated)
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-cu-windows-executor`
- PR title: `executor: native Windows computer-use (UIAutomation + SendInput + BitBlt + multi-monitor) (mission-cu-windows-executor)`

## Notes for the implementer

- Treat `crates/argo-executor-computer-use-macos/src/` as the 1:1 layout reference. Same 9-file structure: lib + executor + a11y + screenshot + config + error + preflight + events + notes. Don't reorganize.
- The `windows` crate (Microsoft official) has a feature-gate per-namespace pattern — declare only the namespaces you need to keep compile time reasonable. The brief enumerates the minimum set.
- `SendInput` for mouse and keyboard is the v1 path. Don't reach for `keybd_event` / `mouse_event` (deprecated); SendInput is the supported modern API.
- UIAutomation tree traversal: use the COM-based `IUIAutomation` interface. Initialize COM via `CoInitializeEx(COINIT_MULTITHREADED)`. Document this in `lib.rs` so consumers know the runtime requirement.
- Multi-monitor coordinate system: virtual-screen origin is the top-left of the primary monitor; secondary monitors get negative coordinates if they're to the left/above. `GetSystemMetrics(SM_XVIRTUALSCREEN)` + `SM_YVIRTUALSCREEN` give the virtual-screen origin. The MOUSEEVENTF_ABSOLUTE coordinates are normalized to `[0, 65535]` across the entire virtual screen (NOT per-monitor).
- DPI awareness: declare per-monitor v2 via `SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2)` if `config.dpi_aware`. Without this, scale-factor-aware screenshots get blurred.
- Secure-desktop detection: check `WTSGetActiveConsoleSessionId` + `WTSQuerySessionInformation` for the active session's `WTSConnectState`. If the session is in `WTSDisconnected` or `WTSConnectQuery`, SendInput silently no-ops.
- UAC elevation parity: `IsUserAnAdmin` + `GetCurrentProcessToken`-based check. If the target window's process is elevated and Argo's is not (or vice-versa), input is silently blocked. Preflight flags this; runbook explains.
- The `notes.rs` file is for caveats that don't fit elsewhere — keep it short, scannable. Operators read this when something goes wrong.
- DO NOT reach for `winapi-rs` — the `windows` crate is the modern, Microsoft-blessed binding. Mixing is painful.
- DO NOT shell out to PowerShell for any of this. All input + screenshot + a11y goes through Win32 / UIAutomation directly. PowerShell-shellout is a v2 fallback if native binding becomes a problem.
- DO NOT add Citrix / RDP indirect-input support. Out of scope; if operators are running Argo over RDP, they should configure the local CU executor.
- Per-namespace `windows` features keep compile time tractable. Test on Windows once before shipping; the cargo-check-only non-Windows CI gate is fine for incremental.
- The runbook should be operator-actionable: each failure-class has a one-line "what to check first" answer. Don't waste lines re-explaining Win32 semantics.
