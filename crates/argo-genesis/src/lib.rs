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
pub mod archetype_meta;
pub mod author;
pub mod broad_role;
pub mod charter;
pub mod charter_dialogue;
pub mod comprehension;
pub mod conflict;
pub mod error;
pub mod eval_seed;
pub mod genesis_judge;
pub mod hire_gate;
pub mod hire_gate_contract;
pub mod hire_gate_disk;
pub mod hire_gate_rubric;
pub mod intake_llm;
pub mod intake_machine;
pub mod intake_store;
pub mod job_description;
pub mod manifest_writer;
pub mod materialize;
mod materialize_naming;
pub mod materialize_writer;
pub mod memory_schema;
pub mod onboarding_documents;
pub mod pipeline;
pub mod pipeline_seed;
pub mod policy_synth;
pub mod prompt_engineer;
pub mod prompt_writer;
pub mod quality;
pub mod refine;
pub mod role_contract;
pub mod rubric;
pub mod safe_fetch;
pub mod safety_judge;
pub mod sandbox_session;
pub mod sop;
pub mod sop_store;
pub mod spec;
pub mod synthesizer;
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
pub use charter::{
    CharterValidationError, WorkerCharter, MAX_RESPONSIBILITIES, MAX_TOOLS_HINT,
    MIN_RESPONSIBILITIES,
};
pub use charter_dialogue::{
    dialogue_to_charter, name_to_persona_slug, CharterDialogue, DialogueError, Question, QUESTIONS,
};
pub use comprehension::{
    build_comprehension_questions, ComprehensionOutcome, ComprehensionQuestion, APPROVAL_KEYWORD,
    BLOCKED_KEYWORD, ESCALATE_KEYWORD, EVIDENCE_KEYWORD,
};
pub use conflict::{detect as detect_conflicts, highest_score, ConflictMatch, ConflictReport};
pub use error::GenesisError;
pub use genesis_judge::{
    build_judge_prompt, judge_genesis_draft, GenesisJudgeClient, GenesisJudgeVerdict, JudgeInput,
};
pub use hire_gate::{
    behaviour_contract_text, build_gate_eval_inputs, DeterministicEvalInput, EvalFloorOutcome,
    GateEvalInputs, GateState, GateVerdict, SafetySignal, EVAL_PASS_RATE_FLOOR,
};
pub use hire_gate_contract::{ContractConfirmation, ContractGateInput};
pub use hire_gate_rubric::judge_input_for_plan;
pub use hire_gate_disk::{gate_inputs_from_dir, DiskGateInputs};
pub use intake_llm::{
    build_extract_prompt, build_question_prompt, extract_delta, propose_question, IntakeClient,
};
pub use intake_machine::{
    IntakeSlot, IntakeState, IntakeTurn, NextStep, SlotDelta, Speaker,
};
pub use intake_store::IntakeStore;
pub use role_contract::{verify_contract_matches, ContractAction, RoleContract};
pub use author::{apply_authored, author_teammate, build_author_prompt, AuthorClient, AuthoredTeammate};
pub use rubric::{
    build_rubric_prompt, grade_rubric, RubricClient, RubricDimension, RubricOutcome, RubricVerdict,
    DIMENSION_FLOOR, RUBRIC_PASS_FLOOR,
};
pub use sop::{
    build_sop_quiz, extract_sop_pdf_text, ingest_sop, SopComprehensionOutcome, SopKnowledge,
    SopQuestion, MIN_SOP_CHARS,
};
pub use sandbox_session::{
    answer_quiz_question, suggested_quiz_questions, QuizAnswer, QuizCitation, NO_KNOWLEDGE_ANSWER,
};
pub use sop_store::{read_sop_knowledge, write_sop_knowledge, DiskSop};
pub use onboarding_documents::{
    ingest_onboarding_document, ingest_onboarding_document_with_kind, onboarding_document_labels,
    read_onboarding_documents, write_onboarding_documents, OnboardingDocument, SourceKind,
    MAX_FILES, MAX_FILE_BYTES, MAX_TOTAL_BYTES,
};
pub use safe_fetch::{safe_fetch_text, SAFE_FETCH_BODY_CAP_BYTES};
pub use policy_synth::{
    boundaries_echo, gated_class_names, normalize_topic, persist_policy, synthesize_policy,
    smart_defaults as boundaries_smart_defaults, AccessLevel, AccessScope, BoundariesSpec,
    DataCategory, BOUNDARIES_POLICY_FILENAME, GENESIS_BOUNDARIES_POLICY_ID,
};
pub use job_description::{
    genesis_from_job_description, genesis_from_job_description_with, GenesisDraft, JobDescInput,
};
pub use materialize::{materialize_spec, write_capability_vertical};
pub use materialize_writer::{persona_prompt_from_spec, system_prompt_from_spec};
pub use spec::{
    CapabilityDecl, CapabilitySpec, EscalationSpec, ExecutorIntent, SuccessCriterion,
};
pub use pipeline::{
    plan, plan_from_seed_vertical, plan_from_seed_vertical_in_default_root, plan_with,
    plan_with_quality, seeded_from, write_vertical, write_vertical_with_overlap,
    write_vertical_with_overwrite, GenesisConfig, GenesisPlan, SEEDED_FROM_TAG_PREFIX,
};
pub use quality::{
    check_plan, require_pass, CheckResult, QualityReport, ACTION_MIN, EVAL_SEED_MIN,
    MEMORY_LAYER_MIN, PROMPT_WORD_MIN,
};
pub use refine::{
    refine_plan, refine_plan_with_cap, AnthropicCaller, RefineConfig, RefinementResponse,
};
pub use safety_judge::{judge as run_safety_judge, SENSITIVE_VERBS};
pub use synthesizer::{
    preprocess as preprocess_jd, synthesize_or_stub, LlmSynthesizer, StubSynthesizer, Synthesizer,
    MAX_PROMPT_INPUT_BYTES,
};
pub use templates::{
    load_all as load_templates, match_template, merge_with_overrides, GeneratedVertical,
    TemplateMeta, TemplateSeed, VerticalTemplate,
};
pub use worker_config::{WorkerConfig, DRY_RUN_HOURS};
