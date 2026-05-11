//! Offline regression tests for the computer-use loop, driven
//! through `argo_executor_computer_use::testing::cu_harness_run`.
//! Mirrors the production `ComputerUseExecutor::run_loop` control
//! flow against scripted `MockAnthropic` + `MockE2bClient`, so
//! every assertion is byte-deterministic and runs without any
//! `E2B_API_KEY` / `ANTHROPIC_API_KEY` set.
//!
//! These guard the Mission Claim 5 ("real-world execution")
//! regression surface that `cu_smoke.rs` covers only when env
//! vars are wired up. `cu_smoke.rs` is intentionally unchanged.

use argo_executor_computer_use::anthropic::{CuStepResponse, StopReason};
use argo_executor_computer_use::audit::AuditRow;
use argo_executor_computer_use::e2b::{ActionResult, CuAction};
use argo_executor_computer_use::testing::{
    cu_harness_run, CannedResponse, ScriptedAction, TestScript,
};
use argo_executor_computer_use::Outcome;

fn click(x: u32, y: u32, tokens: u32) -> CannedResponse {
    CannedResponse::one(CuStepResponse::ToolCall {
        action: CuAction::Click { x, y },
        tokens_used: tokens,
    })
}

fn typed(text: &str, tokens: u32) -> CannedResponse {
    CannedResponse::one(CuStepResponse::ToolCall {
        action: CuAction::Type {
            text: text.into(),
        },
        tokens_used: tokens,
    })
}

fn key(keys: &[&str], tokens: u32) -> CannedResponse {
    CannedResponse::one(CuStepResponse::ToolCall {
        action: CuAction::Key {
            keys: keys.iter().map(|s| (*s).to_string()).collect(),
        },
        tokens_used: tokens,
    })
}

fn mouse_move(x: u32, y: u32, tokens: u32) -> CannedResponse {
    CannedResponse::one(CuStepResponse::ToolCall {
        action: CuAction::MouseMove { x, y },
        tokens_used: tokens,
    })
}

fn stop_success(text: &str, tokens: u32) -> CannedResponse {
    CannedResponse::one(CuStepResponse::Stop {
        reason: StopReason::Success,
        text: Some(text.into()),
        tokens_used: tokens,
    })
}

fn stop_failure(text: &str, tokens: u32) -> CannedResponse {
    CannedResponse::one(CuStepResponse::Stop {
        reason: StopReason::Failure,
        text: Some(text.into()),
        tokens_used: tokens,
    })
}

fn ok_response() -> ActionResult {
    ActionResult {
        ok: true,
        text: None,
        screenshot_b64: None,
    }
}

fn ok_inline_screenshot() -> ActionResult {
    ActionResult {
        ok: true,
        text: None,
        // Some(...) exercises the inline-screenshot branch in
        // the production loop; the harness mirrors it.
        screenshot_b64: Some("aGFybmVzcw==".into()),
    }
}

fn scripted(expected: CuAction, response: ActionResult) -> ScriptedAction {
    ScriptedAction { expected, response }
}

/// Regression 1: a single-action turn (click) followed by a
/// `stop_success` cleanly produces `Outcome::Completed` with the
/// captured trail. Audit log has exactly one row.
#[tokio::test]
async fn single_action_then_stop_success_completes() {
    let script = TestScript::new(
        vec![click(120, 240, 50), stop_success("done", 25)],
        vec![scripted(CuAction::Click { x: 120, y: 240 }, ok_response())],
    );
    let run = cu_harness_run(script).await;
    let outcome = run.outcome.expect("loop returns an outcome");
    let Outcome::Completed { result, .. } = outcome else {
        panic!("expected Completed; got: {outcome:?}");
    };
    assert_eq!(result.actions_taken.len(), 1);
    assert!(result.actions_taken[0].contains("click (120,240)"));
    assert_eq!(result.final_text.as_deref(), Some("done"));
    assert_eq!(run.received_actions.len(), 1);
    assert_eq!(run.received_actions[0], CuAction::Click { x: 120, y: 240 });
    assert_eq!(run.anthropic_calls, 2);
    assert_eq!(run.audit_rows.len(), 1);
    let only: &AuditRow = &run.audit_rows[0];
    assert_eq!(only.step, 1);
    assert_eq!(only.tokens_used, 50);
}

/// Regression 2: mixed action turn — covers every parser-supported
/// CU primitive (click, type, key, mouse_move) plus `stop_success`
/// and `stop_failure` shape parity. We use a sequence of separate
/// single-action turns because today's parser is single-action;
/// when `mission-cu-parser-multi-action` lands this test extends to
/// assert multiple actions per turn.
#[tokio::test]
async fn covers_every_parser_supported_action_shape() {
    let script = TestScript::new(
        vec![
            click(5, 6, 10),
            typed("hello", 10),
            key(&["ctrl", "a"], 10),
            mouse_move(99, 99, 10),
            stop_success("ok", 5),
        ],
        vec![
            scripted(CuAction::Click { x: 5, y: 6 }, ok_response()),
            scripted(
                CuAction::Type {
                    text: "hello".into(),
                },
                ok_response(),
            ),
            scripted(
                CuAction::Key {
                    keys: vec!["ctrl".into(), "a".into()],
                },
                ok_response(),
            ),
            scripted(CuAction::MouseMove { x: 99, y: 99 }, ok_response()),
        ],
    );
    let run = cu_harness_run(script).await;
    assert!(matches!(run.outcome, Ok(Outcome::Completed { .. })));
    assert_eq!(run.received_actions.len(), 4);
    assert_eq!(run.received_actions[0], CuAction::Click { x: 5, y: 6 });
    assert!(matches!(run.received_actions[1], CuAction::Type { .. }));
    assert!(matches!(run.received_actions[2], CuAction::Key { .. }));
    assert_eq!(run.received_actions[3], CuAction::MouseMove { x: 99, y: 99 });
    assert_eq!(run.steps, 5);
}

/// Regression 3: inline-screenshot path — the sandbox embeds the
/// next screenshot in the action response (`screenshot_b64:
/// Some(...)`). The loop still produces the canonical step trail
/// without double-counting steps or losing the screenshot. Guards
/// against the regression `mission-cu-double-screenshot-fix`
/// addressed in the production loop.
#[tokio::test]
async fn inline_screenshot_path_does_not_break_step_trail() {
    let script = TestScript::new(
        vec![click(1, 2, 30), stop_success("inline ok", 10)],
        vec![scripted(
            CuAction::Click { x: 1, y: 2 },
            ok_inline_screenshot(),
        )],
    );
    let run = cu_harness_run(script).await;
    let outcome = run.outcome.expect("loop returns an outcome");
    assert!(matches!(outcome, Outcome::Completed { .. }));
    assert_eq!(run.received_actions.len(), 1);
    assert_eq!(run.audit_rows.len(), 1);
    let row = &run.audit_rows[0];
    assert!(
        row.result.screenshot_b64.is_some(),
        "audit row must preserve inline screenshot signal: {row:?}",
    );
    assert_eq!(row.step, 1);
}

/// Regression 3b: `mission-cu-double-screenshot-fix` — when the
/// sandbox embeds `screenshot_b64: Some(...)` in the action
/// response, the loop must reuse the decoded inline bytes and
/// skip the post-action `e2b.screenshot()` HTTP fetch. With one
/// scripted action the call cadence is: 1 pre-loop screenshot,
/// 0 post-action screenshot HTTP calls = 1 total. The fallback
/// path is also asserted: with `screenshot_b64: None`, the loop
/// must call `e2b.screenshot()` post-action = 2 total.
#[tokio::test]
async fn inline_screenshot_b64_skips_post_action_http_fetch() {
    let inline_script = TestScript::new(
        vec![click(1, 2, 30), stop_success("inline ok", 10)],
        vec![scripted(
            CuAction::Click { x: 1, y: 2 },
            ok_inline_screenshot(),
        )],
    );
    let inline = cu_harness_run(inline_script).await;
    assert!(matches!(inline.outcome, Ok(Outcome::Completed { .. })));
    assert_eq!(
        inline.screenshot_calls, 1,
        "inline screenshot path must skip post-action HTTP fetch \
         (1 pre-loop + 0 post-action); got {} calls",
        inline.screenshot_calls,
    );

    let fallback_script = TestScript::new(
        vec![click(3, 4, 30), stop_success("fallback ok", 10)],
        vec![scripted(CuAction::Click { x: 3, y: 4 }, ok_response())],
    );
    let fallback = cu_harness_run(fallback_script).await;
    assert!(matches!(fallback.outcome, Ok(Outcome::Completed { .. })));
    assert_eq!(
        fallback.screenshot_calls, 2,
        "absent screenshot_b64 must fall back to post-action HTTP \
         fetch (1 pre-loop + 1 post-action); got {} calls",
        fallback.screenshot_calls,
    );
}

/// Regression 4: malformed Anthropic reply mid-loop. The mock
/// returns a `CannedResponse::Protocol` on the second turn; the
/// loop must surface a `CuError::Protocol` cleanly, not silently
/// swallow it or panic.
#[tokio::test]
async fn protocol_error_bubbles_up_cleanly_mid_loop() {
    let script = TestScript::new(
        vec![
            click(50, 60, 20),
            CannedResponse::Protocol("unparseable tool_use block".into()),
        ],
        vec![scripted(CuAction::Click { x: 50, y: 60 }, ok_response())],
    );
    let run = cu_harness_run(script).await;
    use argo_executor_computer_use::CuError;
    match run.outcome {
        Err(CuError::Protocol(msg)) => {
            assert!(msg.contains("unparseable tool_use"), "got: {msg}");
        }
        other => panic!("expected CuError::Protocol; got: {other:?}"),
    }
    assert_eq!(run.received_actions.len(), 1);
    assert_eq!(run.anthropic_calls, 2);
    assert_eq!(run.audit_rows.len(), 1);
}

/// Regression 5b (mission-cu-parser-multi-action): a single turn
/// emits two actions (mouse_move then left_click). Both must
/// dispatch to the sandbox in document order with only ONE
/// anthropic round-trip (vs today's one round-trip per dropped
/// action). Audit rows for the two actions both reference the
/// single post-turn screenshot.
#[tokio::test]
async fn multi_action_turn_dispatches_both_actions_in_one_round_trip() {
    let multi = CannedResponse::Ok(vec![
        CuStepResponse::ToolCall {
            action: CuAction::MouseMove { x: 10, y: 20 },
            tokens_used: 30,
        },
        CuStepResponse::ToolCall {
            action: CuAction::Click { x: 10, y: 20 },
            tokens_used: 0,
        },
    ]);
    let script = TestScript::new(
        vec![multi, stop_success("done", 5)],
        vec![
            scripted(CuAction::MouseMove { x: 10, y: 20 }, ok_response()),
            scripted(CuAction::Click { x: 10, y: 20 }, ok_response()),
        ],
    );
    let run = cu_harness_run(script).await;
    let outcome = run.outcome.expect("loop returns an outcome");
    assert!(matches!(outcome, Outcome::Completed { .. }));
    // Both actions ran in document order.
    assert_eq!(run.received_actions.len(), 2);
    assert_eq!(run.received_actions[0], CuAction::MouseMove { x: 10, y: 20 });
    assert_eq!(run.received_actions[1], CuAction::Click { x: 10, y: 20 });
    // Two anthropic calls only: one for the multi-action turn,
    // one for the terminal stop_success. Today (pre-fix) this
    // required THREE round-trips because the parser dropped the
    // click and asked the model again.
    assert_eq!(run.anthropic_calls, 2);
    assert_eq!(run.audit_rows.len(), 2);
    // Both audit rows reference the SAME post-turn screenshot
    // (one shot taken after the Vec is exhausted).
    assert_eq!(
        run.audit_rows[0].screenshot_relpath,
        run.audit_rows[1].screenshot_relpath,
    );
}

/// Regression 5: action coordinates round-trip end-to-end through
/// the loop. The mock asserts the expected sequence at the
/// trait-impl level (panicking with the diff on mismatch); the
/// test additionally checks the audit log's recorded coordinates
/// match the script. Guards against parser/coordinate bugs that
/// surface as off-by-one rendering shifts.
#[tokio::test]
async fn e2b_sees_expected_coordinates_in_order() {
    let coords: &[(u32, u32)] = &[(10, 20), (300, 400), (1279, 799)];
    let mut anthropic_script = Vec::new();
    let mut e2b_script = Vec::new();
    for &(x, y) in coords {
        anthropic_script.push(click(x, y, 15));
        e2b_script.push(scripted(CuAction::Click { x, y }, ok_response()));
    }
    anthropic_script.push(stop_failure("done with handoff", 5));

    let script = TestScript::new(anthropic_script, e2b_script);
    let run = cu_harness_run(script).await;

    let outcome = run.outcome.expect("loop returns an outcome");
    let Outcome::HumanHandoff { reason, .. } = outcome else {
        panic!("stop_failure must map to HumanHandoff; got: {outcome:?}");
    };
    assert!(reason.contains("stop_failure"), "got reason: {reason}");

    assert_eq!(run.received_actions.len(), coords.len());
    for (i, (x, y)) in coords.iter().enumerate() {
        let actual = &run.received_actions[i];
        assert_eq!(
            actual,
            &CuAction::Click { x: *x, y: *y },
            "received action at index {i} did not match script",
        );
    }
    assert_eq!(run.audit_rows.len(), coords.len());
    for (i, (x, y)) in coords.iter().enumerate() {
        let row_action = &run.audit_rows[i].action;
        assert_eq!(
            row_action,
            &CuAction::Click { x: *x, y: *y },
            "audit row {i} action did not match script",
        );
    }
}
