//! Unit tests for [`crate::operator_preferences`].

use super::*;
use crate::{ChannelId, RiskLevel};
use chrono::{NaiveTime, TimeZone, Utc};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration as StdDuration;
use tempfile::tempdir;

fn ch(id: &str) -> ChannelId {
    ChannelId::new(id)
}

fn op(id: &str) -> OperatorId {
    OperatorId::new(id)
}

fn prefs(
    id: &str,
    primary: &str,
    chain: Vec<(&str, u32)>,
    quiet: Option<OperatorQuietHours>,
    overrides: Vec<(RiskLevel, &str)>,
) -> OperatorPreferences {
    OperatorPreferences {
        operator_id: op(id),
        primary_channel: ch(primary),
        fallback_chain: chain
            .into_iter()
            .map(|(c, s)| ChannelChainStep {
                channel: ch(c),
                after_seconds: s,
            })
            .collect(),
        quiet_hours: quiet,
        channel_overrides: overrides.into_iter().map(|(r, c)| (r, ch(c))).collect(),
    }
}

fn source_with(prefs_in: OperatorPreferences) -> Arc<InMemoryPreferencesSource> {
    let s = InMemoryPreferencesSource::new();
    s.insert(prefs_in);
    Arc::new(s)
}

#[tokio::test]
async fn primary_no_quiet_returns_primary() {
    let p = prefs("a", "slack", vec![("email", 600)], None, vec![]);
    let resolver = OperatorPreferencesResolver::new(source_with(p));
    let now = Utc.with_ymd_and_hms(2026, 5, 11, 14, 0, 0).unwrap();
    let picked = resolver
        .resolve_initial(&op("a"), RiskLevel::Medium, now)
        .await
        .expect("resolve");
    assert_eq!(picked, ch("slack"));
}

#[tokio::test]
async fn quiet_honored_walks_to_next_chain_step() {
    let mut honor = HashSet::new();
    honor.insert(ch("slack"));
    let quiet = OperatorQuietHours {
        start_local: NaiveTime::from_hms_opt(22, 0, 0).unwrap(),
        end_local: NaiveTime::from_hms_opt(7, 0, 0).unwrap(),
        timezone: chrono_tz::UTC,
        honor_for: honor,
    };
    let p = prefs(
        "a",
        "slack",
        vec![("email", 60), ("voice", 600)],
        Some(quiet),
        vec![],
    );
    let resolver = OperatorPreferencesResolver::new(source_with(p));
    // 23:30 UTC is inside the quiet window.
    let now = Utc.with_ymd_and_hms(2026, 5, 11, 23, 30, 0).unwrap();
    let picked = resolver
        .resolve_initial(&op("a"), RiskLevel::Medium, now)
        .await
        .expect("resolve");
    assert_eq!(picked, ch("email"));
}

#[tokio::test]
async fn quiet_not_honored_by_channel_returns_primary() {
    // Quiet window matches but honor_for is empty, so slack rings.
    let quiet = OperatorQuietHours {
        start_local: NaiveTime::from_hms_opt(22, 0, 0).unwrap(),
        end_local: NaiveTime::from_hms_opt(7, 0, 0).unwrap(),
        timezone: chrono_tz::UTC,
        honor_for: HashSet::new(),
    };
    let p = prefs("a", "slack", vec![("email", 60)], Some(quiet), vec![]);
    let resolver = OperatorPreferencesResolver::new(source_with(p));
    let now = Utc.with_ymd_and_hms(2026, 5, 11, 23, 30, 0).unwrap();
    let picked = resolver
        .resolve_initial(&op("a"), RiskLevel::Medium, now)
        .await
        .expect("resolve");
    assert_eq!(picked, ch("slack"));
}

#[tokio::test]
async fn risk_override_trumps_primary() {
    let p = prefs(
        "a",
        "slack",
        vec![("email", 60)],
        None,
        vec![(RiskLevel::High, "voice")],
    );
    let resolver = OperatorPreferencesResolver::new(source_with(p));
    let now = Utc.with_ymd_and_hms(2026, 5, 11, 14, 0, 0).unwrap();
    let picked = resolver
        .resolve_initial(&op("a"), RiskLevel::High, now)
        .await
        .expect("resolve");
    assert_eq!(picked, ch("voice"));
    // medium has no override, falls through to primary
    let picked_med = resolver
        .resolve_initial(&op("a"), RiskLevel::Medium, now)
        .await
        .expect("resolve");
    assert_eq!(picked_med, ch("slack"));
}

#[tokio::test]
async fn next_fallback_returns_next_after_elapsed() {
    let p = prefs(
        "a",
        "slack",
        vec![("email", 60), ("voice", 600)],
        None,
        vec![],
    );
    let resolver = OperatorPreferencesResolver::new(source_with(p));
    let now = Utc.with_ymd_and_hms(2026, 5, 11, 14, 0, 0).unwrap();
    let next = resolver
        .next_fallback(&op("a"), ch("slack"), StdDuration::from_secs(60), now)
        .await
        .expect("next");
    assert_eq!(next, Some(ch("email")));
    let next2 = resolver
        .next_fallback(&op("a"), ch("email"), StdDuration::from_secs(600), now)
        .await
        .expect("next");
    assert_eq!(next2, Some(ch("voice")));
    // Elapsed below the next step threshold yields None.
    let none = resolver
        .next_fallback(&op("a"), ch("slack"), StdDuration::from_secs(30), now)
        .await
        .expect("next");
    assert!(none.is_none());
}

#[tokio::test]
async fn next_fallback_exhausted_returns_none() {
    let p = prefs("a", "slack", vec![("email", 60)], None, vec![]);
    let resolver = OperatorPreferencesResolver::new(source_with(p));
    let now = Utc.with_ymd_and_hms(2026, 5, 11, 14, 0, 0).unwrap();
    let exhausted = resolver
        .next_fallback(&op("a"), ch("email"), StdDuration::from_secs(3600), now)
        .await
        .expect("next");
    assert!(exhausted.is_none());
}

#[tokio::test]
async fn timezone_correctly_applied() {
    let mut honor = HashSet::new();
    honor.insert(ch("slack"));
    let quiet = OperatorQuietHours {
        start_local: NaiveTime::from_hms_opt(22, 0, 0).unwrap(),
        end_local: NaiveTime::from_hms_opt(7, 0, 0).unwrap(),
        // America/Los_Angeles: UTC-7 in May (PDT).
        timezone: chrono_tz::America::Los_Angeles,
        honor_for: honor,
    };
    let p = prefs("a", "slack", vec![("email", 60)], Some(quiet), vec![]);
    let resolver = OperatorPreferencesResolver::new(source_with(p));
    // 14:00 UTC = 07:00 LA — just outside quiet window's end (07:00).
    let day_utc = Utc.with_ymd_and_hms(2026, 5, 11, 14, 0, 0).unwrap();
    let picked = resolver
        .resolve_initial(&op("a"), RiskLevel::Medium, day_utc)
        .await
        .expect("resolve");
    assert_eq!(picked, ch("slack"), "07:00 LA is outside quiet window");
    // 13:00 UTC = 06:00 LA — inside the overnight window, walks to email.
    let night_utc = Utc.with_ymd_and_hms(2026, 5, 11, 13, 0, 0).unwrap();
    let picked2 = resolver
        .resolve_initial(&op("a"), RiskLevel::Medium, night_utc)
        .await
        .expect("resolve");
    assert_eq!(picked2, ch("email"), "06:00 LA is inside quiet window");
}

#[tokio::test]
async fn mtime_reload_picks_up_changes() {
    let tmp = tempdir().expect("tempdir");
    let path = tmp.path().join("argo.toml");
    let initial = r#"
[operator.alice.preferences]
primary_channel = "slack"
fallback_chain = []
channel_overrides = {}
"#;
    std::fs::write(&path, initial).expect("write");
    let source = TomlPreferencesSource::new(&path);
    let p1 = source.for_operator(&op("alice")).await.expect("first read");
    assert_eq!(p1.primary_channel, ch("slack"));

    // Bump mtime forward to guarantee a reload-trigger regardless
    // of filesystem timestamp granularity.
    let updated = r#"
[operator.alice.preferences]
primary_channel = "email"
fallback_chain = []
channel_overrides = {}
"#;
    std::fs::write(&path, updated).expect("rewrite");
    let new_mtime = std::time::SystemTime::now() + std::time::Duration::from_secs(2);
    let f = std::fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .expect("open for mtime");
    f.set_modified(new_mtime).expect("bump mtime");
    drop(f);

    let p2 = source.for_operator(&op("alice")).await.expect("second read");
    assert_eq!(p2.primary_channel, ch("email"));
}

#[tokio::test]
async fn file_missing_returns_typed_error() {
    let tmp = tempdir().expect("tempdir");
    let missing = tmp.path().join("does-not-exist.toml");
    let source = TomlPreferencesSource::new(&missing);
    let err = source
        .for_operator(&op("alice"))
        .await
        .expect_err("missing file should error");
    match err {
        ChannelError::Config(msg) => assert!(
            msg.contains("does-not-exist.toml"),
            "error should name the path: {msg}"
        ),
        other => panic!("expected Config error, got {other:?}"),
    }
}

#[tokio::test]
async fn unknown_operator_returns_typed_error() {
    let source = InMemoryPreferencesSource::new();
    let resolver = OperatorPreferencesResolver::new(Arc::new(source));
    let now = Utc::now();
    let err = resolver
        .resolve_initial(&op("unknown"), RiskLevel::Low, now)
        .await
        .expect_err("missing op");
    assert!(matches!(err, ChannelError::Config(_)));
}

#[test]
fn operator_preferences_round_trips_through_toml() {
    let tmp = tempdir().expect("tempdir");
    let path = tmp.path().join("argo.toml");
    let body = r#"
[operator.bob.preferences]
primary_channel = "signal"

[[operator.bob.preferences.fallback_chain]]
channel = "imessage"
after_seconds = 600

[operator.bob.preferences.quiet_hours]
start_local = "22:00"
end_local = "07:00"
timezone = "America/Los_Angeles"
honor_for = ["signal", "imessage"]

[operator.bob.preferences.channel_overrides]
high = "voice"
"#;
    std::fs::write(&path, body).expect("write");
    let source = TomlPreferencesSource::new(&path);
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    let prefs_out = rt
        .block_on(source.for_operator(&op("bob")))
        .expect("read");
    assert_eq!(prefs_out.primary_channel, ch("signal"));
    assert_eq!(prefs_out.fallback_chain.len(), 1);
    assert_eq!(prefs_out.fallback_chain[0].channel, ch("imessage"));
    assert_eq!(prefs_out.fallback_chain[0].after_seconds, 600);
    let q = prefs_out.quiet_hours.expect("quiet hours present");
    assert_eq!(q.start_local, NaiveTime::from_hms_opt(22, 0, 0).unwrap());
    assert_eq!(q.end_local, NaiveTime::from_hms_opt(7, 0, 0).unwrap());
    assert_eq!(q.timezone, chrono_tz::America::Los_Angeles);
    assert!(q.honor_for.contains(&ch("signal")));
    let high = prefs_out
        .channel_overrides
        .get(&RiskLevel::High)
        .expect("high override");
    assert_eq!(high, &ch("voice"));
}

#[test]
fn operator_preferences_omitted_blocks_dont_crash() {
    let map: HashMap<OperatorId, OperatorPreferences> = HashMap::new();
    let _src = InMemoryPreferencesSource::with_map(map);
}
