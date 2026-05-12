# argo-executor-computer-use-linux

Native Linux computer-use executor with two display-server backends.

## System dependencies

### X11 backend

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

### Wayland backend

- `wtype` for typing and key emulation.
- `grim` for screenshot fallback (Sway, Hyprland).
- `xdg-desktop-portal` + the compositor-specific backend
  (`xdg-desktop-portal-gnome`, `xdg-desktop-portal-kde`, etc.)
  for cross-compositor Screenshot and RemoteDesktop APIs.

On Debian or Ubuntu:

```bash
sudo apt-get install wtype grim xdg-desktop-portal
# Plus your compositor's portal backend, e.g.:
sudo apt-get install xdg-desktop-portal-gnome
```

## Platform support

The crate compiles on macOS and other non-Linux hosts, but the
runtime probe reports `Unsupported` there.

The probe inspects `WAYLAND_DISPLAY` first and falls back to
`$DISPLAY` (X11) when no Wayland session is running.

### Compositor support matrix (Wayland)

| Compositor          | Screenshot       | Typing  | Mouse click          |
|---------------------|------------------|---------|----------------------|
| GNOME ≥ 42          | portal           | `wtype` | portal RemoteDesktop |
| KDE Plasma ≥ 5.27   | portal           | `wtype` | portal RemoteDesktop |
| Sway ≥ 1.7          | `grim`           | `wtype` | not supported (v1)   |
| Hyprland            | `grim`           | `wtype` | not supported (v1)   |
| Other Wayland       | probe Unsupported | —       | —                    |

The portal RemoteDesktop API prompts the user for consent on the
first input call per session. This is by design — Wayland's security
model requires user consent for input emulation, and that consent
prompt cannot be bypassed silently.

### v1 fail-closed paths

- **Mouse click on Sway / Hyprland**: returns `Unsupported` with a
  compositor-name hint, because those compositors do not advertise
  portal RemoteDesktop. Typing still works via `wtype`. The
  `mission-enterprise-wayland-libei` hardening brief tracks the
  migration to `libei` (native input-emulation protocol) which is
  the strategic replacement.

### Headless / CI

The probe reports `Unsupported` on hosts without a Wayland or X11
session. Use the E2B sandbox executor for CI runs that need a
desktop.

## Example

```rust,no_run
use argo_executor_computer_use_linux::{
    DisplayMode, LinuxComputerUse, SessionOptions,
};

# fn demo() -> Result<(), argo_executor_computer_use_linux::LinuxCuError> {
let cu = LinuxComputerUse::new();
let report = cu.probe();
if report.is_supported() {
    match report.display_mode {
        DisplayMode::Wayland(ref caps) => {
            tracing::info!(
                compositor = caps.compositor.as_str(),
                "running native CU on Wayland",
            );
        }
        DisplayMode::X11 => tracing::info!("running native CU on X11"),
        DisplayMode::None => {}
    }
    let session = cu.session(SessionOptions::default());
    let _log = session.run_example()?;
}
# Ok(())
# }
```

## Testing

Unit tests run cross-platform with `cargo test`. Wayland
integration tests are gated by an environment variable so they run
only on a real Wayland session:

```bash
ARGO_WAYLAND_INTEGRATION=1 cargo test -p argo-executor-computer-use-linux \
  --features wayland-integration -- --ignored
```
