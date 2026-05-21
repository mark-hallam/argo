//! Typed audit-event kinds.
//!
//! Phase-1 kinds (turn boundaries + tool dispatch) plus the v0.2
//! kinds called out in §17.1: routing, judge, RBAC, forge, steer.
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
    /// A tool dispatch was halted because the bound vertical's
    /// `[escalation].human_approval_required` list named the tool
    /// (#660). The dispatch did NOT run; an operator must approve
    /// (or refuse) before the agent can re-issue the call.
    EscalationRequired,
    /// A pending action's auto-approval gate decided
    /// `AutoApprovalDecision::Ask` and the outbound notify
    /// dispatcher successfully delivered the ask to a channel
    /// (slack / email / sms / voice / discord / push /
    /// webinbox). One row per channel that successfully
    /// delivered — a fallback chain that lands on the 3rd
    /// channel emits one row for that channel; a v0 fan-out
    /// where 3 channels succeed in parallel emits 3 rows.
    /// Emit happens AFTER the channel's `send()` returns Ok so
    /// failed sends don't bloat the chain with phantom asks.
    /// Pairs with [`Self::ApprovalReplied`] for forensic
    /// reconstruction of the ask/reply round-trip. Payload:
    /// `{ action_id, worker_id, channel, target, risk_level,
    ///   requested_at, expiry_ts }`. (Audit finding P0-2.)
    ApprovalAsked,
    /// An operator replied to a pending approval via one of
    /// the channel inbound webhooks (slack action button,
    /// email reply parse, voice IVR, etc.). Emitted at the
    /// channel-dispatch entry point — after HMAC verify +
    /// payload decode succeed, before the gate (`approve` /
    /// `reject` state transition) runs. First reply wins per
    /// the existing channel-dispatch contract; late replies
    /// hit the log/discard path and DO NOT emit a second
    /// `ApprovalReplied` row. Payload: `{ action_id, channel,
    ///   replier, reply_kind: "approve"|"reject", reply_body,
    ///   replied_at }`. Pairs with [`Self::ApprovalAsked`].
    /// (Audit finding P0-2.)
    ApprovalReplied,
    /// An operator granted human approval for a previously-
    /// blocked tool on a specific session via `argo session
    /// approve` or the gateway equivalent (#664).
    ApprovalGranted,
    /// An operator revoked a previously-granted approval (#664).
    ApprovalRevoked,
    /// A one-shot approval grant was consumed by a successful
    /// gate-passing dispatch (#667). The corresponding grant
    /// row's `consumed_at` column is now populated; subsequent
    /// dispatches of the same tool re-block unless another grant
    /// is in place.
    ApprovalConsumed,
    /// A capability-gap signal was detected (#682, work order
    /// #4): repeated failure / manual correction / weak
    /// evidence / missing knowledge / policy denial / repeated
    /// similar task. Payload carries the gap kind + the
    /// underlying signal (e.g. `{kind:"policy_denial",
    /// tool:"...", count: N}`). The detector NEVER auto-bypasses
    /// policy — it only records the signal so a human (or the
    /// learned-skill review gate, #681) can decide what to do.
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
    /// A memory-backed skill candidate (JSON in MEMORY.md body
    /// matching the `learned_skill_candidates` schema_key
    /// convention) was materialised into a `pending_review`
    /// skill row by an operator (#697). Distinct from
    /// `SkillApproved` because nothing is activated yet — the
    /// promote step just moves the artefact from memory into
    /// the skills table for review.
    /// Payload: `{skill_id, name, vertical_id, memory_entry_id}`.
    SkillCandidatePromoted,
    /// A skill version was rolled back to a prior active-eligible
    /// row by an operator. Payload carries `name`, from/to
    /// versions, skill ids, and optional reason.
    SkillRolledBack,
    /// A scheduled skill re-evaluation detected an eval pass-rate
    /// regression and flagged the skill for operator review. Payload:
    /// `{skill_id, previous_pass_rate, current_pass_rate, eval_run_id}`.
    SkillRegressionDetected,
    /// One inbound HTTP request to the gateway. Recorded after the
    /// route runs so the payload can carry response status +
    /// wall-clock duration alongside method, path (query string
    /// stripped — may carry secrets), and the resolved principal.
    /// Liveness probes (`/health*`, `/metrics*`) are skipped by the
    /// gateway wrapper to keep the chain free of probe noise.
    HttpRequest,
    /// Whole-process worker subprocess died and the supervisor
    /// reacted (mission-oom-restart-handler). Payload carries the
    /// typed `WorkerExitReason` (Clean / OomKilled / OtherSignal /
    /// Hung) + the supervisor's `RestartDecision` (Restart with
    /// backoff / GiveUp with salvage note / IgnoreCleanExit) + the
    /// attempt counter within the rolling restart window. Distinct
    /// from `Halt` (per-turn iteration cap) and from the executor
    /// `RestartStrategy` paths (per-action retry, not whole-process
    /// death). One row per non-clean exit.
    WorkerOomRestart,
    /// A worker action or turn was blocked because the worker is
    /// currently frozen by the PM kill switch. Payload carries
    /// `worker_id` and `reason`; sibling slices add the emitters after
    /// this first-class kind exists.
    WorkerFreezeBlocked,
    /// Runtime compressed the conversation history because the
    /// next turn would have exceeded the model's context window
    /// (`mission-context-window-aware-compression`). Payload shape
    /// matches `argo_runtime::context_compress::build_audit_payload`:
    /// `{ pre_token_count, post_token_count, summary_length_tokens,
    /// turns_compressed }`. Distinct from `TurnStart`/`TurnEnd`
    /// (per-turn lifecycle) — one row per compression pass.
    /// Promoted from `HttpRequest+event_type="runtime.context_compressed"`
    /// to a first-class variant (PR-deferral from #1465).
    RuntimeContextCompressed,
    /// A retro row was created via the gateway
    /// `POST /api/v1/workers/<id>/retros` route
    /// (`mission-retro-review-ui`). Payload carries
    /// `retro_id`, `worker_id`, and the metrics window
    /// (`window_start`/`window_end`). Promoted from
    /// `HttpRequest+event_type="retro.created"` (PR-deferral
    /// from #1480) so the audit-chain verifier can categorize it.
    RetroCreated,
    /// A retro row was finalized via the gateway
    /// `POST /api/v1/workers/<id>/retros/<retro_id>/finalize`
    /// route (`mission-retro-review-ui`). Payload carries
    /// `retro_id` and `finalized_at`. Promoted from
    /// `HttpRequest+event_type="retro.finalized"` (PR-deferral
    /// from #1480).
    RetroFinalized,
    /// `argo security rotate-keys commit` recorded a rotation
    /// (`mission-automated-key-rotation`). Payload carries
    /// `actor`, `action`, `key`, `category`, `env_var`,
    /// `rotation_command`, redacted `new_key_source`,
    /// `rotation_id`, and `at`. The chain self-hash also
    /// depends on the canonical kind label, so promoting the
    /// historical `"security.key_rotated"` string into the
    /// enum keeps existing rows valid.
    SecurityKeyRotated,
    /// An approval-request row was promoted to its
    /// `default_decision` by the timeout sweeper after its
    /// `deadline_at` elapsed (downstream of
    /// `claude-approval-timeout-default-decision`). Payload
    /// carries `count` (rows promoted in this sweep) and
    /// `swept_at` (ISO-8601 UTC). One row per non-zero sweep,
    /// not per promoted approval. Variant added ahead of the
    /// sweeper's first emit so the brief that wires it can
    /// reference a first-class kind from day one.
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
    /// Gateway boot deliberately bound a non-loopback address through
    /// the development/recovery escape hatch. Payload carries bind,
    /// port, pid, TLS/proxy context, and the source of the bind value.
    GatewayInsecureBind,
    /// A new tenant (+ owner-linked workspace) was provisioned via
    /// `POST /api/v1/onboarding/provision` (stream
    /// `claude-tenant-provisioning-endpoint`). Payload carries
    /// `tenant_id`, `workspace_id`, `user_id`, `workspace_name`, and
    /// `created` (false on the idempotent re-call). Distinct from the
    /// `HttpRequest` row the gateway wrapper also writes so the
    /// onboarding-funnel verifier can categorise it directly.
    TenantProvisioned,
    /// A provider API key was validated and stored for an onboarded
    /// workspace. Payload carries provider + masked metadata only;
    /// raw key bytes must never be written to the audit chain.
    ProviderKeyConfigured,
    /// A vertical was installed into an onboarded workspace. Payload
    /// carries the vertical identifier plus workflow/skill counts.
    VerticalInstalled,
    /// A customer-facing GDPR/CCPA Article 15 data export was requested
    /// via `POST /api/v1/me/export` (stream
    /// `mission-data-export-customer-flow`). Payload carries
    /// `user_id`, `tenant_id` (when the user has one), `job_id`, and
    /// `requested_at`. One row per accepted request; rate-limit
    /// rejections (HTTP 429) do not emit.
    DataExportRequested,
    /// A principal's role binding changed in the capability engine.
    /// Payload carries `principal`, `role_or_capability`, `actor`,
    /// `at`, and `action` (`bind` or `unbind`).
    RoleBindingChanged,
    /// A principal's dynamic capability grants changed in the
    /// capability engine. Payload carries `principal`,
    /// `role_or_capability`, `actor`, `at`, and `action` (`grant` or
    /// `revoke`).
    DynamicCapabilityGranted,
    /// A magic-link `POST /api/v1/auth/verify` exchange succeeded
    /// (mission-magic-link-audit-followup-2026-05-18, audit P1 #5).
    /// Payload carries `method` (`"magic_link"`), `user_id`,
    /// `tenant_id` (may be null when the user has not yet onboarded a
    /// workspace), and `source_ip`. Distinct from [`Self::HttpRequest`]
    /// so the auth-funnel rollup can categorise success/failure
    /// without parsing payloads.
    AuthSucceeded,
    /// A magic-link `POST /api/v1/auth/verify` exchange failed
    /// (mission-magic-link-audit-followup-2026-05-18, audit P1 #5).
    /// Payload carries `method` (`"magic_link"`), `reason`
    /// (`"missing"|"invalid"|"expired"|"already_used"|"rate_limited"|
    /// "store_unavailable"`), and `source_ip`. Pre-authentication, so
    /// emitted under the [`crate::ANONYMOUS_AUDIT_TENANT_ID`] bucket.
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
    /// A vertical-workflow step began execution
    /// (mission-pm-vertical-workflow-skill-invoke, P1 #8). Payload
    /// carries `workflow_name`, `step_name`, and (for `skill_invoke`
    /// steps) `skill_name`. Pairs with [`Self::WorkflowStepCompleted`]
    /// so `argo audit show --session <id>` shows workflow-step
    /// boundaries alongside agent turns.
    WorkflowStepStarted,
    /// A vertical-workflow step finished execution
    /// (mission-pm-vertical-workflow-skill-invoke, P1 #8). Payload
    /// carries `workflow_name`, `step_name`, `status`, and (for
    /// `skill_invoke` steps) `skill_name` plus `final_text_sha`
    /// (sha-256 of the assistant's final-content text, hex-encoded)
    /// so forensic readers can correlate the step output with the
    /// agent turn's persisted assistant message without re-storing
    /// the body in the audit row.
    WorkflowStepCompleted,
}

impl AuditKind {
    /// Wire label persisted in the `kind` column.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::TurnStart => "turn_start",
            Self::AssistantMessage => "assistant_message",
            Self::ToolCall => "tool_call",
            Self::ToolResult => "tool_result",
            Self::TurnEnd => "turn_end",
            Self::Halt => "halt",
            Self::RoutingDecision => "routing.decision",
            Self::JudgeVerdict => "judge.verdict",
            Self::RbacDeny => "rbac.deny",
            Self::ForgeRunStart => "forge.run.start",
            Self::ForgeRunEnd => "forge.run.end",
            Self::SteerEnqueued => "steer.enqueued",
            Self::EscalationRequired => "escalation.required",
            Self::ApprovalAsked => "approval.asked",
            Self::ApprovalReplied => "approval.replied",
            Self::ApprovalGranted => "approval.granted",
            Self::ApprovalRevoked => "approval.revoked",
            Self::ApprovalConsumed => "approval.consumed",
            Self::CapabilityGap => "capability.gap",
            Self::SkillApproved => "skill.approved",
            Self::SkillRejected => "skill.rejected",
            Self::SkillCandidatePromoted => "skill.candidate.promoted",
            Self::SkillRolledBack => "skill.rollback",
            Self::SkillRegressionDetected => "skill.regression_detected",
            Self::HttpRequest => "http.request",
            Self::WorkerOomRestart => "worker.oom_restart",
            Self::WorkerFreezeBlocked => "worker_freeze_blocked",
            Self::RuntimeContextCompressed => "runtime.context_compressed",
            Self::RetroCreated => "retro.created",
            Self::RetroFinalized => "retro.finalized",
            Self::SecurityKeyRotated => "security.key_rotated",
            Self::ApprovalExpiredByTimeout => "approval.expired_by_timeout",
            Self::ServiceTokenBindingExpired => "service_token_binding.expired",
            Self::CrossRegionBlocked => "security.cross_region_blocked",
            Self::GatewayInsecureBind => "gateway_insecure_bind",
            Self::TenantProvisioned => "tenant.provisioned",
            Self::ProviderKeyConfigured => "onboarding.provider_key_configured",
            Self::VerticalInstalled => "onboarding.vertical_installed",
            Self::DataExportRequested => "data_export.requested",
            Self::RoleBindingChanged => "role_binding_changed",
            Self::DynamicCapabilityGranted => "dynamic_capability_granted",
            Self::AuthSucceeded => "auth.succeeded",
            Self::AuthFailed => "auth.failed",
            Self::WorkflowDefinitionCreated => "workflow_definition_created",
            Self::WorkflowHeadChanged => "workflow_head_changed",
            Self::WorkflowVersionArchived => "workflow_version_archived",
            Self::WorkflowStepStarted => "workflow.step.started",
            Self::WorkflowStepCompleted => "workflow.step.completed",
        }
    }

    /// Parse from a `kind` column value. Returns `None` for unknown
    /// labels — `argo audit verify` flags those rows so an out-of-band
    /// migration didn't introduce silent corruption.
    #[must_use]
    pub fn from_label(s: &str) -> Option<Self> {
        Some(match s {
            "turn_start" => Self::TurnStart,
            "assistant_message" => Self::AssistantMessage,
            "tool_call" => Self::ToolCall,
            "tool_result" => Self::ToolResult,
            "turn_end" => Self::TurnEnd,
            "halt" => Self::Halt,
            "routing.decision" => Self::RoutingDecision,
            "judge.verdict" => Self::JudgeVerdict,
            "rbac.deny" => Self::RbacDeny,
            "forge.run.start" => Self::ForgeRunStart,
            "forge.run.end" => Self::ForgeRunEnd,
            "steer.enqueued" => Self::SteerEnqueued,
            "escalation.required" => Self::EscalationRequired,
            "approval.asked" => Self::ApprovalAsked,
            "approval.replied" => Self::ApprovalReplied,
            "approval.granted" => Self::ApprovalGranted,
            "approval.revoked" => Self::ApprovalRevoked,
            "approval.consumed" => Self::ApprovalConsumed,
            "capability.gap" => Self::CapabilityGap,
            "skill.approved" => Self::SkillApproved,
            "skill.rejected" => Self::SkillRejected,
            "skill.candidate.promoted" => Self::SkillCandidatePromoted,
            "skill.rollback" => Self::SkillRolledBack,
            "skill.regression_detected" => Self::SkillRegressionDetected,
            "http.request" => Self::HttpRequest,
            "worker.oom_restart" => Self::WorkerOomRestart,
            "worker_freeze_blocked" => Self::WorkerFreezeBlocked,
            "runtime.context_compressed" => Self::RuntimeContextCompressed,
            "retro.created" => Self::RetroCreated,
            "retro.finalized" => Self::RetroFinalized,
            "security.key_rotated" => Self::SecurityKeyRotated,
            "approval.expired_by_timeout" => Self::ApprovalExpiredByTimeout,
            "service_token_binding.expired" => Self::ServiceTokenBindingExpired,
            "security.cross_region_blocked" => Self::CrossRegionBlocked,
            "gateway_insecure_bind" => Self::GatewayInsecureBind,
            "tenant.provisioned" => Self::TenantProvisioned,
            "onboarding.provider_key_configured" => Self::ProviderKeyConfigured,
            "onboarding.vertical_installed" => Self::VerticalInstalled,
            "data_export.requested" => Self::DataExportRequested,
            "role_binding_changed" => Self::RoleBindingChanged,
            "dynamic_capability_granted" => Self::DynamicCapabilityGranted,
            "auth.succeeded" => Self::AuthSucceeded,
            "auth.failed" => Self::AuthFailed,
            "workflow_definition_created" => Self::WorkflowDefinitionCreated,
            "workflow_head_changed" => Self::WorkflowHeadChanged,
            "workflow_version_archived" => Self::WorkflowVersionArchived,
            "workflow.step.started" => Self::WorkflowStepStarted,
            "workflow.step.completed" => Self::WorkflowStepCompleted,
            _ => return None,
        })
    }

    /// All known kinds, sorted by label. Used by tests + `argo audit
    /// verify` to enumerate expected events.
    #[must_use]
    pub fn all() -> Vec<Self> {
        let mut v = vec![
            Self::TurnStart,
            Self::AssistantMessage,
            Self::ToolCall,
            Self::ToolResult,
            Self::TurnEnd,
            Self::Halt,
            Self::RoutingDecision,
            Self::JudgeVerdict,
            Self::RbacDeny,
            Self::ForgeRunStart,
            Self::ForgeRunEnd,
            Self::SteerEnqueued,
            Self::EscalationRequired,
            Self::ApprovalAsked,
            Self::ApprovalReplied,
            Self::ApprovalGranted,
            Self::ApprovalRevoked,
            Self::ApprovalConsumed,
            Self::CapabilityGap,
            Self::SkillApproved,
            Self::SkillRejected,
            Self::SkillCandidatePromoted,
            Self::SkillRolledBack,
            Self::SkillRegressionDetected,
            Self::HttpRequest,
            Self::WorkerOomRestart,
            Self::WorkerFreezeBlocked,
            Self::RuntimeContextCompressed,
            Self::RetroCreated,
            Self::RetroFinalized,
            Self::SecurityKeyRotated,
            Self::ApprovalExpiredByTimeout,
            Self::ServiceTokenBindingExpired,
            Self::CrossRegionBlocked,
            Self::GatewayInsecureBind,
            Self::TenantProvisioned,
            Self::ProviderKeyConfigured,
            Self::VerticalInstalled,
            Self::DataExportRequested,
            Self::RoleBindingChanged,
            Self::DynamicCapabilityGranted,
            Self::AuthSucceeded,
            Self::AuthFailed,
            Self::WorkflowDefinitionCreated,
            Self::WorkflowHeadChanged,
            Self::WorkflowVersionArchived,
            Self::WorkflowStepStarted,
            Self::WorkflowStepCompleted,
        ];
        v.sort_by_key(|k| k.label());
        v
    }
}

#[cfg(test)]
#[path = "event_tests.rs"]
mod event_tests;
