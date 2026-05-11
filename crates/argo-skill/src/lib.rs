//! `argo-skill` — content-addressed skill store with locks (§14.2).
//!
//! A skill is a `SKILL.md` file with a YAML frontmatter block followed
//! by a free-form Markdown body. Frontmatter declares `name`, `version`,
//! `description`, the `capabilities` the skill needs, `provenance`, and a
//! `lock` setting (Amendment 3).
//!
//! Locks gate writes via [`SkillStore::install`] and
//! [`SkillStore::supersede`]:
//!
//! - `Lock::None` — agent may freely edit.
//! - `Lock::UserProtected` — agent edits rejected with
//!   [`argo_types::ArgoError::UserProtected`]; only [`PrincipalOverride::Force`]
//!   may write through.
//! - `Lock::ContentLocked` — agent edits allowed but the new skill MUST
//!   reference the previous skill via `supersede` so the lineage hash
//!   chain stays intact.
//!
//! Spec: `docs/plan/argo-build-plan.md` §14.2.
#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

pub mod ab_test;
mod ab_test_stats;
mod artifact;
pub mod audit_reader;
pub mod canary;
pub mod conflicts;
pub mod deps;
pub mod dreams_distillation;
pub mod eval_runner;
mod eval_runner_gate;
pub mod feedback;
pub mod learn;
pub mod library;
pub mod marketplace;
pub mod publish;
mod parse;
pub mod quality;
pub mod regression;
pub mod sign;
mod skill;
mod store;
mod store_lookups;
mod store_vertical;
mod store_vertical_judge;
pub mod success_metric;
pub mod trust;
pub mod version;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_lifecycle;
#[cfg(test)]
mod sign_dep_smoke;
#[cfg(test)]
mod store_trust_tests;

pub use argo_judge::Judge;
pub use feedback::{should_propose_with_feedback, FeedbackVote};
pub use learn::{should_propose, SkillCandidate, DEFAULT_SUCCESS_THRESHOLD};
pub use success_metric::{
    should_propose_metric, ProposalThreshold, SuccessMetric,
};
pub use conflicts::{detect_conflicts, parse_keep_flag, render_diff, SkillConflict};
pub use deps::{
    build_graph, build_reverse_graph, extract_refs, DepEdge, DepGraph, RefKind, SkillRef,
    DEFAULT_MAX_DEPTH,
};
pub use dreams_distillation::{
    distill_cohort, group_into_cohorts, trajectory_similarity, Cohort,
    DistillationResult, SkillDistillationCaller, TrajectoryInput,
    BODY_SOURCE_DREAMS, COHORT_SIMILARITY_THRESHOLD, MIN_COHORT_SIZE,
};
pub use library::{LayeredLookup, LayeredSkill, SkillLayer};
pub use marketplace::{parse_trust_grade, search, MarketplaceSkill, SearchOptions, DEFAULT_SEARCH_LIMIT};
pub use parse::{parse_skill_md, FRONTMATTER_DELIM};
pub use publish::{
    list_mock_skill_tags, mock_registry_root, publish_mock_skill, unpublish_mock_skill_tag,
    SkillPublishRecord, SkillPublishSource, SkillTag, SkillUnpublishRecord,
};
pub use audit_reader::{AuditRow, AuditStoreReader, SessionStoreAuditReader};
pub use quality::{compute_score, QualityInputs, QualityScore, SkillQualityStore};
pub use regression::{detect_regression, RegressionConfig, RegressionDecision};
pub use eval_runner::{
    run_evals, ActualOutcome, EvalExecutor, EvalReport, DEFAULT_EVAL_PASS_RATE_THRESHOLD,
};
pub use skill::{
    EvalCase, ExpectedOutcome, LifecycleState, PrincipalOverride, Provenance, Skill,
    SkillFrontmatter,
};
pub use store::SkillStore;
pub use trust::{is_skill_eligible, min_trust_for_risk, TrustGrade};
pub use ab_test::{
    create_run, ensure_ab_schema, get_run, maybe_complete, maybe_complete_with_quality,
    promote, record_outcome, select_variant, significance, variant_counts, AbTestRun,
    QualityWeightConfig, RunStatus, Significance, Variant, VariantCounts, WinnerReason,
    CHI_SQUARE_CRIT_DF1_ALPHA_05, DEFAULT_MIN_SAMPLES_PER_VARIANT,
};
pub use artifact::{
    skill_body_sha256, SkillArtifact, SkillArtifactManifest, SkillArtifactMetadata,
    SKILL_ARTIFACT_BODY, SKILL_ARTIFACT_BODY_LEGACY, SKILL_ARTIFACT_MANIFEST,
    SKILL_ARTIFACT_METADATA, SKILL_ARTIFACT_METADATA_FORMAT, SKILL_ARTIFACT_SCHEMA_VERSION,
    SKILL_ARTIFACT_SHA256,
};
pub use canary::{
    hash_seed, is_canary, load_body as load_canary_body, new_config as new_canary_config,
    pick_for_skill as pick_canary_for_skill, pick_version, seed_for_session_skill, CanaryConfig,
    CanaryDecision, CanaryStore,
};
pub use version::{find_version, list_versions, SkillVersion};
