//! The [`AuditKind`] enum - variant declarations only.
//!
//! Inherent impls live in `event.rs`; long-form variant notes live in
//! `event_kind_docs.rs`. Callers still import via `argo_audit::AuditKind`
//! or the crate-internal `crate::event::AuditKind` re-export.
use serde::{Deserialize, Serialize};
/// Stable `kind` label persisted in the `audit_record.kind` column.
///
/// Adding a variant is API-stable; renaming or removing one is not —
/// a rename means a schema migration so old records still verify.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditKind {
    // ── Phase-1 kinds (the agent loop) ────────────────────────────
    /// `run_turn` started.
    TurnStart,
    /// Assistant produced a message (text or tool calls).
    AssistantMessage,
    /// A tool call was dispatched.
    ToolCall,
    /// A tool result came back.
    ToolResult,
    /// `run_turn` ended (with a final answer or halt).
    TurnEnd,
    /// A halt event (max iterations, error, etc.).
    Halt,
    // ── v0.2 kinds (§17.1) ────────────────────────────────────────
    /// Cost-router decision (Amendment 1, §24).
    RoutingDecision,
    /// Judge verdict (Amendment 2, §25).
    JudgeVerdict,
    /// RBAC denial — principal lacked the required capability.
    RbacDeny,
    /// Forge optimization run started.
    ForgeRunStart,
    /// Forge optimization run ended.
    ForgeRunEnd,
    /// Steer enqueued onto a running session.
    SteerEnqueued,
    /// Tool dispatch halted by a vertical human-approval rule.
    EscalationRequired,
    /// Approval ask delivered to an outbound channel.
    ///
    /// See `event_kind_docs.rs` for payload and fan-out semantics.
    ApprovalAsked,
    /// Operator reply received through an approval inbound channel.
    ///
    /// See `event_kind_docs.rs` for first-reply and payload semantics.
    ApprovalReplied,
    /// KPI-rollup scheduler persisted per-worker KPI JSON.
    WorkerKpiRolled,
    /// Operator granted human approval for a blocked tool.
    ApprovalGranted,
    /// Operator declined an approval-request decision.
    ///
    /// Distinct from revocation and inbound-channel reject replies.
    ApprovalDeclined,
    /// An operator revoked a previously-granted approval (#664).
    ApprovalRevoked,
    /// One-shot approval grant consumed by a gate-passing dispatch.
    ApprovalConsumed,
    /// Capability-gap signal detected; never auto-bypasses policy.
    CapabilityGap,
    /// A learned skill in `pending_review` was promoted to
    /// `active` by a human reviewer (#684). Distinct from
    /// `ApprovalGranted` (which is for tool capability grants).
    /// Payload: `{skill_id, name, vertical_id?}`.
    SkillApproved,
    /// A learned skill in `pending_review` was rejected by a
    /// human reviewer (#684). The skill row stays in the table
    /// for audit but the lookup never returns it.
    SkillRejected,
    /// Memory-backed skill candidate was promoted for review.
    SkillCandidatePromoted,
    /// A skill version was rolled back to a prior active-eligible
    /// row by an operator. Payload carries `name`, from/to
    /// versions, skill ids, and optional reason.
    SkillRolledBack,
    /// A scheduled skill re-evaluation detected an eval pass-rate
    /// regression and flagged the skill for operator review. Payload:
    /// `{skill_id, previous_pass_rate, current_pass_rate, eval_run_id}`.
    SkillRegressionDetected,
    /// One inbound HTTP request to the gateway.
    HttpRequest,
    /// Whole-process worker subprocess died and supervisor reacted.
    WorkerOomRestart,
    /// A worker action or turn was blocked because the worker is
    /// currently frozen by the PM kill switch. Payload carries
    /// `worker_id` and `reason`; sibling slices add the emitters after
    /// this first-class kind exists.
    WorkerFreezeBlocked,
    /// Runtime compressed conversation history for context-window fit.
    RuntimeContextCompressed,
    /// Retro row was created through the gateway.
    RetroCreated,
    /// Retro row was finalized through the gateway.
    RetroFinalized,
    /// `argo security rotate-keys commit` recorded a rotation.
    SecurityKeyRotated,
    /// Timeout sweeper promoted approval requests to default decisions.
    ApprovalExpiredByTimeout,
    /// The gateway service-token-binding sweeper deleted expired
    /// token-to-principal rows. Payload carries `count`,
    /// `swept_at_iso`, and `cutoff_iso`. One row per non-zero sweep,
    /// not per deleted binding.
    ServiceTokenBindingExpired,
    /// Gateway refused a tenant-scoped request because the serving
    /// deployment region did not match that tenant's configured
    /// data-residency region.
    CrossRegionBlocked,
    /// Gateway boot used the non-loopback bind escape hatch.
    GatewayInsecureBind,
    /// New tenant and owner-linked workspace were provisioned.
    TenantProvisioned,
    /// A provider API key was validated and stored for an onboarded
    /// workspace. Payload carries provider + masked metadata only;
    /// raw key bytes must never be written to the audit chain.
    ProviderKeyConfigured,
    /// A vertical was installed into an onboarded workspace. Payload
    /// carries the vertical identifier plus workflow/skill counts.
    VerticalInstalled,
    /// Customer-facing GDPR/CCPA Article 15 data export was requested.
    DataExportRequested,
    /// CLI Article 30 ROPA export; payload omits raw paths.
    ComplianceRopaExported,
    /// A principal's role binding changed in the capability engine.
    /// Payload carries `principal`, `role_or_capability`, `actor`,
    /// `at`, and `action` (`bind` or `unbind`).
    RoleBindingChanged,
    /// A principal's dynamic capability grants changed in the
    /// capability engine. Payload carries `principal`,
    /// `role_or_capability`, `actor`, `at`, and `action` (`grant` or
    /// `revoke`).
    DynamicCapabilityGranted,
    /// Magic-link auth verification succeeded.
    AuthSucceeded,
    /// Magic-link auth verification failed before authentication.
    AuthFailed,
    /// A new workflow-definition version was created via
    /// `POST /api/v1/verticals/{vertical_id}/workflows/{name}`
    /// (`mission-pm-workflow-edit-api`). Payload carries `actor`,
    /// `tenant_id`, `vertical_id`, `name`, and `version_id`.
    WorkflowDefinitionCreated,
    /// The workflow-definition head pointer for `(tenant, vertical,
    /// name)` was moved to a new version via
    /// `PUT /api/v1/verticals/{vertical_id}/workflows/{name}/head`
    /// (`mission-pm-workflow-edit-api`). Payload carries `actor`,
    /// `tenant_id`, `vertical_id`, `name`, and `version_id`.
    WorkflowHeadChanged,
    /// A workflow-definition version was archived (lifecycle_state
    /// flipped to `archived`) via
    /// `DELETE /api/v1/verticals/{vertical_id}/workflows/{name}/versions/{id}`
    /// (`mission-pm-workflow-edit-api`). Payload carries `actor`,
    /// `tenant_id`, `vertical_id`, `name`, and `version_id`. When the
    /// archived version was the current head, the head pointer is
    /// cleared in the same DB transaction.
    WorkflowVersionArchived,
    /// Workflow definition was rolled back by creating a new version.
    WorkflowRolledBack,
    /// A vertical-workflow step began execution
    /// (mission-pm-vertical-workflow-skill-invoke, P1 #8). Payload
    /// carries `workflow_name`, `step_name`, and (for `skill_invoke`
    /// steps) `skill_name`. Pairs with [`Self::WorkflowStepCompleted`]
    /// so `argo audit show --session <id>` shows workflow-step
    /// boundaries alongside agent turns.
    WorkflowStepStarted,
    /// A vertical-workflow step finished execution.
    WorkflowStepCompleted,
    /// Worker's trust grade advanced or rolled back.
    TrustGradeChanged,
    /// Worker hired (`POST /workers` / `argo worker hire`). Stream
    /// `web-audit-kinds-for-journey-events`. Payload: `{ actor,
    /// tenant_id, worker_id, vertical_id, name, duty_count,
    /// hired_at }`.
    WorkerHired,
    /// Vertical run requested (`POST /verticals/{vid}/run`). Payload:
    /// `{ actor, tenant_id, vertical_id, workflow_name, session_id,
    /// trigger, requested_at }`.
    VerticalRunRequested,
    /// Vertical run finished. Payload: `{ actor, tenant_id,
    /// vertical_id, workflow_name, session_id,
    /// outcome ("succeeded"|"failed"), step_count, completed_at }`.
    VerticalRunCompleted,
    /// Cron job created (`POST /cron` / `argo cron new`). Payload:
    /// `{ actor, tenant_id, job_name, schedule, platform, chat,
    /// created_at }`.
    CronJobCreated,
    /// Cron job deleted (`POST /cron/<name>/delete` / `argo cron
    /// delete <name>`). Payload: `{ actor, tenant_id, job_name,
    /// schedule, platform, chat, deleted_at, last_fired_at }`.
    CronJobDeleted,
    /// Cron job fired (scheduler-emitted; no actor). Payload:
    /// `{ tenant_id, job_name, schedule, fired_at,
    /// dispatch_target: { platform, chat } }`.
    CronJobFired,
    /// Workspace member invitation was issued.
    WorkspaceMemberInvited,
    /// Workspace member invitation was accepted.
    WorkspaceMemberJoined,
    /// Auth logout endpoint was invoked after cookie JWT decode.
    LogoutCalled,
    /// SPA hire flow used the local Genesis fallback draft.
    GenesisFallbackUsed,
    /// Workplace-knowledge wiki doc was created.
    WorkplaceKnowledgeDocCreated,
    /// Workplace-knowledge wiki doc was updated.
    WorkplaceKnowledgeDocUpdated,
    /// Workplace-knowledge wiki doc was deleted.
    WorkplaceKnowledgeDocDeleted,
    /// Workflow test run was persisted server-side.
    WorkflowTestRunPersisted,
    /// Workflow execution was dispatched.
    WorkflowExecuteDispatched,
    /// DAG workflow step completed successfully through the runner.
    WorkflowStepExecuted,
    /// DAG workflow step failed through the runner.
    WorkflowStepFailed,
    /// Duty scheduler fired a duty and the agent runner succeeded.
    WorkerDutyFired,
    /// Duty scheduler fired a duty and the agent runner failed.
    WorkerDutyFailed,
    /// Cross-tenant orchestrator handoff refused by the substrate.
    HandoffCrossTenantAttempt,
    /// HIPAA deployment-posture breadcrumb emitted at gateway boot.
    ///
    /// Payload carries `{mode, hipaa_enabled, checked_at, features[]}`.
    HipaaComplianceCheck,
}
