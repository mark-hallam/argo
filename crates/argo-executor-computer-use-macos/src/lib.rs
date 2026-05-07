//! `argo-executor-computer-use-macos` — V1.5 PoC: native macOS
//! computer-use executor.
//!
//! Drives macOS Notes via `screencapture` (PNG capture of the
//! front window) + `osascript` (input injection via `System
//! Events`) + the Anthropic computer-use loop. Sibling to the
//! E2B-based [`argo_executor_computer_use`] — both implement the
//! same step shape and reuse the same `anthropic.rs` client.
//!
//! See [`docs/skycot/COMPUTER_USE_NATIVE_MACOS.md`] in the repo
//! for the full architectural rationale, the six open questions,
//! and the V1.5 vs V2.x split.
//!
//! ## Crate layout
//!
//! - [`config`] — `MacosCuConfig` (allowed apps + actions, dwell,
//!   anthropic settings)
//! - [`error`] — `MacosCuError` variants (one per failure mode)
//! - [`screenshot`] — front-window PNG capture via `screencapture`
//! - [`events`] — input injection via `osascript`
//! - [`a11y`] — accessibility / app-control helpers (osascript;
//!   the AXUIElement integration ships in a follow-up brief)
//! - [`notes`] — Notes-specific helpers (open Notes, ensure fresh
//!   note, read note body)
//! - [`preflight`] — permission-check report
//! - [`executor`] — public `MacosCuExecutor` orchestrator
//!
//! ## Quick example
//!
//! ```no_run
//! use argo_executor_computer_use_macos::{MacosCuExecutor, MacosCuConfig};
//! # async fn demo() -> Result<(), argo_executor_computer_use_macos::MacosCuError> {
//! let config = MacosCuConfig::for_notes_app();
//! let exec = MacosCuExecutor::new(config)?;
//! let report = exec.preflight().await;
//! if !report.all_granted() {
//!     eprintln!("missing permissions:\n{}", report.actionable_summary());
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Platform support
//!
//! Compiles on every platform; the macOS-only modules (screencapture,
//! osascript subprocess invocations) return
//! [`MacosCuError::NotMacos`] when called from a non-macOS host.
//! That keeps the workspace `cargo check` green on Linux CI runners
//! while the runtime smoke test is gated on
//! `ARGO_NATIVE_MACOS_INTEGRATION=1`.
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

pub mod a11y;
pub mod config;
pub mod error;
pub mod events;
pub mod executor;
pub mod notes;
pub mod preflight;
pub mod screenshot;

pub use config::MacosCuConfig;
pub use error::MacosCuError;
pub use executor::{MacosCuExecutor, MacosCuOutcome, MacosCuRunRequest};
pub use preflight::{PermissionKind, PermissionReport, PermissionStatus};

/// Result alias for the crate.
pub type MacosCuResult<T> = Result<T, MacosCuError>;
