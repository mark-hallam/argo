//! `argo-executor-computer-use-windows` — V1.5: native Windows
//! computer-use executor.
//!
//! Sibling of [`argo_executor_computer_use_macos`] — same 9-file
//! shape, same step semantics, same `RunRequest → Outcome` flow.
//! The Windows-specific surface area is:
//!
//! - **Accessibility**: `IUIAutomation` (COM) for control-tree
//!   introspection. [`a11y`]
//! - **Input injection**: `SendInput` (Win32) for mouse / keyboard
//!   events (absolute-coordinate mouse via `MOUSEEVENTF_ABSOLUTE`,
//!   normalised over the virtual screen). [`events`]
//! - **Screenshot**: `BitBlt` + `EnumDisplayMonitors` for
//!   per-monitor PNG capture. [`screenshot`]
//! - **Preflight**: UAC consent / secure-desktop / DPI awareness /
//!   UIAutomation service / monitor count probes. [`preflight`]
//!
//! ## Crate layout
//!
//! - [`config`] — `WindowsCuConfig` (allowlists + DPI + dwell)
//! - [`error`] — `WindowsCuError` (one variant per failure class)
//! - [`screenshot`] — multi-monitor `BitBlt → PNG` capture
//! - [`events`] — `SendInput` mouse / keyboard / scroll
//! - [`a11y`] — UIAutomation tree traversal helpers
//! - [`notes`] — Windows-platform caveats for operators
//! - [`preflight`] — six-axis preflight report
//! - [`executor`] — public `WindowsCuExecutor` orchestrator
//!
//! ## Quick example
//!
//! ```no_run
//! use argo_executor_computer_use_windows::{WindowsCuExecutor, WindowsCuConfig};
//! # async fn demo() -> Result<(), argo_executor_computer_use_windows::WindowsCuError> {
//! let config = WindowsCuConfig::for_notepad_app();
//! let exec = WindowsCuExecutor::new(config)?;
//! let report = exec.preflight().await;
//! if !report.all_passed() {
//!     eprintln!("preflight gaps:\n{}", report.actionable_summary());
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## COM runtime requirement
//!
//! UIAutomation is a COM API; consumers must initialise COM on the
//! calling thread via `CoInitializeEx(COINIT_MULTITHREADED)` before
//! invoking [`a11y`] helpers. The executor performs this
//! initialisation lazily inside [`WindowsCuExecutor::warm_up`].
//!
//! ## Platform support
//!
//! Compiles on every platform; the Windows-only modules return
//! [`WindowsCuError::NotWindows`] when called from a non-Windows
//! host. That keeps the workspace `cargo check` green on Linux /
//! macOS CI runners while the Windows-host integration test is
//! gated on `ARGO_NATIVE_WINDOWS_INTEGRATION=1`.
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

pub mod a11y;
pub mod config;
pub mod error;
pub mod events;
pub mod executor;
pub mod notes;
pub mod preflight;
pub mod screenshot;

pub use config::{ScreenshotKind, WindowsCuConfig};
pub use error::WindowsCuError;
pub use executor::{StepRecord, WindowsCuExecutor, WindowsCuOutcome, WindowsCuRunRequest};
pub use preflight::{PreflightCheck, PreflightStatus, WindowsPreflightReport};

/// Result alias for the crate.
pub type WindowsCuResult<T> = Result<T, WindowsCuError>;
