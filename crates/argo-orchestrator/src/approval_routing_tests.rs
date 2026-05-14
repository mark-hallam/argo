use super::approval_routing::{
    ApprovalAttemptStatus, ApprovalChannel, ApprovalChannelKind, ApprovalRouteAttempt,
    ApprovalRouteNode, ApprovalRoutingAction, ApprovalRoutingError, ApprovalRoutingPlan,
    ApprovalRoutingRuntime, APPROVAL_ROUTING_ANNOTATION,
};
use chrono::{Duration, Utc};

fn channel(kind: ApprovalChannelKind, target: &str) -> ApprovalChannel {
    ApprovalChannel::new(kind, target)
}

fn three_channel_plan() -> ApprovalRoutingPlan {
    ApprovalRoutingPlan::new(
        "high-value",
        "High value approval",
        ApprovalRouteNode::new(
            "ceo-slack",
            channel(ApprovalChannelKind::Slack, "#ceo-approvals"),
            30 * 60 * 1000,
        )
        .with_fallback(
            ApprovalRouteNode::new(
                "cfo-sms",
                channel(ApprovalChannelKind::Sms, "+15551234567"),
                4 * 60 * 60 * 1000,
            )
            .with_fallback(ApprovalRouteNode::new(
                "ops-email",
                channel(ApprovalChannelKind::Email, "ops@example.com"),
                24 * 60 * 60 * 1000,
            )),
        ),
    )
}

#[test]
fn first_action_dispatches_root_channel() {
    let plan = three_channel_plan();
    let action = ApprovalRoutingRuntime::next_action(&plan, &[], Utc::now()).unwrap();

    assert_eq!(
        action,
        ApprovalRoutingAction::Dispatch {
            node_id: "ceo-slack".into(),
            channel: channel(ApprovalChannelKind::Slack, "#ceo-approvals"),
        }
    );
}

#[test]
fn pending_attempt_waits_until_timeout() {
    let plan = three_channel_plan();
    let sent_at = Utc::now();
    let attempts = vec![ApprovalRouteAttempt::pending(&plan.root, sent_at)];

    let action =
        ApprovalRoutingRuntime::next_action(&plan, &attempts, sent_at + Duration::minutes(10))
            .unwrap();

    assert_eq!(
        action,
        ApprovalRoutingAction::Wait {
            node_id: "ceo-slack".into(),
            until: sent_at + Duration::minutes(30),
        }
    );
}

#[test]
fn timeout_advances_to_sms_fallback() {
    let plan = three_channel_plan();
    let sent_at = Utc::now();
    let attempts = vec![ApprovalRouteAttempt::pending(&plan.root, sent_at)];

    let action =
        ApprovalRoutingRuntime::next_action(&plan, &attempts, sent_at + Duration::minutes(31))
            .unwrap();

    assert_eq!(
        action,
        ApprovalRoutingAction::Dispatch {
            node_id: "cfo-sms".into(),
            channel: channel(ApprovalChannelKind::Sms, "+15551234567"),
        }
    );
}

#[test]
fn failed_delivery_advances_without_waiting() {
    let plan = three_channel_plan();
    let sent_at = Utc::now();
    let mut attempt = ApprovalRouteAttempt::pending(&plan.root, sent_at);
    attempt.status = ApprovalAttemptStatus::Failed;

    let action = ApprovalRoutingRuntime::next_action(&plan, &[attempt], sent_at).unwrap();

    assert!(matches!(
        action,
        ApprovalRoutingAction::Dispatch { node_id, .. } if node_id == "cfo-sms"
    ));
}

#[test]
fn approved_or_rejected_attempt_resolves() {
    let plan = three_channel_plan();
    let sent_at = Utc::now();
    let sms = plan.root.fallback.as_deref().unwrap();
    let mut attempt = ApprovalRouteAttempt::pending(sms, sent_at);
    attempt.status = ApprovalAttemptStatus::Approved;
    attempt.resolved_at = Some(sent_at + Duration::minutes(2));

    let action = ApprovalRoutingRuntime::next_action(&plan, &[attempt], Utc::now()).unwrap();

    assert_eq!(
        action,
        ApprovalRoutingAction::Resolved {
            node_id: "cfo-sms".into(),
            status: ApprovalAttemptStatus::Approved,
        }
    );
}

#[test]
fn final_timeout_exhausts_the_tree() {
    let plan = three_channel_plan();
    let sent_at = Utc::now();
    let email = plan
        .root
        .fallback
        .as_deref()
        .unwrap()
        .fallback
        .as_deref()
        .unwrap();
    let attempts = vec![ApprovalRouteAttempt::pending(email, sent_at)];

    let action =
        ApprovalRoutingRuntime::next_action(&plan, &attempts, sent_at + Duration::hours(25))
            .unwrap();

    assert_eq!(
        action,
        ApprovalRoutingAction::Exhausted {
            last_node_id: "ops-email".into(),
        }
    );
}

#[test]
fn plan_serializes_as_skill_ir_annotation_payload() {
    let plan = three_channel_plan();
    let raw = serde_json::to_string(&plan).unwrap();
    let restored: ApprovalRoutingPlan = serde_json::from_str(&raw).unwrap();

    assert_eq!(APPROVAL_ROUTING_ANNOTATION, "skycot.approval_routing.v1");
    assert_eq!(restored, plan);
}

#[test]
fn validation_rejects_bad_targets_and_unknown_attempts() {
    let invalid = ApprovalRoutingPlan::new(
        "bad",
        "Bad",
        ApprovalRouteNode::new("root", channel(ApprovalChannelKind::Slack, ""), 1000),
    );
    assert!(matches!(
        invalid.validate(),
        Err(ApprovalRoutingError::EmptyChannelTarget { .. })
    ));

    let plan = three_channel_plan();
    let mut attempt = ApprovalRouteAttempt::pending(&plan.root, Utc::now());
    attempt.node_id = "missing".into();
    assert!(matches!(
        ApprovalRoutingRuntime::next_action(&plan, &[attempt], Utc::now()),
        Err(ApprovalRoutingError::UnknownNode { .. })
    ));
}
