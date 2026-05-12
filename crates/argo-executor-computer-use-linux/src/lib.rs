//! Native Linux computer-use proof of concept.
//!
//! Two display-server backends live behind one [`LinuxComputerUse`]
//! facade: [`x11`] (legacy, `xdotool` + `scrot`) and [`wayland`]
//! (`wtype` + `grim` + xdg-desktop-portal). [`probe`] auto-selects
//! the backend by inspecting `WAYLAND_DISPLAY`, `DISPLAY`, and tool
//! availability on `PATH`.
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

pub mod error;
pub mod probe;
pub mod session;
pub mod wayland;
pub mod x11;
#[cfg(all(test, target_os = "linux"))]
pub(crate) mod test_support;

pub use error::LinuxCuError;
pub use probe::{
    probe, DisplayMode, ProbeReport, ProbeStatus, WaylandCapabilities,
};
pub use session::{Session, SessionOptions, StepLog, StepRecord};
pub use x11::input::InputResult;
pub use x11::screenshot::Screenshot;
pub use x11::window::WindowInfo;

// Compat re-exports — pre-Wayland callers (session.rs tests, downstream
// snapshot fixtures) reference the X11 module paths at the crate root.
pub use x11::input;
pub use x11::screenshot;
pub use x11::window;

pub type LinuxCuResult<T> = Result<T, LinuxCuError>;

#[derive(Debug, Clone, Copy, Default)]
pub struct LinuxComputerUse;

impl LinuxComputerUse {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    #[must_use]
    pub fn probe(&self) -> ProbeReport {
        probe::probe()
    }

    pub fn screenshot(&self, dest: &std::path::Path) -> LinuxCuResult<Screenshot> {
        match probe::probe().display_mode {
            DisplayMode::Wayland(_) => wayland::screenshot::capture(dest),
            _ => x11::screenshot::capture(dest),
        }
    }

    pub fn click(&self, x: i32, y: i32) -> LinuxCuResult<InputResult> {
        match probe::probe().display_mode {
            DisplayMode::Wayland(ref caps) => wayland::input::click(x, y, caps),
            _ => x11::input::click(x, y),
        }
    }

    pub fn key(&self, keysym: &str) -> LinuxCuResult<InputResult> {
        match probe::probe().display_mode {
            DisplayMode::Wayland(_) => wayland::input::key(keysym),
            _ => x11::input::key(keysym),
        }
    }

    pub fn r#type(&self, text: &str) -> LinuxCuResult<InputResult> {
        match probe::probe().display_mode {
            DisplayMode::Wayland(_) => wayland::input::type_text(text),
            _ => x11::input::type_text(text),
        }
    }

    pub fn type_text(&self, text: &str) -> LinuxCuResult<InputResult> {
        self.r#type(text)
    }

    pub fn windows(&self, pattern: &str) -> LinuxCuResult<Vec<WindowInfo>> {
        x11::window::search_by_name(pattern)
    }

    pub fn focus(&self, window_id: u64) -> LinuxCuResult<()> {
        x11::window::focus(window_id)
    }

    #[must_use]
    pub fn session(&self, opts: SessionOptions) -> Session {
        Session::new(*self, opts)
    }
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
