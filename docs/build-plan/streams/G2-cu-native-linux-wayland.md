# Stream G2 — Native Linux Wayland support for computer-use executor

**Owner**: claude.
**Depends on**: `claude-cu-native-linux-poc` MERGED (X11 baseline crate).
**Authored against**: origin/main `63385265` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh G2-cu-native-linux-wayland`.
**Tier tag**: PRODUCT_ROADMAP.md Track G brief 2 of 3 — closes the second leg of mission claim 5 (Wayland coverage flagged in the X11 POC's open questions).

## v1 posture vs enterprise target

| Concern | v1 implementation | Architectural target | Closes via |
|---|---|---|---|
| Wayland input emulation | `wtype` for typing + xdg-desktop-portal RemoteDesktop for mouse / keys | Native `libei` (input emulation protocol) — emerging stack with growing compositor support | `mission-enterprise-wayland-libei` |
| Wayland screenshot | xdg-desktop-portal Screenshot API (cross-compositor) with `grim` fallback for sway/Hyprland | Same | n/a |
| Compositor support matrix | GNOME ≥ 42 / KDE Plasma ≥ 5.27 / sway ≥ 1.7 / Hyprland (best-effort) | Same; v1 documents supported matrix | n/a |
| User-consent gate | Wayland's portal API prompts the user on first input/screenshot attempt; user consents per session | Same; the consent prompt is by design — adversarial AI can't silently capture | n/a |
| Headless / CI runners | Probe returns Unsupported on headless boxes; fallback to E2B sandbox | Same | n/a |

v1 is acceptable: Wayland's security model is
inherently strict — the OS forces user-consent
prompts. This is a feature, not a v1 limitation.

## Why

The Linux POC (`claude-cu-native-linux-poc`) shipped
with X11-only support and an explicit open-question
flag: "Wayland coverage is explicit follow-up work —
file the issue stub but do not block this PR on it.
`grim` + `wtype` would be the eventual primitives."

This brief is that follow-up. Wayland is the
default on most modern Linux desktops (GNOME, KDE,
Sway, Hyprland). Without it, native Linux CU works
only on legacy X11 sessions — a shrinking footprint.

## Scope

Extend `argo-executor-computer-use-linux` (the
existing crate) with Wayland support behind an
auto-detected display-server probe:

- Probe enhancement: detect `$WAYLAND_DISPLAY` and
  the available portal — return `Supported` only
  when (a) compositor supports the required portal
  AND (b) required CLI tools (`grim` / `wtype`) OR
  D-Bus reachable.
- Screenshot path on Wayland: xdg-desktop-portal
  Screenshot API via D-Bus (cross-compositor); fall
  back to `grim` on compositors without portal
  support (Sway/Hyprland).
- Input path on Wayland: `wtype` for typing; mouse
  click via portal RemoteDesktop API (where
  available) or fail-closed with a clear "Wayland
  input emulation requires compositor support"
  error.
- Existing X11 path unchanged.
- Module structure: NEW `src/wayland/` directory in
  the existing Linux crate; `src/x11/` for the
  legacy primitives; `src/probe.rs` dispatches
  between them.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Shared hotspots: `crates/argo-executor-computer-use-linux/`
  (this brief extends the existing crate; no other
  crate touched).

## Files you may touch

- `crates/argo-executor-computer-use-linux/src/wayland/mod.rs`
  — NEW. ≤ 80 LOC.
- `crates/argo-executor-computer-use-linux/src/wayland/screenshot.rs`
  — NEW. Portal-based + grim fallback. ≤ 250 LOC.
- `crates/argo-executor-computer-use-linux/src/wayland/input.rs`
  — NEW. `wtype` + portal RemoteDesktop. ≤ 280
  LOC.
- `crates/argo-executor-computer-use-linux/src/wayland/portal.rs`
  — NEW. D-Bus client for xdg-desktop-portal. ≤
  300 LOC.
- `crates/argo-executor-computer-use-linux/src/wayland/probe.rs`
  — NEW. Compositor + tool detection. ≤ 200 LOC.
- `crates/argo-executor-computer-use-linux/src/x11/` (rename
  from current `src/*.rs` files except `lib.rs`,
  `probe.rs`, `session.rs`, `error.rs`) — narrowly
  scoped move; no logic change.
- `crates/argo-executor-computer-use-linux/src/lib.rs`
  — narrowly scoped: probe dispatch between Wayland
  + X11 modules. ≤ 100 LOC.
- `crates/argo-executor-computer-use-linux/src/probe.rs`
  — narrowly scoped: detect `WAYLAND_DISPLAY` first,
  fall back to `$DISPLAY` (X11). ≤ 180 LOC.
- `crates/argo-executor-computer-use-linux/Cargo.toml`
  — narrowly scoped: add ONE dep `zbus` for D-Bus.
- `crates/argo-executor-computer-use-linux/README.md`
  — narrowly scoped: add Wayland section + supported
  compositor matrix.
- `crates/argo-executor-computer-use-linux/src/wayland/wayland_tests.rs`
  — NEW. ≥ 8 tests (probe matrix + screenshot stub +
  input stub + portal mock). ≤ 350 LOC.

## Files you must NOT touch

- `argo-executor-computer-use-macos`,
  `argo-executor-computer-use-windows` — separate
  crates.
- `argo-executor-computer-use` (E2B) — separate.
- `argo-runtime/src/handlers/worker_actions_executor_cu_linux.rs`
  — locked from the Linux dispatch brief (sibling
  to G1, not this brief).

## Acceptance criteria

- [ ] Crate compiles cleanly on Linux (Wayland +
      X11) and on macOS/Windows (cross-compile
      with all paths probing Unsupported).
- [ ] Probe matrix:
      - `WAYLAND_DISPLAY` set + portal D-Bus
        reachable + `wtype` on PATH → returns
        `Supported(WaylandMode)`.
      - `WAYLAND_DISPLAY` set + `grim` + `wtype`
        but no portal → `Supported(WaylandMode)`
        with limited mouse (screenshot via grim;
        input may degrade per compositor).
      - `WAYLAND_DISPLAY` unset + `$DISPLAY` +
        `xdotool` + `scrot` → `Supported(X11Mode)`
        (existing path).
      - None of the above → `Unsupported(reason)`.
- [ ] Screenshot via xdg-desktop-portal returns PNG
      bytes; fall back to `grim` if portal missing.
- [ ] Typing via `wtype` on Wayland — verified by
      integration test (Wayland integration tests
      gated on `ARGO_WAYLAND_INTEGRATION=1` env
      var).
- [ ] Mouse click via portal RemoteDesktop —
      verified by integration test where portal
      supports it.
- [ ] When mouse click is unsupported (no portal +
      no compositor extension): returns a clear
      `Capability::UnsupportedExecutor` with
      compositor name + suggestion ("use
      GNOME/KDE/sway/Hyprland for full input
      support").
- [ ] X11 path remains byte-identical to current
      behavior (verified by snapshot tests on
      shared probe output).
- [ ] 8 unit tests pass.
- [ ] `cargo build -p argo-executor-computer-use-linux`
      clean on all 3 OSes.
- [ ] `cargo clippy -p argo-executor-computer-use-linux
      --all-targets -- -D warnings` clean.
- [ ] Every file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-G2-cu-native-linux-wayland`
- PR title: `executor(cu-linux): Wayland support via portal + wtype (G2)`

## Notes for the implementer

- **xdg-desktop-portal D-Bus interface** is the
  canonical cross-compositor API. Use `zbus` (pure-
  Rust D-Bus); preferred over `dbus` (C deps).
  Portal interfaces used:
  - `org.freedesktop.portal.Screenshot` →
    `Screenshot()` returns a URI to a PNG file.
  - `org.freedesktop.portal.RemoteDesktop` →
    `CreateSession` + `SelectDevices` + `Start`
    + `NotifyKeyboardKeycode` /
    `NotifyPointerButton`.

- **Portal consent flow.** First call to
  `RemoteDesktop` prompts the user to select
  outputs + devices. The consent is per-session
  (kept for the duration of the user's login
  session). v1 calls `CreateSession` lazily on
  first input attempt + caches the session handle.

- **`wtype` for typing.** Simpler subprocess
  approach for text typing on Wayland. Fast (no
  portal round-trip). Doesn't require portal
  RemoteDesktop. Pure CLI:
  ```
  wtype "Hello world"
  wtype -k Return
  ```

- **`grim` for screenshots.** Only used as fallback
  when portal isn't available (Sway/Hyprland
  compositors). Standard CLI:
  ```
  grim screenshot.png
  ```
  Set `-o <output>` for specific monitor;
  fullscreen otherwise.

- **Compositor detection.** Read `XDG_CURRENT_DESKTOP`
  + `XDG_SESSION_TYPE`. Map to compositor name +
  known portal support. Document the support matrix
  in the README:
  ```
  GNOME ≥ 42:        portal full
  KDE Plasma ≥ 5.27: portal full
  Sway ≥ 1.7:        grim + wtype (no portal)
  Hyprland:          grim + wtype (no portal)
  Other:             probe Unsupported
  ```

- **`#[cfg(target_os = "linux")]`-gated Wayland
  modules.** All Wayland code is Linux-only.
  Non-Linux compiles cross-platform clean by
  conditional compilation.

- **Integration tests** gated on env var:
  - `ARGO_WAYLAND_INTEGRATION=1` → runs against a
    real Wayland session (typically on dev
    machine).
  - CI runs only the unit tests (no Wayland
    available).

- **What this brief does NOT do.**
  - Dispatch wiring (mirrors G1 for Windows;
    Linux dispatch is sibling to G1 for Track G,
    not this brief).
  - GNOME/KDE-specific extensions beyond the
    standard portal API.
  - libei integration (named in the hardening
    table for v1.5).

## Hardening briefs implied (Track H)

NOT blocking this PR:

- `mission-enterprise-wayland-libei` — migrate to
  `libei` (input emulation) protocol once stable
  + has broad compositor support.
