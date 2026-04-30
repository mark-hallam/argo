use super::*;
use argo_types::SessionId;

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
