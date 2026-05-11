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

pub mod action_suggest;
pub mod archetype;
pub mod broad_role;
pub mod charter_dialogue;
pub mod conflict;
pub mod error;
pub mod eval_seed;
pub mod genesis_judge;
pub mod job_description;
pub mod manifest_writer;
pub mod memory_schema;
pub mod pipeline;
pub mod pipeline_seed;
pub mod prompt_engineer;
pub mod prompt_writer;
pub mod quality;
pub mod refine;
pub mod safety_judge;
pub mod templates;
pub mod worker_config;

pub use archetype::{
    classify_charter, classify_llm, Archetype, ClassificationResult, ProviderClient,
};
pub use broad_role::{
    build_broad_role_prompt, build_confidence_classifier_prompt, classify_llm_with_confidence,
    genesis_broad_role, plan_or_broad, should_use_broad_role, BroadRoleClient, BroadRoleDraft,
    GenesisOutcome, BROAD_ROLE_CONFIDENCE_THRESHOLD, BROAD_SENTINEL,
};
pub use genesis_judge::{
    build_judge_prompt, judge_genesis_draft, GenesisJudgeClient, JudgeInput, JudgeVerdict,
};
pub use charter_dialogue::{
    dialogue_to_charter, name_to_persona_slug, CharterDialogue, DialogueError, Question, QUESTIONS,
};
pub use conflict::{detect as detect_conflicts, highest_score, ConflictMatch, ConflictReport};
pub use error::GenesisError;
pub use job_description::{genesis_from_job_description, GenesisDraft, JobDescInput};
pub use pipeline::{
    plan, plan_from_seed_vertical, plan_from_seed_vertical_in_default_root, plan_with,
    plan_with_quality, seeded_from, write_vertical, write_vertical_with_overlap, GenesisConfig,
    GenesisPlan, SEEDED_FROM_TAG_PREFIX,
};
pub use quality::{
    check_plan, require_pass, CheckResult, QualityReport, ACTION_MIN, EVAL_SEED_MIN,
    MEMORY_LAYER_MIN, PROMPT_WORD_MIN,
};
pub use refine::{
    refine_plan, refine_plan_with_cap, AnthropicCaller, RefineConfig, RefinementResponse,
};
pub use safety_judge::{judge as run_safety_judge, SENSITIVE_VERBS};
pub use templates::{
    load_all as load_templates, match_template, merge_with_overrides, GeneratedVertical,
    TemplateMeta, TemplateSeed, VerticalTemplate,
};
pub use worker_config::{WorkerConfig, DRY_RUN_HOURS};
