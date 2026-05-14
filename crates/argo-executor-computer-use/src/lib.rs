//! `argo-executor-computer-use` — Stream B `#B-computer-use-executor`.
//!
//! Computer-use executor for the Worker v3 dispatch trait
//! defined in Stream A `#A-executor-trait`. Drives a Firecracker
//! microVM (via the E2B managed service) running a desktop image,
//! and runs the Anthropic computer-use API in a loop: send
//! screenshots + previous actions, receive next actions (click /
//! type / scroll / key), execute them in the sandbox, capture a
//! fresh screenshot, repeat until the model emits a `stop_reason`
//! signal or the per-action budget is exhausted.
//!
//! On success the executor auto-completes the action with the
//! captured `result_json` (final URL, confirmation text, action
//! trajectory, screenshot path, tokens used). On budget exhaustion
//! or sandbox failure it falls back to a `HumanHandoff` so the
//! operator can finish manually — never silently drops work.
//!
//! ## Crate layout
//!
//! - [`profile`] — `Profile` schema + the `marketplace_browser`
//!   profile that ships in v1
//! - [`e2b`] — E2B HTTP client (sandbox lifecycle + computer
//!   primitives)
//! - [`anthropic`] — Anthropic computer-use loop driver
//! - [`budget`] — Token + wallclock budget tracking
//! - [`pool`] — VM pool keyed by `(tenant, persona, profile)` with
//!   10-min idle TTL
//! - [`audit`] — `browser.jsonl` writer + per-step screenshot
//!   capture
//! - [`config`] — environment-driven configuration
//! - [`executor`] — public `ComputerUseExecutor` orchestrator
//!
//! ## Quick example
//!
//! ```no_run
//! use argo_executor_computer_use::{ComputerUseExecutor, RunRequest};
//! # async fn demo() -> Result<(), argo_executor_computer_use::CuError> {
//! let exec = ComputerUseExecutor::from_env()?;
//! // Caller fills in tenant / profile / instructions / budget;
//! // see `RunRequest` for the full shape.
//! let req = RunRequest::demo("act-1", "marketplace_browser");
//! let outcome = exec.run(req).await?;
//! match outcome {
//!     argo_executor_computer_use::Outcome::Completed { result, .. } => {
//!         println!("CU loop completed: {result:?}");
//!     }
//!     argo_executor_computer_use::Outcome::HumanHandoff { reason, .. } => {
//!         eprintln!("CU loop handed off: {reason}");
//!     }
//! }
//! # Ok(())
//! # }
//! ```
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

pub mod anthropic;
mod anthropic_parse;
pub mod audit;
pub mod budget;
pub mod config;
pub mod e2b;
pub mod e2b_reap;
pub mod e2b_reap_cron;
pub mod executor;
pub(crate) mod executor_loop_exit;
pub(crate) mod executor_step_helpers;
pub mod executor_types;
pub mod learn;
pub mod pool;
pub mod profile;
pub mod restart;
pub mod sandbox_prewarm;
pub mod screenshot_persist;
pub mod success_metric;
#[cfg(any(test, feature = "test-harness"))]
pub mod testing;
#[cfg(any(test, feature = "test-harness"))]
pub mod testing_mocks;

pub use config::CuConfig;
pub use e2b_reap::{E2bReapReport, E2bSandboxInfo};
pub use error::CuError;
pub use executor::{ComputerUseExecutor, Outcome, RunRequest, RunResult};
pub use profile::{Profile, ProfileRegistry};

mod error;
