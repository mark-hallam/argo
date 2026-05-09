//! Native Linux computer-use proof of concept.
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

pub mod error;
pub mod input;
pub mod probe;
pub mod screenshot;
pub mod session;
#[cfg(all(test, target_os = "linux"))]
pub(crate) mod test_support;
pub mod window;

pub use error::LinuxCuError;
pub use input::InputResult;
pub use probe::{probe, ProbeReport, ProbeStatus};
pub use screenshot::Screenshot;
pub use session::{Session, SessionOptions, StepLog, StepRecord};
pub use window::WindowInfo;

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
        screenshot::capture(dest)
    }

    pub fn click(&self, x: i32, y: i32) -> LinuxCuResult<InputResult> {
        input::click(x, y)
    }

    pub fn key(&self, keysym: &str) -> LinuxCuResult<InputResult> {
        input::key(keysym)
    }

    pub fn r#type(&self, text: &str) -> LinuxCuResult<InputResult> {
        input::type_text(text)
    }

    pub fn type_text(&self, text: &str) -> LinuxCuResult<InputResult> {
        self.r#type(text)
    }

    pub fn windows(&self, pattern: &str) -> LinuxCuResult<Vec<WindowInfo>> {
        window::search_by_name(pattern)
    }

    pub fn focus(&self, window_id: u64) -> LinuxCuResult<()> {
        window::focus(window_id)
    }

    #[must_use]
    pub fn session(&self, opts: SessionOptions) -> Session {
        Session::new(*self, opts)
    }
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
