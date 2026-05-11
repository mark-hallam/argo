//! Native Windows computer-use proof of concept.
//!
//! Subprocess-only: every primitive shells out to `powershell.exe`
//! with `-NoProfile -NonInteractive -Command` plus an inline script.
//! No new workspace deps; mirrors the macOS (#941) and Linux POCs.
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

pub mod error;
pub mod input;
pub mod probe;
pub mod screenshot;
pub mod session;
pub mod window;

pub use error::WindowsCuError;
pub use input::InputResult;
pub use probe::{probe, ProbeReport, ProbeStatus};
pub use screenshot::Screenshot;
pub use session::{Session, SessionOptions, StepLog, StepRecord};
pub use window::WindowInfo;

pub type WindowsCuResult<T> = Result<T, WindowsCuError>;

/// Native Windows computer-use facade.
///
/// All methods route through `powershell.exe` subprocess wrappers.
/// On non-Windows hosts each method returns
/// [`WindowsCuError::Unsupported`] via the runtime probe.
#[derive(Debug, Clone, Copy, Default)]
pub struct WindowsComputerUse;

impl WindowsComputerUse {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    #[must_use]
    pub fn probe(&self) -> ProbeReport {
        probe::probe()
    }

    pub fn screenshot(&self, dest: &std::path::Path) -> WindowsCuResult<Screenshot> {
        screenshot::capture(dest)
    }

    pub fn click(&self, x: i32, y: i32) -> WindowsCuResult<InputResult> {
        input::click(x, y)
    }

    pub fn key(&self, vk: &str) -> WindowsCuResult<InputResult> {
        input::key(vk)
    }

    pub fn r#type(&self, text: &str) -> WindowsCuResult<InputResult> {
        input::type_text(text)
    }

    pub fn type_text(&self, text: &str) -> WindowsCuResult<InputResult> {
        self.r#type(text)
    }

    pub fn windows(&self, pattern: &str) -> WindowsCuResult<Vec<WindowInfo>> {
        window::search_by_name(pattern)
    }

    pub fn focus(&self, window_handle: i64) -> WindowsCuResult<()> {
        window::focus(window_handle)
    }

    #[must_use]
    pub fn session(&self, opts: SessionOptions) -> Session {
        Session::new(*self, opts)
    }
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
