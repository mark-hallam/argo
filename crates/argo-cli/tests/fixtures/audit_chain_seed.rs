//! Deterministic seed sequence for `audit_chain_replay_smoke`.
//!
//! Returns a fixed ordered list of `(AuditKind, payload)` tuples — no
//! wall-clock dependence and no randomness — so the closure test can
//! assert structural equivalence between the audit chain and the
//! replayed trajectory across runs.
//!
//! Imported via `#[path = "fixtures/audit_chain_seed.rs"] mod …;` —
//! Cargo treats files under `tests/<subdir>/` as plain modules rather
//! than separate test binaries.

#![allow(dead_code)]

use argo_audit::AuditKind;
use serde_json::{json, Value};

pub struct SeedEvent {
    pub kind: AuditKind,
    pub payload: Value,
}

/// The full session lifecycle the closure test drives: a turn that
/// dispatches one tool, returns a result, then hits a human-approval
/// gate (ask → reply) before closing.
#[must_use]
pub fn happy_path_session() -> Vec<SeedEvent> {
    vec![
        SeedEvent {
            kind: AuditKind::TurnStart,
            payload: json!({"turn": 1}),
        },
        SeedEvent {
            kind: AuditKind::ToolCall,
            payload: json!({"tool": "shell", "call_id": "c-1", "args": {"cmd": "ls"}}),
        },
        SeedEvent {
            kind: AuditKind::ToolResult,
            payload: json!({"call_id": "c-1", "ok": true, "output": "file.txt"}),
        },
        SeedEvent {
            kind: AuditKind::ApprovalAsked,
            payload: json!({
                "action_id": "act-1",
                "channel": "slack",
                "risk_level": "medium",
            }),
        },
        SeedEvent {
            kind: AuditKind::ApprovalReplied,
            payload: json!({
                "action_id": "act-1",
                "channel": "slack",
                "reply_kind": "approve",
            }),
        },
        SeedEvent {
            kind: AuditKind::TurnEnd,
            payload: json!({"turn": 1, "final_text": "done"}),
        },
    ]
}
