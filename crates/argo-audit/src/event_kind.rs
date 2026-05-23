//! The [`AuditKind`] enum — variant declarations only.
//!
//! Inherent impls (`label`, `from_label`, `all`) live in
//! `crates/argo-audit/src/event.rs` alongside the unit tests, so each
//! file stays under the 500-LOC ceiling per HARD RULE 6 of the swarm
//! protocol. Callsites continue to import via `argo_audit::AuditKind`
//! (the `pub use` in `lib.rs`) or `crate::event::AuditKind` (the
//! re-export in `event.rs`).
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
    /// An operator declined an approval-request decision via the
    /// gateway approve-or-refuse path (Stream
    /// `mission-add-approval-audit-kinds`). Distinct from
    /// [`Self::ApprovalRevoked`] (which is "undo a previously-
    /// granted tool grant") and from [`Self::ApprovalReplied`]
    /// with `reply_kind: "reject"` (which is the channel-inbound
    /// webhook reply edge before the gate's state transition
    /// runs). Pairs with [`Self::ApprovalGranted`] on the decide
    /// side: exactly one of the two is written per
    /// `approvals::decide_request` success when the canonical
    /// callsite is wired. Wire-up is deferred to a follow-up
    /// brief because `argo-store::approvals` cannot depend on
    /// `argo-audit` (argo-audit already depends on argo-store)
    /// and the gateway approve route currently uses the
    /// `worker_actions_transitions` flow, not `decide_request`.
    /// Payload (per the deferred wire-up brief):
    /// `{ approval_id, decided_by, decision: "reject",
    ///   decided_at, request_summary }`.
    ApprovalDeclined,
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
    /// A workflow-definition version was reverted-to via
    /// `POST /api/v1/verticals/{vertical_id}/workflows/{name}/rollback`
    /// (`web-workflow-rollback-feature`). Rollback is implemented as
    /// an additive new version copied from a prior one — the new
    /// row's `parent_id` is the rolled-back-from row's id. Payload
    /// carries `actor`, `tenant_id`, `vertical_id`, `name`,
    /// `rolled_back_from_version_id`, `new_version_id`, and
    /// `new_version_number`. Pairs with [`Self::WorkflowHeadChanged`]
    /// (the head-flip half of the rollback transaction) but is
    /// distinguishable so audit consumers can filter rollback vs
    /// ordinary head-promote events.
    WorkflowRolledBack,
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
    /// A worker's trust grade was advanced or rolled back by the
    /// periodic `argo-trust-engine` cron loop
    /// (`web-worker-trust-promotion-engine`). Pairs with
    /// [`Self::ApprovalGranted`] / [`Self::EscalationRequired`] /
    /// [`Self::RbacDeny`] as the input signals — the trust engine
    /// reads those rows, computes a score, and emits this row when
    /// the resulting ratchet decision moves a rung. Payload:
    /// `{ worker_id, tenant_id, from_grade, to_grade, score_total,
    /// score_breakdown: { streak_score, approval_score,
    /// escalation_score, rbac_score, quality_score, total,
    /// computed_at, worker_id }, decision: "promote" | "demote" }`.
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
    /// `/settings/members` invitation issued
    /// (`web-settings-members-invite-real-wire`). Payload carries
    /// `actor`, `tenant_id`, `invitee_email`, `role`, `cohort_scope`,
    /// `invitation_id`, and `expires_at`. One row per accepted POST
    /// to `/api/v1/workspace/members/invite`; duplicate-pending or
    /// invalid-email rejections do not emit.
    WorkspaceMemberInvited,
    /// `/settings/members` invitation accepted
    /// (`web-settings-members-invite-real-wire`). Payload carries
    /// `actor` (the joining user_id), `tenant_id`, `user_id`,
    /// `role`, and `invited_by`. Pairs with
    /// [`Self::WorkspaceMemberInvited`] so the audit trail tells the
    /// invite/accept story end-to-end.
    WorkspaceMemberJoined,
    /// `POST /api/v1/auth/logout` invoked
    /// (mission-revocation-with-verify-middleware-v1x). Emitted after
    /// the cookie JWT decode succeeds and the `jti` claim (when
    /// present) is recorded into `revoked_tokens`. Payload:
    /// `{ user_id, tenant_id, jti, at }`. Cookies without a `jti`
    /// claim (pre-jti-emission cohort) still emit the audit row with
    /// `jti: null` so the auth-funnel rollup can quantify the
    /// drain-on-exp cohort.
    LogoutCalled,
    /// The SPA hire flow's Step 3 Genesis call fell back to the
    /// local stub draft (404 / malformed / 5xx / network error) and
    /// the hire commit was completed from that stub instead of a
    /// gateway-served draft (`mission-genesis-fallback-audit-kind`).
    /// The SPA tags the `POST /api/v1/workers` body with
    /// `"genesis_fallback": true` when the draft came from the stub;
    /// the gateway emits this row alongside the regular hire
    /// `WorkerHired` row so operators can see in `/audit` when their
    /// workspace's Genesis is failing. Payload:
    /// `{ actor, tenant_id, worker_id, vertical_id }`. Pairs with
    /// [`Self::WorkerHired`] — a hire row without an accompanying
    /// `genesis.fallback_used` row in the same request means the
    /// gateway draft served cleanly.
    GenesisFallbackUsed,
    /// A workplace-knowledge wiki doc was created via the gateway
    /// (`mission-workplace-knowledge-audit-kinds`, design doc §3.3,
    /// §7 Pillar 1). The substrate (`argo-workplace-knowledge`) holds
    /// the versioned doc tree; this audit row gives `argo audit show`
    /// and the operator audit feed a first-class kind to filter on
    /// instead of the generic `HttpRequest` bucket. Emission lives in
    /// the gateway wiki brief (sibling Wave-A brief 3); this brief
    /// only makes the variant exist. Payload (per the emission brief):
    /// `{ actor, tenant_id, workspace_id, slug, version, created_by,
    /// created_at }`.
    WorkplaceKnowledgeDocCreated,
    /// A workplace-knowledge wiki doc was updated via the gateway
    /// (`mission-workplace-knowledge-audit-kinds`, design doc §3.3,
    /// §7 Pillar 1 acceptance — `SELECT kind, payload FROM
    /// audit_record WHERE kind = 'workplace_knowledge.doc.updated'`
    /// returns rows with the diff in payload after every edit). Pairs
    /// with [`Self::WorkplaceKnowledgeDocCreated`] /
    /// [`Self::WorkplaceKnowledgeDocDeleted`] for full lifecycle
    /// coverage. Payload (per the emission brief): `{ actor,
    /// tenant_id, workspace_id, slug, prev_version, new_version, diff,
    /// updated_by, updated_at }`.
    WorkplaceKnowledgeDocUpdated,
    /// A workplace-knowledge wiki doc was deleted (soft-delete /
    /// tombstoned) via the gateway
    /// (`mission-workplace-knowledge-audit-kinds`). The substrate
    /// retains the historical versions for audit; the audit row marks
    /// the lifecycle transition. Payload (per the emission brief):
    /// `{ actor, tenant_id, workspace_id, slug, last_version,
    /// deleted_by, deleted_at }`.
    WorkplaceKnowledgeDocDeleted,
    /// A workflow test run was persisted server-side via
    /// `POST /api/v1/verticals/{vid}/workflows/{name}/test_runs`
    /// (Stream `claude-followup-pr-2021-test-run-gateway-persist`,
    /// re-author of closed PR #2021). The builder's "Test run"
    /// overlay now POSTs each completed run; this audit row promotes
    /// the event out of the generic `HttpRequest` bucket so the
    /// audit feed can filter on a first-class kind. Payload (per the
    /// brief): `{ worker_id, vertical_id, workflow_name, status }`.
    /// `worker_id` carries the resolved gateway principal (the
    /// closest stand-in for an originating worker when the test was
    /// triggered by a human operator); `status` is the run-phase
    /// string (`"complete"` | `"failed"`).
    WorkflowTestRunPersisted,
    /// A workflow execution was dispatched via
    /// `POST /api/v1/verticals/{vid}/workflows/{name}/execute`
    /// (`mission-workflow-execute-route`). One row per accepted POST
    /// — refused requests (401 / 403 / 404 / 409) do not emit. Pairs
    /// with [`Self::WorkflowStepExecuted`] / [`Self::WorkflowStepFailed`]
    /// so the audit chain tells the dispatch → step-by-step story
    /// when the production `WorkerSessionRunner` is wired in. Payload:
    /// `{ actor, tenant_id, vertical_id, workflow_name, run_id,
    /// idempotency_key?, dispatched_at, ok }`. `ok` carries the
    /// synchronous run outcome — v1 runs the DAG via `MockRunner` so
    /// this lands `true` for shape-correct workflows.
    WorkflowExecuteDispatched,
    /// A single DAG-step inside a workflow_run completed successfully
    /// via the production `WorkerSessionRunner`
    /// (`wire-worker-session-runner`). Distinct from
    /// [`Self::WorkflowStepCompleted`] which covers the
    /// vertical-workflow + skill-invoke surface — this row is the
    /// MCP-dispatch arm of the DAG executor. Payload (per the
    /// emission brief): `{ tenant_id, workflow_id, task_id, tool,
    /// outputs }` where `outputs` is the JSON tool response trimmed
    /// to the runner-visible map. The row is written by the runner
    /// after capability check + MCP dispatch succeed so a forensic
    /// reader can pin who ran which tool against which tenant.
    WorkflowStepExecuted,
    /// A single DAG-step inside a workflow_run failed during the
    /// production `WorkerSessionRunner` dispatch
    /// (`wire-worker-session-runner`). Failure mode is one of:
    /// capability denied, MCP transport / call error, audit write
    /// rolled back. Payload (per the emission brief): `{ tenant_id,
    /// workflow_id, task_id, tool, reason }` where `reason` is a
    /// short string suitable for `argo audit show` rendering. Pairs
    /// with [`Self::WorkflowStepExecuted`] so success and failure
    /// share the same forensic surface.
    WorkflowStepFailed,
    /// `argo_orchestrator::DutyScheduler::tick` matched a duty,
    /// auto-approved its fire, spawned the session row, and the
    /// injected `DutyAgentRunner::run_turn` returned `Ok`. Stream
    /// `fix-duty-scheduler-invokes-agent-runner` — the brief that
    /// closed the P0 broken wire surfaced by PR #2085's review
    /// pre-flight grep (gateway boots the orchestrator's scheduler;
    /// the cron crate's `Scheduler::tick_with_audit` is not wired).
    /// Payload: `{ tenant_id, worker_id, duty_id, session_id,
    /// cron_spec, fired_at, entry_action }`. Pairs with
    /// [`Self::WorkerDutyFailed`] — exactly one of the two is
    /// written per fired duty, so an auditor can quantify the
    /// duty-loop's healthy-vs-failed ratio by counting kinds.
    WorkerDutyFired,
    /// `argo_orchestrator::DutyScheduler::tick` matched + auto-
    /// approved a duty and spawned the session row, but the
    /// injected `DutyAgentRunner::run_turn` returned `Err`. Stream
    /// `fix-duty-scheduler-invokes-agent-runner`. The scheduler
    /// keeps ticking — silent runner failures are the worst
    /// failure mode, so this variant exists specifically to make
    /// them loud. Payload mirrors [`Self::WorkerDutyFired`] with an
    /// extra `error` field carrying the stringified `ArgoError`:
    /// `{ tenant_id, worker_id, duty_id, session_id, cron_spec,
    /// fired_at, entry_action, error }`.
    WorkerDutyFailed,
}
