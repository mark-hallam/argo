//! `argo-vertical` — foundation for the vertical-template system
//! (#649). Argo's core platform stays vertical-agnostic; this crate
//! is the layer that lets the same binary be cloned into multiple
//! vertical-agent products (PR Checker, AI Receptionist, Sales
//! Follow-up, Ticket Triage, Tech Support, …) by manifest +
//! configuration only.
//!
//! ## Scope of this crate
//!
//! This crate is **purely declarative + read-only**. It owns:
//!
//! - the [`VerticalManifest`] schema (parsed from `vertical.toml`)
//! - the [`parse`] entry point + structural validation
//! - the [`validate`] entry point + cross-reference checks against
//!   on-disk files
//! - the [`Registry`] holding loaded verticals in memory
//! - the [`load_dir`] walker that populates a registry from a
//!   `verticals/` directory
//!
//! It does NOT own:
//!
//! - any vertical-specific behaviour (the strict invariant of the
//!   architecture — receptionist / support / triage / PR-checker /
//!   sales logic must live in the templates themselves, not here);
//! - CLI subcommands (Phase 2 — #650);
//! - gateway routes (Phase 2 — #650);
//! - SQLite migrations or session integration (Phase 3 — #651);
//! - workflow runtime (Phase 5 — #653);
//! - escalation enforcement (Phase 6 — #655).
//!
//! Those layers ship in subsequent PRs and depend on this crate's
//! public API. Keeping the dependency arrow one-way (this crate
//! depends on `argo-types` only) lets every higher layer evolve
//! independently.
//!
//! ## Quick example
//!
//! ```no_run
//! use argo_vertical::load_dir;
//!
//! let (registry, errors) = load_dir(std::path::Path::new("verticals"))
//!     .expect("verticals dir is a directory");
//! for err in &errors {
//!     eprintln!("vertical at {} failed to load: {}", err.dir.display(), err.error);
//! }
//! for (id, loaded) in registry.list() {
//!     println!("loaded vertical: {id} (v{})", loaded.manifest.vertical.version);
//! }
//! ```
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

mod eval_score;
mod eval_score_actuals;
mod eval_score_judge;
mod isolation;
mod loader;
mod manifest;
mod manifest_actions;
mod memory_schema;
mod registry;
mod validator;
mod workflow;

use std::path::PathBuf;

/// Env var that overrides the default verticals discovery
/// directory. Set by tests + by users who keep their verticals
/// outside the repo.
pub const ENV_VERTICALS_DIR: &str = "ARGO_VERTICALS_DIR";

/// Resolve the default verticals discovery directory.
/// `$ARGO_VERTICALS_DIR` wins; default is `./verticals`
/// (cwd-relative). Pure policy — does no I/O.
#[must_use]
pub fn default_dir() -> PathBuf {
    if let Ok(p) = std::env::var(ENV_VERTICALS_DIR) {
        if !p.trim().is_empty() {
            return PathBuf::from(p);
        }
    }
    PathBuf::from("verticals")
}

pub use eval_score::{
    score_fixture, score_suite_jsonl, FixtureScore, FixtureStatus, ScoreCheck, SuiteScore,
};
pub use eval_score_actuals::{
    parse_actuals_jsonl, score_suite_jsonl_with_actuals, ActualsMap, ActualsParseError,
};
pub use eval_score_judge::{
    build_judge_prompt, judge_fixture, parse_judge_response, parse_judge_spec, JudgePolicy,
    JudgeVerdict,
};
pub use isolation::check_isolation;
pub use loader::{load_dir, LoadError};
pub use manifest::{
    parse, ActionDeclaration, ActionRiskLevel, ActionsSection, EscalationSection,
    EvaluationSection, GatewaySection, MemorySection, ProviderSection, RetryPolicy, RoleSection,
    SkillsSection, SuccessMetricsSection, ToolsSection, TuiSection, VerticalIdentity,
    VerticalManifest, WorkflowsSection, SCHEMA_VERSION,
};
// Stream A (#A-executor-trait) executor-decl schema. Re-exported
// directly from `manifest_actions` since they're not consumed by
// `manifest.rs` itself.
pub use manifest_actions::{ExecutorDeclaration, ExecutorTagged};
pub use memory_schema::{
    parse_memory_schema, validate_memory_schema, MemoryEntry, MemorySchema, MEMORY_SCHEMA_VERSION,
};
pub use registry::{LoadedVertical, Registry};
pub use validator::validate;
pub use workflow::{
    parse_workflow, validate_workflow, Workflow, WorkflowHeader, WorkflowStep,
    WORKFLOW_SCHEMA_VERSION,
};
