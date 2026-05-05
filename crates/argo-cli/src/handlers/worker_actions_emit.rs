//! Worker v3 action emitter (#729). Owns the OS-level rules
//! for translating a model-proposed action into a persisted
//! `worker_action` row:
//!
//! 1. Classify the action against the bound vertical's
//!    `[actions]` section (`SafeAutonomous` /  `Approval` /
//!    `Forbidden`).
//! 2. Stamp `risk_level` + `requires_approval` from the
//!    declaration onto the persisted row — the model NEVER
//!    overrides these.
//! 3. Reject undeclared action_types as `blocked` so the
//!    operator sees the gap in the ledger.
//!
//! See `docs/design/worker_v3_execution.md` §2 + §5 for the
//! semantics this module enforces.

use super::worker_actions::ActionRecord;
use serde_json::Value;

/// Categories the OS uses when classifying an emitted action
/// against the bound vertical's `[actions]` section. See the
/// design doc §2 for the boundary semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum ApprovalCategory {
    /// `requires_approval = false` per the manifest. Worker may
    /// transition `pending → approved → executing → completed`
    /// autonomously.
    SafeAutonomous,
    /// `requires_approval = true` per the manifest. Worker may
    /// only emit `pending`; an explicit `argo worker actions
    /// approve` is required to advance.
    Approval,
    /// `action_type` is not declared in the vertical's `[actions]`
    /// section. The OS rejects the emission and persists the
    /// action as `blocked` with `blocked_reason` naming the gap.
    Forbidden,
}

/// Inputs the action emitter takes. Bundled into a struct so
/// the call sites stay readable as the field set grows in
/// #730+ (e.g. when execution_status, evidence accumulators,
/// and per-step grouping land).
#[allow(dead_code)]
pub(crate) struct ActionEmissionInputs<'a> {
    pub vertical: &'a argo_vertical::VerticalManifest,
    pub job_id: &'a str,
    pub action_type: &'a str,
    pub payload: Value,
    pub item_id: Option<&'a str>,
    pub platform: Option<&'a str>,
    pub evidence: Vec<String>,
}

/// Classify a proposed `action_type` against the bound
/// vertical's manifest. The OS uses the result to decide how
/// to write the action row (or reject the emission entirely).
///
/// Verticals without an `[actions]` section behave as v2 —
/// every emission is `Forbidden` because no vocabulary is
/// declared. That's the safe default: opt-in per vertical.
#[allow(dead_code)]
pub(crate) fn classify_for_emission(
    vertical: &argo_vertical::VerticalManifest,
    action_type: &str,
) -> ApprovalCategory {
    let Some(actions) = vertical.actions.as_ref() else {
        return ApprovalCategory::Forbidden;
    };
    let Some(decl) = actions.actions.get(action_type) else {
        return ApprovalCategory::Forbidden;
    };
    if decl.requires_approval {
        ApprovalCategory::Approval
    } else {
        ApprovalCategory::SafeAutonomous
    }
}

/// Build an `ActionRecord` ready to be persisted as the
/// initial state-row for a new action. The state is `pending`
/// (or `blocked` when the emission is forbidden). The
/// `risk_level` and `requires_approval` fields are stamped
/// from the bound vertical's manifest, NOT the caller — the
/// model can't override the vertical's classification.
///
/// Forbidden action_types still return Ok with state pre-set
/// to `blocked` so the operator sees the rejection in the
/// ledger rather than an opaque error.
#[allow(dead_code)]
pub(crate) fn build_emission(inputs: &ActionEmissionInputs<'_>) -> ActionRecord {
    let now = chrono::Utc::now().to_rfc3339();
    let action_id = uuid::Uuid::now_v7().to_string();
    let category = classify_for_emission(inputs.vertical, inputs.action_type);

    let (risk_level, requires_approval, state, blocked_reason) = match category {
        ApprovalCategory::SafeAutonomous | ApprovalCategory::Approval => {
            // `classify_for_emission` already proved the action
            // is declared, so the chained lookup is total. We
            // unwrap_or_else to a sentinel rather than `expect`
            // to satisfy clippy's `expect_used` lint and to
            // degrade gracefully if the manifest mutates between
            // classify and build (impossible today, but cheap to
            // be defensive).
            let decl = inputs
                .vertical
                .actions
                .as_ref()
                .and_then(|s| s.actions.get(inputs.action_type));
            let Some(decl) = decl else {
                // Treat as forbidden if somehow lost between
                // calls — same code path as the explicit
                // forbidden case below.
                return forbidden_record(
                    &action_id,
                    &now,
                    inputs.job_id,
                    inputs.action_type,
                    &inputs.vertical.vertical.id,
                    inputs.payload.clone(),
                    inputs.item_id,
                    inputs.platform,
                    inputs.evidence.clone(),
                );
            };
            let risk = match decl.risk {
                argo_vertical::ActionRiskLevel::Low => "low",
                argo_vertical::ActionRiskLevel::Medium => "medium",
                argo_vertical::ActionRiskLevel::High => "high",
            };
            (
                risk.to_string(),
                decl.requires_approval,
                "pending".to_string(),
                None,
            )
        }
        ApprovalCategory::Forbidden => (
            "low".to_string(),
            false,
            "blocked".to_string(),
            Some(format!(
                "no capability grant for action_type '{}' in vertical '{}'",
                inputs.action_type, inputs.vertical.vertical.id
            )),
        ),
    };

    ActionRecord {
        schema_key: ActionRecord::SCHEMA_KEY.to_string(),
        action_id,
        job_id: inputs.job_id.to_string(),
        item_id: inputs.item_id.map(String::from),
        action_type: inputs.action_type.to_string(),
        risk_level,
        requires_approval,
        state,
        payload: inputs.payload.clone(),
        approved_by: None,
        approved_at: None,
        executed_at: None,
        platform: inputs.platform.map(String::from),
        result: None,
        evidence: inputs.evidence.clone(),
        blocked_reason,
        created_at: now,
    }
}

#[allow(clippy::too_many_arguments)]
fn forbidden_record(
    action_id: &str,
    now: &str,
    job_id: &str,
    action_type: &str,
    vertical_id: &str,
    payload: Value,
    item_id: Option<&str>,
    platform: Option<&str>,
    evidence: Vec<String>,
) -> ActionRecord {
    ActionRecord {
        schema_key: ActionRecord::SCHEMA_KEY.to_string(),
        action_id: action_id.to_string(),
        job_id: job_id.to_string(),
        item_id: item_id.map(String::from),
        action_type: action_type.to_string(),
        risk_level: "low".to_string(),
        requires_approval: false,
        state: "blocked".to_string(),
        payload,
        approved_by: None,
        approved_at: None,
        executed_at: None,
        platform: platform.map(String::from),
        result: None,
        evidence,
        blocked_reason: Some(format!(
            "no capability grant for action_type '{action_type}' in vertical '{vertical_id}'"
        )),
        created_at: now.to_string(),
    }
}

// ───────────────── extract proposals (#729) ──────────────

/// One action proposal extracted from the worker's assistant
/// text. Pre-classification — the OS hasn't yet decided whether
/// to allow / gate / forbid it.
#[derive(Debug, Clone)]
pub(crate) struct ProposedAction {
    pub action_type: String,
    pub payload: Value,
    pub item_id: Option<String>,
    pub platform: Option<String>,
    pub evidence: Vec<String>,
}

/// Walk an assistant message looking for fenced `action` JSON
/// blocks of the form:
///
/// ````markdown
/// ```action
/// {
///   "action_type": "verify_price",
///   "payload": {...},
///   "platform": "ebay",      // optional
///   "item_id": "skates-110", // optional
///   "evidence": ["https://..."]
/// }
/// ```
/// ````
///
/// Returns every parseable proposal. Malformed blocks are
/// skipped (so a stray fence in prose can't crash the
/// extractor). At least `action_type` + `payload` are required;
/// other fields default sensibly.
pub(crate) fn extract_action_proposals(text: &str) -> Vec<ProposedAction> {
    let mut out = Vec::new();
    let needle = "```action";
    let bytes = text.as_bytes();
    let mut cursor = 0;
    while let Some(rel) = text[cursor..].find(needle) {
        let block_start = cursor + rel + needle.len();
        // Skip optional language extras up to the newline.
        let body_start = match text[block_start..].find('\n') {
            Some(n) => block_start + n + 1,
            None => break,
        };
        // Closing fence ``` at the start of a line.
        let body_end = match find_close_fence(&text[body_start..]) {
            Some(n) => body_start + n,
            None => break,
        };
        let body = &text[body_start..body_end];
        if let Some(parsed) = parse_action_body(body) {
            out.push(parsed);
        }
        cursor = body_end + 3; // past the closing ```
                               // Walk to next newline so we don't loop on the same fence.
        if cursor < bytes.len() {
            cursor = match text[cursor..].find('\n') {
                Some(n) => cursor + n + 1,
                None => bytes.len(),
            };
        }
    }
    out
}

/// Find the byte index of the next closing ``` that starts at
/// the beginning of a line. Returns the index of the first `
/// of the fence (so the caller can take `..idx` for body).
fn find_close_fence(s: &str) -> Option<usize> {
    let mut i = 0;
    let bytes = s.as_bytes();
    while i < bytes.len() {
        // Match ``` at start of line.
        if bytes[i..].starts_with(b"```") && (i == 0 || bytes[i - 1] == b'\n') {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn parse_action_body(body: &str) -> Option<ProposedAction> {
    let v: Value = serde_json::from_str(body.trim()).ok()?;
    let action_type = v.get("action_type")?.as_str()?.to_string();
    let payload = v.get("payload").cloned().unwrap_or(Value::Null);
    let item_id = v.get("item_id").and_then(|x| x.as_str()).map(String::from);
    let platform = v.get("platform").and_then(|x| x.as_str()).map(String::from);
    let evidence = v
        .get("evidence")
        .and_then(|x| x.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|e| e.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    if action_type.trim().is_empty() {
        return None;
    }
    Some(ProposedAction {
        action_type,
        payload,
        item_id,
        platform,
        evidence,
    })
}

/// Top-level emission orchestrator. Given the worker's assistant
/// text, the bound vertical, the originating job_id, and the
/// research-evidence verdict from #727, produce ready-to-persist
/// `ActionRecord`s.
///
/// The evidence gate (#729 Quality Gate per the design doc):
/// when `evidence_sufficient = false`, any approval-gated action
/// is downgraded to `state = "blocked"` rather than `"pending"`.
/// The operator sees the gap in the ledger; the model never
/// gets to claim approval-readiness on top of weak research.
pub(crate) fn emit_actions_from_text(
    vertical: &argo_vertical::VerticalManifest,
    job_id: &str,
    assistant_text: &str,
    evidence_sufficient: bool,
) -> Vec<ActionRecord> {
    extract_action_proposals(assistant_text)
        .into_iter()
        .map(|p| {
            let inputs = ActionEmissionInputs {
                vertical,
                job_id,
                action_type: &p.action_type,
                payload: p.payload,
                item_id: p.item_id.as_deref(),
                platform: p.platform.as_deref(),
                evidence: p.evidence,
            };
            let mut rec = build_emission(&inputs);
            if !evidence_sufficient && rec.requires_approval && rec.state == "pending" {
                rec.state = "blocked".to_string();
                rec.blocked_reason = Some(
                    "research evidence insufficient — see result.json's `evidence` \
                     summary; approval-gated action cannot proceed without verified \
                     sources."
                        .to_string(),
                );
            }
            rec
        })
        .collect()
}

// ───────────────── persistence + plan render (#729) ──────

/// Append a single `ActionRecord` to the workspace's memory
/// namespace as a `worker_action` row scoped to `vertical_id`.
/// The action ledger uses memory-row append-only semantics —
/// each state transition (#730+) appends a NEW row carrying the
/// full snapshot at that state; the "current state" of an
/// action is determined by latest-wins per `action_id`.
///
/// Errors are surfaced; callers in `process_job` log and continue
/// so a single bad action doesn't tank the whole job's bundle.
pub(crate) async fn persist_action_record(
    memory: &argo_memory::MemoryStore,
    workspace: argo_types::WorkspaceId,
    vertical_id: &str,
    record: &super::worker_actions::ActionRecord,
) -> argo_types::ArgoResult<()> {
    let body = serde_json::to_string(record)
        .map_err(|e| argo_types::ArgoError::Provider(format!("serialize ActionRecord: {e}")))?;
    memory
        .append_for_vertical(
            workspace,
            argo_memory::MemoryFile::Memory,
            body,
            None,
            vertical_id,
        )
        .await?;
    Ok(())
}

/// Render the `execution_plan.md` artifact alongside the rest
/// of the per-job bundle. Groups emitted actions by state with
/// the operator's reading order:
///
///   1. **Pending approval** — the queue the human reviews next
///   2. **Pending (autonomous)** — safe-autonomous actions ready
///      to run without approval
///   3. **Blocked** — undeclared action_types or evidence-gated
///      approval-gated emissions, with the `blocked_reason` so
///      the operator sees the gap
///
/// When the worker emitted no actions, returns a clear empty-
/// state placeholder rather than an empty file. The renderer is
/// pure — takes a slice of records, returns a string — so it's
/// trivial to unit-test alongside the orchestrator.
pub(crate) fn render_execution_plan(records: &[super::worker_actions::ActionRecord]) -> String {
    if records.is_empty() {
        return "# Execution plan\n\n_(no actions emitted this run)_\n".to_string();
    }
    let mut pending_approval: Vec<&super::worker_actions::ActionRecord> = Vec::new();
    let mut pending_safe: Vec<&super::worker_actions::ActionRecord> = Vec::new();
    let mut blocked: Vec<&super::worker_actions::ActionRecord> = Vec::new();
    for r in records {
        match (r.state.as_str(), r.requires_approval) {
            ("pending", true) => pending_approval.push(r),
            ("pending", false) => pending_safe.push(r),
            ("blocked", _) => blocked.push(r),
            _ => {} // approved/executing/completed/failed not seen at emission time
        }
    }

    let mut out = String::from("# Execution plan\n\n");
    out.push_str(&format!(
        "Emitted {} action(s) — {} pending approval, {} \
         autonomous-safe, {} blocked.\n\n",
        records.len(),
        pending_approval.len(),
        pending_safe.len(),
        blocked.len(),
    ));

    if !pending_approval.is_empty() {
        out.push_str("## Pending approval\n\n");
        out.push_str(
            "These actions require an explicit `argo worker actions approve <id>` \
             before they advance.\n\n",
        );
        for r in &pending_approval {
            out.push_str(&render_action_line(r));
        }
        out.push('\n');
    }

    if !pending_safe.is_empty() {
        out.push_str("## Pending (autonomous-safe)\n\n");
        out.push_str(
            "Autonomous-safe actions — the worker may advance these without \
             operator approval per the bound vertical's manifest.\n\n",
        );
        for r in &pending_safe {
            out.push_str(&render_action_line(r));
        }
        out.push('\n');
    }

    if !blocked.is_empty() {
        out.push_str("## Blocked\n\n");
        out.push_str(
            "These emissions were rejected by the OS — undeclared action types \
             or approval-gated emissions on top of insufficient research evidence. \
             The operator must address the cause before re-emitting.\n\n",
        );
        for r in &blocked {
            out.push_str(&render_action_line(r));
            if let Some(reason) = r.blocked_reason.as_deref() {
                out.push_str(&format!("    - reason: {reason}\n"));
            }
        }
        out.push('\n');
    }

    out
}

fn render_action_line(r: &super::worker_actions::ActionRecord) -> String {
    let platform = r
        .platform
        .as_deref()
        .map(|p| format!(" · platform `{p}`"))
        .unwrap_or_default();
    let item = r
        .item_id
        .as_deref()
        .map(|i| format!(" · item `{i}`"))
        .unwrap_or_default();
    format!(
        "- `{}` — `{}` (risk: {}){platform}{item}\n",
        r.action_id, r.action_type, r.risk_level,
    )
}

#[cfg(test)]
#[path = "worker_actions_emit_tests.rs"]
mod tests;
