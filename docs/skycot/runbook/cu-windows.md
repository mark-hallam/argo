# Computer-use runbook — Windows

> Operator guide for the native Windows computer-use executor
> (`argo-executor-computer-use-windows`). Sibling of
> `cu-macos.md`; if the macOS one is familiar, the structure
> here will read the same.

## Quick start

1. Open `argo.toml` and add the `[executor.computer_use.windows]`
   section:

   ```toml
   [executor.computer_use.windows]
   allow_send_keys = true
   dpi_aware = true
   screenshot_kind = "per_monitor"
   click_settle_ms = 50
   key_delay_ms = 15
   ```

2. Restart argo so the new DPI-awareness declaration takes effect.
   DPI awareness must be set at process start; mid-process changes
   do not reliably propagate to the compositor.

3. Run preflight:

   ```powershell
   argo cu windows preflight
   ```

   You should see all six axes pass. If any fail, see the
   "Failure triage" section below for the one-line fix.

## Preflight axes

| Axis | What it checks | Failure means |
| --- | --- | --- |
| `UacConsent` | argo's own integrity level | argo can't query its own token — broken environment |
| `SecureDesktop` | session is `WTSActive`, not `WTSDisconnected` / on the secure desktop | UAC prompt, lock screen, or Ctrl-Alt-Del menu is visible |
| `DpiAwareness` | process is per-monitor v2 DPI aware | `dpi_aware` is off or wasn't set before argo started |
| `A11yService` | `IUIAutomation` instantiation succeeds | UIAutomation service disabled in services.msc |
| `MonitorCount` | at least one monitor enumerated | RDP-disconnected or all displays detached |
| `PrimaryMonitor` | primary monitor identified | display arrangement lacks a designated primary |

## Enabling UIAutomation programmatic access

UIAutomation requires the `Microsoft UI Automation Service` to be
running. On most Windows installs it starts automatically; on
locked-down corporate images it's sometimes disabled by group
policy.

To enable it manually:

1. Open `services.msc` (Win+R → services.msc → Enter).
2. Find `Microsoft UI Automation Service`.
3. Right-click → Properties.
4. Set **Startup type** to `Automatic`.
5. Click **Start**.
6. Click OK and restart argo.

If the service is greyed out, group policy is enforcing the
disabled state. Contact the workstation administrator.

## Multi-monitor coordinate model

- The **virtual screen** is the union of every attached monitor's
  rectangle.
- The **primary monitor's** top-left corner is the virtual-screen
  origin `(0, 0)`.
- Secondary monitors positioned to the left or above the primary
  carry **negative** virtual coordinates.
- `MOUSEEVENTF_ABSOLUTE` normalises coordinates to `[0, 65535]`
  across the **entire virtual screen** (not per-monitor). The
  `normalise_mouse_coords` helper does the conversion.
- Screenshots are captured per-monitor by default
  (`screenshot_kind = "per_monitor"`). Each PNG carries its own
  scale factor and virtual-screen offset metadata so the
  Anthropic CU loop can map model-coordinates back to the right
  monitor.

## DPI awareness

Argo declares **per-monitor v2** DPI awareness at process start
(`SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2)`).
This is required so:

- Screenshots return physical pixels, not virtual (scaled) pixels.
- `BitBlt`-captured images are crisp on high-density displays.
- Click coordinates land where the operator expects, regardless of
  per-monitor scale factor.

If you change the operator's `dpi_aware` setting after argo has
started, restart argo. Mid-process DPI changes are unreliable.

## Known limitations

### Elevation parity required

Windows enforces UIPI (User Interface Privilege Isolation). Argo
cannot inject input into a target whose integrity level differs
from argo's own. Practical implications:

- If the target is **elevated** (running as administrator), launch
  argo with **Run as administrator**.
- If the target is a **standard-user** app, do NOT launch argo
  elevated — it will be unable to drive the target.

The preflight axis `UacConsent` records argo's own integrity
level. Mismatches surface at runtime as
`WindowsCuError::IntegrityMismatch`.

### Secure desktop blocks injection silently

When a UAC prompt is visible, the workstation is locked, or the
user has hit Ctrl-Alt-Del, the OS routes input to the secure
desktop and rejects user-mode `SendInput` calls. The preflight
axis `SecureDesktop` detects this state pre-flight; runtime
detection surfaces as `WindowsCuError::InputBlocked`.

### Out of scope (v1)

- **RDP / Citrix indirect-input passthrough** — install argo on
  the remote Windows host instead.
- **UWP / WinUI XAML automation** — v1 targets Win32 + classic
  WinForms / WPF surfaces. UWP store apps work for screenshot
  capture, but UIAutomation introspection support varies.
- **WSL bridge** — WSL is not a Windows desktop for CU purposes.

## Failure triage

| Symptom | First thing to check |
| --- | --- |
| Preflight passed but click does nothing | Secure-desktop state — is a UAC prompt visible? Is the workstation locked? |
| Preflight passed but screenshots are blank | Sleep / lock state — is the display powered off? |
| Preflight passed but click lands off-target | DPI awareness — restart argo with `dpi_aware = true` |
| `WindowsCuError::IntegrityMismatch` | UAC elevation parity — relaunch argo at the target's integrity level |
| `WindowsCuError::A11yLookupFailed` | `Microsoft UI Automation Service` running? Target app exposing automation properties? |
| `WindowsCuError::MonitorEnumerationFailed` | RDP-disconnected? Attach a monitor or run argo on the console session |
| `WindowsCuError::InputBlocked` | Secure-desktop or session-lock — dismiss the prompt and retry |

## Reference

- The executor crate is `crates/argo-executor-computer-use-windows/`.
- The macOS sibling lives at `crates/argo-executor-computer-use-macos/`.
- The cross-platform loop + Anthropic CU client is
  `crates/argo-executor-computer-use/`.
- The mission brief is
  `docs/build-plan/streams/mission-cu-windows-executor.md`.
