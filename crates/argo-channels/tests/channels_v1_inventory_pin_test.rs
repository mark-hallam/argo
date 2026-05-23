//! Pin test for the v1.0 channel inventory.
//!
//! Companion to the report at
//! `docs/skycot/evals/scheduled/2026-05-23/eval-channels-v1-config-vs-code-inventory.md`.
//!
//! Asserts that the 11 channel ids enumerated by the
//! `eval-channels-v1-config-vs-code-inventory` audit remain present
//! in `argo-channels`. Construct each impl via its test-friendly
//! constructor (`disabled_for_tests`, `local`, or `new(config)` with
//! a hand-built test config) and confirm the `id()` matches the
//! expected stable string.
//!
//! If a channel is deleted from `argo-channels`, this file fails to
//! compile — forcing the audit and the
//! `mission-vs-code-gap-manifest.md` row to be re-validated rather
//! than silently drifting. If a channel is renamed, the
//! `EXPECTED_CHANNEL_IDS` assert fails, which also forces a manual
//! re-validation. Either way, the lock the test holds is:
//!
//! ```text
//! v1.0 ships exactly these 11 channel impls; any change to that
//! list is a deliberate edit that updates the eval report.
//! ```

use argo_channels::discord::{DiscordChannel, DiscordChannelConfig};
use argo_channels::email::{EmailChannel, EmailChannelConfig};
use argo_channels::imessage::{IMessageChannel, IMessageChannelConfig};
use argo_channels::matrix::{MatrixChannel, MatrixChannelConfig};
use argo_channels::push::{PushApprovalChannel, PushApprovalChannelConfig};
use argo_channels::slack::{SlackApprovalChannel, SlackApprovalChannelConfig};
use argo_channels::sms::{SmsApprovalChannel, SmsApprovalChannelConfig};
use argo_channels::teams::{TeamsApprovalChannel, TeamsApprovalChannelConfig};
use argo_channels::voice::{VoiceApprovalChannel, VoiceApprovalChannelConfig};
use argo_channels::webinbox::{WebInboxChannel, WebInboxChannelConfig};
use argo_channels::whatsapp::{WhatsAppApprovalChannel, WhatsAppApprovalChannelConfig};
use argo_channels::{ChannelId, NotifyChannel, RateLimitConfig};
use std::collections::{BTreeMap, BTreeSet};

/// The 11 channel ids the v1.0 audit pins. Order is the
/// alphabetical canonical order used in the eval report's matrix.
const EXPECTED_CHANNEL_IDS: &[&str] = &[
    "discord",
    "email",
    "imessage",
    "matrix",
    "push",
    "slack",
    "sms",
    "teams",
    "voice",
    "webinbox",
    "whatsapp",
];

/// Build one instance of every channel impl using the available
/// test constructors. The set of `id()` returns is the contract
/// pinned by this file.
fn instantiate_all_channels() -> Vec<Box<dyn NotifyChannel>> {
    let temp_dir = std::env::temp_dir();
    let webinbox_path = temp_dir.join("channels_v1_inventory_pin_test.jsonl");
    let push_registry = temp_dir.join("channels_v1_inventory_pin_test_push.jsonl");

    let imessage_config = IMessageChannelConfig::local("http://127.0.0.1:9");
    let imessage = IMessageChannel::new(imessage_config)
        .expect("IMessageChannel::new with local config must succeed");

    let slack_config = SlackApprovalChannelConfig {
        channel_id: ChannelId::new("slack"),
        bot_token: "test-token".into(),
        endpoint: "http://127.0.0.1:9/chat.postMessage".into(),
        operator_routes: BTreeMap::new(),
        rate_limit: RateLimitConfig::unlimited(),
        quiet_hours: None,
    };

    vec![
        Box::new(DiscordChannel::new(DiscordChannelConfig::disabled_for_tests("C1"))),
        Box::new(EmailChannel::new(EmailChannelConfig::disabled_for_tests())),
        Box::new(imessage),
        Box::new(MatrixChannel::new(MatrixChannelConfig::disabled_for_tests())),
        Box::new(PushApprovalChannel::new(PushApprovalChannelConfig::local(
            push_registry,
        ))),
        Box::new(SlackApprovalChannel::new(slack_config)),
        Box::new(SmsApprovalChannel::new(
            SmsApprovalChannelConfig::disabled_for_tests(BTreeMap::new()),
        )),
        Box::new(TeamsApprovalChannel::new(
            TeamsApprovalChannelConfig::disabled_for_tests(),
        )),
        Box::new(VoiceApprovalChannel::new(
            VoiceApprovalChannelConfig::disabled_for_tests(BTreeMap::new()),
        )),
        Box::new(WebInboxChannel::new(WebInboxChannelConfig::local(webinbox_path))),
        Box::new(WhatsAppApprovalChannel::new(
            WhatsAppApprovalChannelConfig::disabled_for_tests(BTreeMap::new()),
        )),
    ]
}

#[test]
fn v1_channel_inventory_is_exactly_eleven_ids() {
    let channels = instantiate_all_channels();
    assert_eq!(
        channels.len(),
        EXPECTED_CHANNEL_IDS.len(),
        "instantiated channel count must match EXPECTED_CHANNEL_IDS"
    );

    let actual: BTreeSet<String> = channels.iter().map(|c| c.id().to_string()).collect();
    let expected: BTreeSet<String> = EXPECTED_CHANNEL_IDS.iter().map(|s| s.to_string()).collect();

    assert_eq!(
        actual, expected,
        "channel-id set drifted from the v1.0 inventory audit. \
         If you added/removed/renamed a channel, update \
         docs/skycot/evals/scheduled/2026-05-23/eval-channels-v1-config-vs-code-inventory.md \
         and EXPECTED_CHANNEL_IDS in this file."
    );
}

#[test]
fn every_channel_id_is_lowercase_and_nonempty() {
    for c in instantiate_all_channels() {
        let id = c.id().to_string();
        assert!(!id.is_empty(), "channel returned empty id");
        assert_eq!(
            id,
            id.to_lowercase(),
            "channel id `{id}` must be lowercase (operators grep this string)"
        );
        assert!(
            id.chars().all(|ch| ch.is_ascii_lowercase() || ch == '_'),
            "channel id `{id}` must be ASCII lowercase (no spaces, dashes, or unicode)"
        );
    }
}

#[test]
fn channel_ids_are_unique() {
    let channels = instantiate_all_channels();
    let ids: Vec<String> = channels.iter().map(|c| c.id().to_string()).collect();
    let unique: BTreeSet<&String> = ids.iter().collect();
    assert_eq!(
        ids.len(),
        unique.len(),
        "duplicate channel id found in inventory: {ids:?}"
    );
}
