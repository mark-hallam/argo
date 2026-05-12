//! X11 primitives: subprocess wrappers around `xdotool` and `scrot`.
//!
//! Module surface preserved byte-identical from the pre-Wayland POC.
//! The `wayland/` sibling layers the alternative display-server path;
//! [`crate::probe`] dispatches between them.

pub mod input;
pub mod screenshot;
pub mod window;
