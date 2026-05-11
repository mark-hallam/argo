//! End-to-end smoke for the operator-preferences resolver:
//! TOML on disk → resolver → channel decision → mtime-reload picks
//! up the rewrite.

use argo_channels::operator_preferences::{
    OperatorId, OperatorPreferencesResolver, OperatorPreferencesSource,
    TomlPreferencesSource,
};
use argo_channels::{ChannelId, RiskLevel};
use chrono::{TimeZone, Utc};
use std::sync::Arc;
use std::time::Duration as StdDuration;
use tempfile::tempdir;

fn write_initial(path: &std::path::Path) {
    let body = r#"
[operator.alice.preferences]
primary_channel = "slack"

[[operator.alice.preferences.fallback_chain]]
channel = "email"
after_seconds = 60

[[operator.alice.preferences.fallback_chain]]
channel = "voice"
after_seconds = 600

[operator.alice.preferences.channel_overrides]
high = "voice"

[operator.bob.preferences]
primary_channel = "imessage"

[[operator.bob.preferences.fallback_chain]]
channel = "signal"
after_seconds = 90
"#;
    std::fs::write(path, body).expect("write initial");
}

fn write_alice_email_primary(path: &std::path::Path) {
    let body = r#"
[operator.alice.preferences]
primary_channel = "email"

[[operator.alice.preferences.fallback_chain]]
channel = "voice"
after_seconds = 300

[operator.alice.preferences.channel_overrides]
high = "voice"
"#;
    std::fs::write(path, body).expect("rewrite");
}

#[tokio::test]
async fn resolves_initial_and_fallback_then_reloads_on_mtime() {
    let tmp = tempdir().expect("tempdir");
    let path = tmp.path().join("argo.toml");
    write_initial(&path);

    let source: Arc<dyn OperatorPreferencesSource> =
        Arc::new(TomlPreferencesSource::new(&path));
    let resolver = OperatorPreferencesResolver::new(source);
    let now = Utc.with_ymd_and_hms(2026, 5, 11, 14, 0, 0).unwrap();

    // Alice's medium-risk routes to her primary, slack.
    let alice = OperatorId::new("alice");
    let picked = resolver
        .resolve_initial(&alice, RiskLevel::Medium, now)
        .await
        .expect("alice resolve");
    assert_eq!(picked, ChannelId::new("slack"));

    // Alice's high-risk override promotes voice.
    let picked_high = resolver
        .resolve_initial(&alice, RiskLevel::High, now)
        .await
        .expect("alice high");
    assert_eq!(picked_high, ChannelId::new("voice"));

    // Bob's primary is imessage; smoke the second operator entry.
    let bob = OperatorId::new("bob");
    let picked_bob = resolver
        .resolve_initial(&bob, RiskLevel::Low, now)
        .await
        .expect("bob resolve");
    assert_eq!(picked_bob, ChannelId::new("imessage"));

    // Walk Alice's chain: 60s elapsed → email, 600s elapsed → voice.
    let next1 = resolver
        .next_fallback(
            &alice,
            ChannelId::new("slack"),
            StdDuration::from_secs(60),
            now,
        )
        .await
        .expect("alice next");
    assert_eq!(next1, Some(ChannelId::new("email")));

    let next2 = resolver
        .next_fallback(
            &alice,
            ChannelId::new("email"),
            StdDuration::from_secs(600),
            now,
        )
        .await
        .expect("alice voice");
    assert_eq!(next2, Some(ChannelId::new("voice")));

    // Mutate the file and bump the mtime forward, then assert
    // the resolver picks up the new primary on the next call.
    write_alice_email_primary(&path);
    let new_mtime = std::time::SystemTime::now() + std::time::Duration::from_secs(2);
    let f = std::fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .expect("open for mtime");
    f.set_modified(new_mtime).expect("bump mtime");
    drop(f);

    let reloaded = resolver
        .resolve_initial(&alice, RiskLevel::Medium, now)
        .await
        .expect("alice reload");
    assert_eq!(reloaded, ChannelId::new("email"), "mtime reload picked up");
}
