//! Pin the v1 channel inbound-coverage matrix.
//!
//! This is the report for `eval-channels-inbound-coverage-v2`, kept in
//! a test file because the brief allows `argo-channels` and gateway
//! channel-route surfaces, not eval docs.
//!
//! | channel | outbound arm | inbound route | token handler | decision sink |
//! |---|---:|---:|---:|---:|
//! | discord | yes | yes | yes | yes |
//! | email | yes | yes | yes | yes |
//! | imessage | yes | yes | yes | yes |
//! | matrix | no | no | no | no |
//! | push | yes | yes | yes | yes |
//! | slack | yes | yes | yes | yes |
//! | sms | yes | yes | yes | yes |
//! | teams | no | no | no | no |
//! | voice | yes | yes | yes | yes |
//! | webinbox | yes | yes | yes | yes |
//! | whatsapp | no | yes | yes | yes |
//!
//! Findings / brief shapes:
//!
//! - `mission-sms-inbound-approval-reply-route`: shipped SMS inbound
//!   settlement through `routes_channels_sms.rs`.
//! - `mission-whatsapp-wire-outbound-dispatch`: WhatsApp already has
//!   inbound settlement, but `EnvChannelSender` refuses outbound.
//! - `mission-teams-inbound-approval-route` and
//!   `mission-matrix-inbound-approval-route`: do not unblock v1 until
//!   those channels are intentionally moved out of DEFERRED-v1.x.

const NOTIFY_ENV: &str = include_str!("../../argo-cli/src/handlers/worker_actions_notify_env.rs");
const ROUTES: &str = include_str!("../../argo-cli/src/handlers/gateway/routes_channels.rs");
const ROUTE_DISCORD: &str =
    include_str!("../../argo-cli/src/handlers/gateway/routes_channels_discord.rs");
const ROUTE_EMAIL: &str =
    include_str!("../../argo-cli/src/handlers/gateway/routes_channels_email.rs");
const ROUTE_IMESSAGE: &str =
    include_str!("../../argo-cli/src/handlers/gateway/routes_channels_imessage.rs");
const ROUTE_PUSH: &str =
    include_str!("../../argo-cli/src/handlers/gateway/routes_channels_push.rs");
const ROUTE_SLACK: &str =
    include_str!("../../argo-cli/src/handlers/gateway/routes_channels_slack.rs");
const ROUTE_SMS: &str = include_str!("../../argo-cli/src/handlers/gateway/routes_channels_sms.rs");
const ROUTE_VOICE: &str =
    include_str!("../../argo-cli/src/handlers/gateway/routes_channels_voice.rs");
const ROUTE_WEBINBOX: &str =
    include_str!("../../argo-cli/src/handlers/gateway/routes_channels_webinbox.rs");
const ROUTE_WHATSAPP: &str =
    include_str!("../../argo-cli/src/handlers/gateway/routes_channels_whatsapp.rs");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Coverage {
    id: &'static str,
    outbound_arm: bool,
    inbound_route: bool,
    token_handler: bool,
    decision_sink: bool,
}

const COVERAGE: &[Coverage] = &[
    Coverage::new("discord", true, true, true, true),
    Coverage::new("email", true, true, true, true),
    Coverage::new("imessage", true, true, true, true),
    Coverage::new("matrix", false, false, false, false),
    Coverage::new("push", true, true, true, true),
    Coverage::new("slack", true, true, true, true),
    Coverage::new("sms", true, true, true, true),
    Coverage::new("teams", false, false, false, false),
    Coverage::new("voice", true, true, true, true),
    Coverage::new("webinbox", true, true, true, true),
    Coverage::new("whatsapp", false, true, true, true),
];

impl Coverage {
    const fn new(
        id: &'static str,
        outbound_arm: bool,
        inbound_route: bool,
        token_handler: bool,
        decision_sink: bool,
    ) -> Self {
        Self {
            id,
            outbound_arm,
            inbound_route,
            token_handler,
            decision_sink,
        }
    }
}

#[test]
fn inbound_coverage_matrix_matches_current_source() {
    for row in COVERAGE {
        assert_eq!(
            has_outbound_arm(row.id),
            row.outbound_arm,
            "{} outbound EnvChannelSender arm drifted",
            row.id
        );
        assert_eq!(
            has_inbound_route(row.id),
            row.inbound_route,
            "{} gateway inbound route drifted",
            row.id
        );
        assert_eq!(
            has_token_handler(row.id),
            row.token_handler,
            "{} token/reply handler drifted",
            row.id
        );
        assert_eq!(
            has_decision_sink(row.id),
            row.decision_sink,
            "{} approval decision sink drifted",
            row.id
        );
    }
}

#[test]
fn route_and_sink_counts_pin_the_actual_gateway_surface() {
    assert_eq!(count_where(|row| row.outbound_arm), 8);
    assert_eq!(count_where(|row| row.inbound_route), 9);
    assert_eq!(count_where(|row| row.token_handler), 9);
    assert_eq!(count_where(|row| row.decision_sink), 9);
}

#[test]
fn no_outbound_channel_lacks_inbound_settlement() {
    let gaps = outbound_without_inbound();
    assert!(
        gaps.is_empty(),
        "outbound channels lacking inbound settlement: {gaps:?}",
    );
}

fn count_where(predicate: impl Fn(&Coverage) -> bool) -> usize {
    COVERAGE.iter().filter(|row| predicate(row)).count()
}

fn outbound_without_inbound() -> Vec<&'static str> {
    COVERAGE
        .iter()
        .filter(|row| row.outbound_arm && !row.decision_sink)
        .map(|row| row.id)
        .collect()
}

fn has_outbound_arm(id: &str) -> bool {
    NOTIFY_ENV.contains(&format!("\"{id}\" =>"))
}

fn has_inbound_route(id: &str) -> bool {
    match route_source(id) {
        Some(_) if id == "email" => ROUTES.contains("POST /channels/email/webhook"),
        Some(_) => ROUTES.contains(&format!("is_{id}")) || ROUTES.contains(id),
        None => false,
    }
}

fn has_token_handler(id: &str) -> bool {
    route_source(id).is_some_and(|src| {
        src.contains("verify_token(")
            || src.contains("verify_voice_token(")
            || src.contains("token_from_url(")
    })
}

fn has_decision_sink(id: &str) -> bool {
    route_source(id).is_some_and(|src| {
        src.contains("settle(")
            || src.contains("settle_voice(")
            || src.contains("worker_actions_transitions::approve")
    })
}

fn route_source(id: &str) -> Option<&'static str> {
    match id {
        "discord" => Some(ROUTE_DISCORD),
        "email" => Some(ROUTE_EMAIL),
        "imessage" => Some(ROUTE_IMESSAGE),
        "push" => Some(ROUTE_PUSH),
        "slack" => Some(ROUTE_SLACK),
        "sms" => Some(ROUTE_SMS),
        "voice" => Some(ROUTE_VOICE),
        "webinbox" => Some(ROUTE_WEBINBOX),
        "whatsapp" => Some(ROUTE_WHATSAPP),
        _ => None,
    }
}
