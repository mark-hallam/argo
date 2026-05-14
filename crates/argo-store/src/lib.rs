//! `argo-store` — SQLite + FTS5 session store with up/down migrations.
#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

pub mod anomaly;
mod anomaly_persistence;
/// Generic write coalescer used by the message + audit hot paths
/// (Stream `claude-perf-store-batched-writes`). Each per-row insert
/// becomes a `submit()` into the writer; many concurrent submits
/// share one `BEGIN`/`COMMIT` to amortize WAL fsync.
pub mod batched_writer;
/// First-class approval requests for the auto-approval gate
/// (Stream `claude-approval-request-store`). Records pending grants,
/// who can decide them, and a deadline + `default_decision` the
/// downstream timeout sweep promotes stale rows by.
pub mod approvals;
/// Typed scaffold for blob storage — `AttachedBlob`, `BlobRef`,
/// `BlobStore` trait, and an in-memory test impl. No persistence yet
/// (Stream `claude-store-attached-blob-layer`); downstream
/// `claude-cu-screenshot-storage` lands the SQLite-backed impl.
pub mod attached_blob;
/// Budget rollups over skills and spend.
pub mod budget;
/// Data classification tags (Stream `claude-data-classification`).
pub mod classification;
/// Fleet-wide skill-cohort aggregation (Stream
/// `mission-skill-cohort-ui`, MISSION_BACKLOG #198). Read-only
/// helpers powering the `/cohorts` route — groups workers by
/// `(vertical, trust_grade_band)` and surfaces member + skill
/// coverage rolls-up over the `worker`/`worker_skill` tables.
pub mod cohort;
/// Operator feedback (Stream `claude-operator-feedback-channel`).
pub mod feedback;
mod forge_meta;
/// GDPR erasure helpers.
pub mod gdpr;
/// Gateway request audit log query helpers.
pub mod gateway_request_audit;
/// Aggregate query helpers over the `handoff` table for the
/// `argo handoff-stats` CLI surface (Stream `codex-handoff-stats-cli`).
pub mod handoff_stats_query;
mod gdpr_apply;
mod grants;
mod marketplace_meta;
mod message_query;
mod migrations;
mod models;
mod session_meta;
mod session_query;
/// Chronological skill invocation history query helpers.
pub mod skill_runs_query;
mod skill_runs_query_decode;
/// Skill discovery query helpers — vertical-scoped search and
/// session-attachment indirection (Stream
/// `claude-skill-search-by-vertical`).
pub mod skill_query;
/// Cross-tenant skill sharing query helpers (Stream
/// `claude-skill-share-cross-tenant-poc`, mission claim 4).
pub mod skill_share;
/// Operator-to-publisher skill-rating store (Stream
/// `mission-skill-ratings-publisher`, mission claim 4 #80) — one row
/// per `(skill_id, skill_version, rater_tenant)`, idempotent on the
/// composite primary key.
pub mod skill_rating;
/// Per-skill operator flags (Stream
/// `mission-forge-operator-feedback-eval-signal`, MISSION_BACKLOG
/// Section 3 Claim 4 #85). Generic flag store consulted by the
/// Forge ingestion path before pulling operator feedback as eval
/// cases — `no-forge-feedback` toggles ingestion off for noisy
/// skills.
pub mod skill_flag;
mod spend;
/// Spend anomaly detector — Welford rolling stats per (worker,
/// hour-of-day) + z-score check (Stream
/// `claude-spend-anomaly-alerts`).
pub mod spend_anomaly;
/// Hard `[budget.ceiling]` enforcement read side —
/// `CeilingConfig`, `CeilingCheck`, and the
/// `SessionStore::check_ceiling` query that feeds the
/// `argo-provider::spend_gate::SpendGate` enforcement-time wrapper
/// (Stream `mission-llm-budget-hard-ceiling`, MISSION_BACKLOG
/// Section 7 #225).
pub mod spend_ceiling;
mod stats;
/// Per-executor-kind action latency histogram — power-of-2 ms
/// buckets, p50/p95/p99 readout (Stream
/// `claude-latency-percentiles-tracking`).
pub mod latency_tracker;
/// Persistent learned-plan-template store — Dreams-distilled
/// workflow YAML body + TrustGrade ladder (Stream
/// `claude-dreams-plan-templates`).
pub mod learned_plan_template;
/// Typed query helpers backed by lazy raw-SQL schemas
/// (Stream `codex-mcp-server-registry-cli`).
pub mod queries;
/// Per-worker persona revision log — append-only diff history with
/// sha256 audit-chain hashes + one-step revert (Stream
/// `mission-persona-evolution-audit-revert`, MISSION_BACKLOG Section
/// 3 Claim 1 #46).
pub mod persona_revision;
/// Recovery-cache CRUD helpers — `argo recover cache list/show/clear`.
pub mod recovery_cache;
/// Persistent subtask ledger — durable record of Planner-emitted
/// decompositions (Stream `claude-subtask-ledger-store`).
pub mod subtask;
/// Per-subtask budget tree — declared cap, accumulated spend, and a
/// recursive walk_tree CTE that rolls aggregated spend up to the
/// root. New sibling functions only; the original `subtask` module
/// is unchanged (Stream `mission-delegation-budget-tree`,
/// MISSION_BACKLOG Section 3 Claim 2 #57).
pub mod subtask_budget;
/// Operator-facing team primitive — bridges `org_unit` + `roster`
/// + `roster_member` for the `argo team` CLI surface (Stream
/// `claude-worker-team-roster-cli`).
pub mod team_roster;
/// Mutation surface for the team-roster primitive (split from
/// `team_roster.rs` for the 500-LOC budget). Public symbols are
/// re-exported from `team_roster` so external imports remain
/// `argo_store::team_roster::create_team`.
pub mod team_roster_mutations;
/// Elevated-view grant store (Stream J16) — per-principal,
/// per-category, time-bounded "show me the raw value" override
/// consulted by the J2 redact-policy resolver. Rows are inserted by
/// the `POST /api/v1/audit/elevated-view-request` handler and revoked
/// by `DELETE /api/v1/audit/elevated-view-grants/<id>`.
pub mod elevated_view_grant;
/// SCIM 2.0 user provisioning store
/// (Mission `mission-scim-provisioning`, MISSION_BACKLOG Section 7
/// #231). Backs the `/scim/v2/Users` endpoints exposed by argo-cli's
/// gateway; soft-delete writes a row in
/// `scim_provisioned_tombstones` (same transaction) so the audit
/// chain can still resolve tenant + `userName` for a deprovisioned
/// user.
pub mod scim_user;
/// At-rest encryption key resolution + `PRAGMA key`/`rekey` helpers
/// (Stream `mission-encryption-at-rest-sqlcipher`).
pub mod encryption;
/// Cloud-KMS parser + client-build glue split out from
/// [`crate::encryption`] under the 500-LOC ceiling (Stream
/// `mission-byok-enterprise-tenants`). Kept `pub(crate)` — public
/// surface is [`encryption::KeySource`].
pub(crate) mod encryption_kms;
/// Cloud KMS wrap/unwrap of the wrapped data-encryption-key (DEK)
/// for BYOK enterprise tenants (Stream
/// `mission-byok-enterprise-tenants`). Empty in default builds —
/// each provider is gated behind its own `kms-{aws,gcp,azure}`
/// Cargo feature so default builds don't pull cloud HTTP stacks.
#[cfg(feature = "kms")]
pub mod kms;
/// Wiremock-based tests for the cloud KMS providers (Stream
/// `mission-byok-enterprise-tenants`).
#[cfg(all(test, feature = "kms"))]
#[path = "kms_tests.rs"]
mod kms_tests;
/// Per-tenant resource quotas for budget and auto-approval gates.
pub mod tenant_quota;
/// Quota kind enums + error type for the tenant_quota gate
/// (split from `tenant_quota.rs` for the 500-LOC budget).
pub mod tenant_quota_kinds;
/// Usage accounting + headroom-check methods for `TenantQuotaStore`
/// (split from `tenant_quota.rs` for the 500-LOC budget).
pub mod tenant_quota_usage;
/// sqlx connection-pool tuning knobs (env-driven).
pub mod pool_config;
/// Opinionated SQLite WAL tuning profiles
/// (Stream `mission-sqlite-wal-tuning`).
pub mod wal_tuning;
/// Worker archive (sleep / wake) — frozen JSON snapshot store
/// (Stream `claude-worker-archive-rehydrate`).
pub mod worker_archive;
/// Worker dependency graph — declared inter-worker dependencies with
/// per-kind cycle detection and downstream impact closure
/// (Stream `claude-worker-dependency-tracking`).
pub mod worker_deps;
/// Per-worker KPI time-series store — append-only observations
/// recorded against KPI names from the worker's charter
/// (Stream `claude-worker-kpi-tracker`, mission claim 2).
pub mod worker_kpi;
/// Per-worker quarterly OKRs — objectives + weighted key-results
/// scored against `worker_kpi_value`
/// (Stream `claude-worker-okr-cli`, mission claim 2 worker-goals layer).
pub mod worker_okr;
/// Data types backing [`worker_okr`] — split out under the 500-LOC
/// per-file ceiling. Re-exported through `worker_okr::*`; reach for
/// that path from outside the crate.
pub(crate) mod worker_okr_types;
/// Per-worker spend cap write helpers — `argo worker budget set/show`
/// (Stream `claude-worker-budget-cli`). Caps live on the worker's
/// `auto_approval_json` column; the spend totals come from the
/// existing `worker_budget` ledger.
pub mod worker_budget_write;
/// Worker presence (`last_active_at`) and fleet listing helpers
/// (Stream `claude-worker-presence-tracking`).
pub mod worker_presence;
/// Per-tenant gateway rate-limit overrides (Stream
/// `mission-per-tenant-rate-limiting`, MISSION_BACKLOG Section 1
/// #21). Stores one row per tenant that supersedes the gateway's
/// config-driven `requests_per_minute` default.
pub mod tenant_rate_limit;
/// Per-operator risk-level overrides for the action classification
/// pipeline (Stream `mission-operator-specific-risk-overrides`,
/// MISSION_BACKLOG Section 3 Claim 6 #122). Stores one row per
/// `(operator_id, action_kind, vertical_id)` triple. Missing row →
/// caller falls back to the manifest-driven default.
pub mod operator_risk_overrides;
/// Per-worker retrospective store (Stream `mission-retro-review-ui`,
/// MISSION_BACKLOG Section 6 #209). One row per retro over a time
/// window; aggregated metrics from `audit_record` are frozen at
/// creation, and the editable 5-section operator template lives on
/// `sections_json`. `finalized_at` locks the row for further edits.
pub mod retro;
/// Tenant inspection and id-rename helpers.
pub mod tenant_query;
/// `show_tenant` + `rename_tenant` family — split out from
/// `tenant_query.rs` for the 500-LOC ceiling. Public symbols are
/// re-exported through `tenant_query`; external callers keep
/// importing from `argo_store::tenant_query::*`.
pub(crate) mod tenant_query_inspect;
/// Append-only cross-tenant trust observation log (Stream
/// `mission-cross-tenant-trust-graph`, MISSION_BACKLOG Section 3
/// Claim 4 #81). Each row is one quality observation made by a
/// consumer tenant against a skill shipped by a source tenant; the
/// in-memory `argo_skill::trust_graph::TrustGraph` consumes a Vec
/// of rows and materialises pairwise weights.
pub mod trust_observation;
/// Workflow checkpoint helpers (Stream `claude-checkpoint-resume`).
pub mod workflow_checkpoint;
mod session_store;
mod store_open;
mod workspace;

pub use argo_runtime::prompt_cache_affinity::PromptCacheAffinity;
pub use grants::SessionGrant;
pub use recovery_cache::RecoveryCacheEntry;
pub use marketplace_meta::MarketplaceArtifactRow;
pub use models::{
    AuditRow, ForgeRunRow, JudgeRow, MessageRow, MessageSearchHit, SessionHeader, WorkerId,
};
pub use session_query::SessionCreate;
pub use session_store::SessionStore;
pub use skill_runs_query::{runs_for_skill, runs_for_tenant, SkillRunRow};
pub use spend::GlobalSpend;
pub use stats::{tool_errors_histogram, DbStats};

use argo_types::ArgoError;

// The `lib_tests.rs` glob `use super::*;` previously picked up
// `TenantId`/`WorkspaceId` from this scope. The split moved the
// runtime consumers to `session_store.rs`, so the imports are
// test-only here — keeping them avoids amending `lib_tests.rs`,
// which sits outside the brief's may-touch list.
#[cfg(test)]
use argo_types::{TenantId, WorkspaceId};

/// Result alias.
pub type StoreResult<T> = Result<T, ArgoError>;

/// Highest migration version this binary knows how to apply. The
/// gateway startup health check (`claude-startup-health-check`)
/// compares this to the on-disk `schema_version` and refuses to bind
/// when the two diverge. Bump this whenever a new `vNN` migration is
/// added.
pub const EXPECTED_SCHEMA_VERSION: u32 = 67;

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
#[cfg(test)]
#[path = "migration_reconciliation_tests.rs"]
mod migration_reconciliation_tests;
#[cfg(test)]
#[path = "handoff_stats_query_tests.rs"]
mod handoff_stats_query_tests;
