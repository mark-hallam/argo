# argo-executor-computer-use-linux

Native Linux computer-use proof of concept for X11 desktops.

## System dependencies

- `xdotool` for mouse, keyboard, window search, and focus.
- `scrot` for PNG screenshots.

On Debian or Ubuntu:

```bash
sudo apt-get install xdotool scrot
```

On Fedora:

```bash
sudo dnf install xdotool scrot
```

## Platform support

The crate compiles on macOS and other non-Linux hosts, but the runtime
probe reports `Unsupported` there. On Linux, the probe requires both
`xdotool` and `scrot` on `PATH` plus an X11 display.

## Wayland gap

Wayland support is a follow-up. The expected primitive set is `grim`
for screenshots and `wtype` or compositor-specific protocols for input.
Track this as the follow-up issue: native Linux CU Wayland backend.

## Example

```rust,no_run
use argo_executor_computer_use_linux::{LinuxComputerUse, SessionOptions};

# fn demo() -> Result<(), argo_executor_computer_use_linux::LinuxCuError> {
let cu = LinuxComputerUse::new();
if cu.probe().is_supported() {
    let session = cu.session(SessionOptions::default());
    let _log = session.run_example()?;
}
# Ok(())
# }
```
