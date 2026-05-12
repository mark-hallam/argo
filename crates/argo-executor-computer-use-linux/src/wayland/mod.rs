//! Wayland backend for native Linux computer-use.
//!
//! Three primitives:
//! - [`probe`]: detect compositor, portal D-Bus reachability, and
//!   `wtype`/`grim` presence on `PATH`.
//! - [`screenshot`]: capture via xdg-desktop-portal (cross-compositor)
//!   with `grim` fallback on Sway/Hyprland.
//! - [`input`]: typing via `wtype`; mouse click via portal
//!   RemoteDesktop (where available) or fail-closed.
//!
//! All D-Bus code is gated under `#[cfg(target_os = "linux")]` in
//! [`portal`]. Non-Linux targets compile cleanly: probes return the
//! `Unsupported(...)` branch and primitives short-circuit before any
//! D-Bus or subprocess call is made.

pub mod input;
pub mod portal;
pub mod probe;
pub mod screenshot;

#[cfg(test)]
#[path = "wayland_tests.rs"]
mod tests;
