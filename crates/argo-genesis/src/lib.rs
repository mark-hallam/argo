//! `argo-genesis` — Stream G `#G-genesis`.
//!
//! Charter → vertical generator. Operator describes a role in
//! plain English; Genesis produces a complete `verticals/<id>/`
//! tree with safety guarantees on every generated action.
//!
//! Module layout (per the brief's pipeline shape):
//!
//! - [`archetype`] — 12-archetype taxonomy + offline classifier.
//! - [`manifest_writer`] — assembles `vertical.toml` from
//!   archetype + persona + charter; auto-fixes sensitive
//!   actions to require approval.
//! - [`prompt_writer`] — system prompt, persona prompt, eval
//!   seed, memory schema.
//! - [`worker_config`] — `worker.toml` with `dry_run = true`
//!   for the first 24h.
//! - [`safety_judge`] — non-negotiable hard rule: any action
//!   matching a sensitive verb MUST be `requires_approval`.
//! - [`pipeline`] — top-level orchestrator (`plan` +
//!   `write_vertical`).
//!
//! ## Quick example
//!
//! ```no_run
//! # fn demo() -> Result<(), argo_genesis::GenesisError> {
//! let plan = argo_genesis::plan(
//!     "be my marketing intern, run weekly campaigns to cold leads",
//!     "maya",
//! )?;
//! let dir = tempfile::tempdir().expect("tempdir");
//! let out = dir.path().join(&plan.vertical_id);
//! argo_genesis::write_vertical(&plan, &out)?;
//! # Ok(())
//! # }
//! ```
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

pub mod archetype;
pub mod error;
pub mod manifest_writer;
pub mod pipeline;
pub mod prompt_writer;
pub mod refine;
pub mod safety_judge;
pub mod worker_config;

pub use archetype::{classify_charter, Archetype};
pub use error::GenesisError;
pub use pipeline::{plan, write_vertical, GenesisPlan};
pub use refine::{refine_plan, AnthropicCaller, RefineConfig, RefinementResponse};
pub use safety_judge::{judge as run_safety_judge, SENSITIVE_VERBS};
pub use worker_config::{WorkerConfig, DRY_RUN_HOURS};
