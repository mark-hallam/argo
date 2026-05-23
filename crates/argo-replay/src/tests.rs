use super::*;
use argo_types::{ArgoResult, SessionId};
use async_trait::async_trait;

fn turn(n: u32, user: &str, assistant: &str, tools: &[(&str, bool)]) -> TurnRecord {
    TurnRecord {
        turn: n,
        user_text: user.to_string(),
        assistant_text: assistant.to_string(),
        tool_calls: tools
            .iter()
            .map(|(name, err)| ToolCallRecord {
                name: (*name).to_string(),
                arguments_json: "{}".into(),
                result_json: Some("{}".into()),
                is_error: *err,
            })
            .collect(),
    }
}

fn three_turn_session() -> Vec<TurnRecord> {
    vec![
        turn(0, "what is 2+2", "4", &[]),
        turn(1, "and 3+3", "6", &[("calc", false)]),
        turn(2, "save it", "saved", &[("file_write", false)]),
    ]
}

#[tokio::test]
async fn verbatim_replay_reproduces_original() {
    // §28.BB gate 1: replay reproduces original verbatim when no
    // overrides given.
    let s = SessionId::new();
    let mut src = InMemorySource::new();
    src.insert(s, three_turn_session());
    let plan = ReplayPlan::verbatim(s);
    assert!(plan.is_verbatim());
    let result = VerbatimReplayer.execute(&plan, &src).await.unwrap();
    assert_eq!(result.turns.len(), 3);
    assert_eq!(result.divergence_count(), 0);
    for t in &result.turns {
        assert_eq!(t.replayed_assistant, t.original.assistant_text);
        assert!(!t.diverged);
    }
}

#[tokio::test]
async fn replay_with_model_override_marks_plan_non_verbatim() {
    // §28.BB gate 2 setup: a non-verbatim plan does NOT promise
    // byte-equal replay. The verbatim machinery still runs (real
    // model-override replayers plug provider + judge here).
    let s = SessionId::new();
    let mut src = InMemorySource::new();
    src.insert(s, three_turn_session());
    let plan = ReplayPlan {
        source_session: s,
        model_override: Some("claude-sonnet-4-6".into()),
        tools_override: None,
        workspace_override: None,
    };
    assert!(!plan.is_verbatim());
    let result = VerbatimReplayer.execute(&plan, &src).await.unwrap();
    assert_eq!(result.turns.len(), 3);
}

#[tokio::test]
async fn replay_unknown_session_yields_provider_error() {
    let src = InMemorySource::new();
    let plan = ReplayPlan::verbatim(SessionId::new());
    assert!(VerbatimReplayer.execute(&plan, &src).await.is_err());
}

#[test]
fn diff_identical_sessions_reports_identical() {
    let a = three_turn_session();
    let b = three_turn_session();
    let d = diff_sessions(&a, &b);
    assert!(d.identical);
    assert_eq!(d.turn_count_left, 3);
    assert_eq!(d.turn_count_right, 3);
    assert!(d.deltas.is_empty());
    assert!(d.render().contains("identical: yes"));
}

#[test]
fn diff_detects_assistant_text_change() {
    let a = three_turn_session();
    let mut b = three_turn_session();
    b[1].assistant_text = "seven".into();
    let d = diff_sessions(&a, &b);
    assert!(!d.identical);
    assert_eq!(d.deltas.len(), 1);
    assert_eq!(d.deltas[0].turn, 1);
    assert!(d.deltas[0].assistant_text_changed);
    let rendered = d.render();
    assert!(rendered.contains("turn 1"));
    assert!(rendered.contains("[assistant]"));
}

#[test]
fn diff_detects_tool_call_changes() {
    let a = three_turn_session();
    let mut b = three_turn_session();
    // Replace `file_write` on turn 2 with `file_read`.
    b[2].tool_calls = vec![ToolCallRecord {
        name: "file_read".into(),
        arguments_json: "{}".into(),
        result_json: Some("{}".into()),
        is_error: false,
    }];
    let d = diff_sessions(&a, &b);
    assert!(!d.identical);
    let delta = d.deltas.iter().find(|t| t.turn == 2).unwrap();
    assert_eq!(delta.tools_only_left, vec!["file_write".to_string()]);
    assert_eq!(delta.tools_only_right, vec!["file_read".to_string()]);
    let rendered = d.render();
    assert!(rendered.contains("-[file_write]"));
    assert!(rendered.contains("+[file_read]"));
}

#[test]
fn diff_handles_unequal_turn_counts() {
    let a = three_turn_session();
    let b = vec![turn(0, "what is 2+2", "4", &[])];
    let d = diff_sessions(&a, &b);
    assert!(!d.identical);
    assert_eq!(d.turn_count_left, 3);
    assert_eq!(d.turn_count_right, 1);
}

#[test]
fn bisect_returns_first_violating_turn() {
    // §28.BB gate 4: bisect terminates correctly on synthetic
    // divergence cases.
    let turns = three_turn_session();
    // Hypothesis: every assistant_text is a single character.
    let first = bisect(&turns, |t| t.assistant_text.len() == 1);
    assert_eq!(first, Some(2)); // "saved" is 5 chars; turn 2 is the first violation.
}

#[test]
fn bisect_returns_none_when_hypothesis_holds_throughout() {
    let turns = three_turn_session();
    let first = bisect(&turns, |_| true);
    assert_eq!(first, None);
}

#[test]
fn bisect_returns_zero_when_first_turn_violates() {
    let turns = three_turn_session();
    let first = bisect(&turns, |_| false);
    assert_eq!(first, Some(0));
}

#[test]
fn bisect_terminates_on_empty_input() {
    let turns: Vec<TurnRecord> = vec![];
    let first = bisect(&turns, |_| false);
    assert_eq!(first, None);
}

// -------------------------------------------------------------------
// Re-verification after #1992 (eval-argo-replay-after-1992-fix).
//
// #1992 fixed cross-tenant leaks on `routes_audit_detail`,
// `routes_audit_replay`, and `routes_retros` by adding
// `AND tenant_id = ?` to the `audit_record` / retros lookups. The
// audit re-verifies `argo-replay` itself plus the adjacent code
// paths that consume it (`SessionSource` production bindings + the
// CLI/gateway routes that call `argo_audit::rows_for_session` /
// `store.list_messages`).
//
// `argo-replay` itself is a pure-logic crate decoupled from
// `argo-store` via the `SessionSource` trait, so there is no SQL
// surface in this crate to leak. The two tests below pin the
// contract that future production `SessionSource` impls must honor.
// -------------------------------------------------------------------

#[tokio::test]
async fn in_memory_source_isolates_sessions_with_overlapping_turn_data() {
    // Baseline: two distinct `SessionId`s with byte-identical
    // `TurnRecord` payloads do not bleed across each other in the
    // `InMemorySource` HashMap. Regression guard: if the source ever
    // collapses keys (e.g. via `entry().or_insert` mis-use) this test
    // fires.
    let alpha = SessionId::new();
    let bravo = SessionId::new();
    assert_ne!(alpha, bravo, "uuid generation collided");
    let mut src = InMemorySource::new();
    src.insert(alpha, three_turn_session());
    src.insert(bravo, three_turn_session());
    let alpha_turns = src.fetch_turns(alpha).await.unwrap();
    let bravo_turns = src.fetch_turns(bravo).await.unwrap();
    assert_eq!(alpha_turns.len(), 3);
    assert_eq!(bravo_turns.len(), 3);
    // Both keys present and independent: writing to one mustn't
    // surface in the other (HashMap semantics, but pinned here).
    let mut src2 = InMemorySource::new();
    src2.insert(alpha, three_turn_session());
    assert!(src2.fetch_turns(bravo).await.is_err());
}

// FINDINGS-ONLY — leaks demonstrated in adjacent code paths.
//
// `routes_session_get_export.rs::export_response` calls
// `store.list_messages(id)` + `argo_audit::rows_for_session(store,
// id)`. Both queries are `WHERE session_id = ?` with NO `AND
// tenant_id = ?` filter. A caller in tenant A who presents a
// SessionId belonging to tenant B receives B's full message log
// and (with `?include-audit=true`) B's audit chain.
//
// `routes_audit_session.rs::session_response` calls the same
// `argo_audit::rows_for_session` — same shape, same leak.
//
// `argo-replay` itself never reads from the store directly, so the
// leak does not live inside this crate's may-touch list. The test
// below simulates the failure mode against a production-shaped stub
// `SessionSource` that mirrors the missing filter; the matching
// fix-brief (`mission-gateway-session-export-tenant-bind`, see PR
// body) lands the `AND tenant_id = ?` predicate at the SQL bind
// site and removes the `#[ignore]`.
#[tokio::test]
#[ignore = "FINDINGS-ONLY: cross-tenant leak in \
    routes_session_get_export.rs + routes_audit_session.rs; \
    fix via mission-gateway-session-export-tenant-bind."]
async fn cross_tenant_session_export_leak_findings_only() {
    // Tenant-blind production-shaped source — mirrors what
    // `store.list_messages(id)` returns today: any caller, any
    // tenant, by session_id alone.
    struct TenantBlindSource {
        rows: std::collections::HashMap<SessionId, Vec<TurnRecord>>,
        // The tenant making the request — production bindings should
        // be constructed already-bound to this; the leak is exactly
        // that today's bindings ignore it.
        caller_tenant: &'static str,
        // The session's owner — what a fixed binding would compare
        // `caller_tenant` against.
        session_tenant: std::collections::HashMap<SessionId, &'static str>,
    }
    #[async_trait]
    impl SessionSource for TenantBlindSource {
        async fn fetch_turns(&self, session: SessionId) -> ArgoResult<Vec<TurnRecord>> {
            // BUG SHAPE: no comparison of `self.caller_tenant` to
            // `self.session_tenant[&session]`. A fixed impl would
            // return `ArgoError::Schema("not found")` (or a 404)
            // when the caller is not the owner.
            self.rows
                .get(&session)
                .cloned()
                .ok_or_else(|| argo_types::ArgoError::Schema("unknown session".into()))
        }
    }
    let bravo_session = SessionId::new();
    let mut rows = std::collections::HashMap::new();
    rows.insert(bravo_session, three_turn_session());
    let mut session_tenant = std::collections::HashMap::new();
    session_tenant.insert(bravo_session, "tenant-bravo");
    let leaky = TenantBlindSource {
        rows,
        caller_tenant: "tenant-alpha",
        session_tenant,
    };
    let plan = ReplayPlan::verbatim(bravo_session);
    let result = VerbatimReplayer.execute(&plan, &leaky).await.unwrap();
    // ASSERTING THE LEAK: tenant-alpha caller successfully reads
    // tenant-bravo's session turns. A fix at the production binding
    // would make this `result.is_err()` — flip when the fix-brief
    // lands and remove the `#[ignore]`.
    assert_eq!(result.turns.len(), 3);
    assert_ne!(leaky.caller_tenant, leaky.session_tenant[&bravo_session]);
}
