# argo-executor-computer-use-windows

Native Windows computer-use proof of concept. Mirrors the macOS
(`argo-executor-computer-use-macos`, #941) and Linux POCs in shape so a
future generic `NativeComputerUse` trait can subsume all three.

## System requirements

- Windows 7 or newer (no installers needed; PowerShell 5.1+ ships
  built-in on every modern Windows SKU).
- An interactive desktop session. Headless / RDP-disconnected /
  service-account contexts have no foreground window and the runtime
  probe will report `Unsupported`.

No external installers. Every primitive shells out to `powershell.exe`
with `-NoProfile -NonInteractive -Command` plus an inline script using
either the built-in `System.Drawing` / `System.Windows.Forms`
assemblies or `user32.dll` P/Invoke.

## Platform support

The crate compiles on macOS and Linux as well as Windows. On non-Windows
hosts the runtime probe reports `Unsupported` and every primitive
returns `WindowsCuError::Unsupported` immediately, so the workspace
stays cross-platform.

## Security model

- All PowerShell invocations use `-NoProfile -NonInteractive` and pass
  the script via `-Command`. Profiles are not loaded; AMSI is **not**
  bypassed.
- The crate **never** writes `Set-ExecutionPolicy` or
  `-ExecutionPolicy Bypass`. A unit test asserts the produced command
  lines do not contain those strings.
- Subprocesses run un-elevated. This executor cannot escape the
  invoking user's normal Windows permissions: anything the operator
  could not click on themselves, the agent cannot click on either.
- Inputs that flow into PowerShell strings (file paths, window-search
  patterns, typed text) are escaped via single-quote doubling, plus
  the SendKeys meta-character escape rules per MSDN.

## Example

```rust,no_run
use argo_executor_computer_use_windows::{SessionOptions, WindowsComputerUse};

# fn demo() -> Result<(), argo_executor_computer_use_windows::WindowsCuError> {
let cu = WindowsComputerUse::new();
if cu.probe().is_supported() {
    let session = cu.session(SessionOptions::default());
    let _log = session.run_example()?;
}
# Ok(())
# }
```

## Follow-ups

- Dispatch wiring (an `ExecutorKind::ComputerUseWindows` variant + thin
  adapter mirroring `mission-cu-native-macos-executor-dispatch` /
  `#955`) is a separate brief.
- Persistent PowerShell session to amortize the ~150 ms startup cost.
- A future `NativeComputerUse` trait that abstracts macOS, Linux, and
  Windows behind one surface.
