//! Unit tests for [`crate::operator_risk_overrides`].

use super::*;
use crate::RiskLevel;
use std::sync::Arc;

fn op(id: &str) -> OperatorId {
    OperatorId::new(id)
}

struct FixedDefault(RiskLevel);

#[async_trait]
impl DefaultRiskPolicy for FixedDefault {
    async fn classify(&self, _: &str, _: &str) -> ChannelResult<RiskLevel> {
        Ok(self.0)
    }
}

fn overrides_for(
    id: &str,
    entries: Vec<(&str, Option<&str>, RiskLevel)>,
    allow_downgrade: bool,
) -> OperatorRiskOverrides {
    let mut map = HashMap::new();
    for (kind, vert, risk) in entries {
        map.insert((kind.to_string(), vert.map(String::from)), risk);
    }
    OperatorRiskOverrides {
        operator_id: op(id),
        overrides: map,
        allow_downgrade_below_default: allow_downgrade,
    }
}

fn resolver_with(
    default: RiskLevel,
    overrides: OperatorRiskOverrides,
) -> RiskResolver {
    let source = InMemoryOperatorRiskOverridesSource::new();
    source.insert(overrides);
    RiskResolver::new(Arc::new(source), Arc::new(FixedDefault(default)))
}

#[tokio::test]
async fn no_override_returns_default() {
    let resolver = RiskResolver::new(
        Arc::new(InMemoryOperatorRiskOverridesSource::new()),
        Arc::new(FixedDefault(RiskLevel::Medium)),
    );
    let (risk, trace) = resolver
        .resolve(&op("alice"), "book_travel", "support")
        .await
        .expect("resolve");
    assert_eq!(risk, RiskLevel::Medium);
    assert_eq!(trace.default, RiskLevel::Medium);
    assert_eq!(trace.override_applied, None);
    assert_eq!(trace.override_scope, None);
    assert_eq!(trace.downgrade_blocked, None);
}

#[tokio::test]
async fn per_vertical_override_applied() {
    let resolver = resolver_with(
        RiskLevel::Low,
        overrides_for(
            "alice",
            vec![("send_email", Some("legal"), RiskLevel::High)],
            false,
        ),
    );
    let (risk, trace) = resolver
        .resolve(&op("alice"), "send_email", "legal")
        .await
        .expect("resolve");
    assert_eq!(risk, RiskLevel::High);
    assert_eq!(trace.override_applied, Some(RiskLevel::High));
    assert_eq!(
        trace.override_scope,
        Some(OverrideScope::PerVertical("legal".to_string()))
    );
}

#[tokio::test]
async fn all_verticals_override_applied() {
    let resolver = resolver_with(
        RiskLevel::Low,
        overrides_for(
            "alice",
            vec![("send_email_external", None, RiskLevel::High)],
            false,
        ),
    );
    let (risk, trace) = resolver
        .resolve(&op("alice"), "send_email_external", "support")
        .await
        .expect("resolve");
    assert_eq!(risk, RiskLevel::High);
    assert_eq!(trace.override_applied, Some(RiskLevel::High));
    assert_eq!(trace.override_scope, Some(OverrideScope::AllVerticals));
}

#[tokio::test]
async fn per_vertical_wins_over_all_verticals() {
    let resolver = resolver_with(
        RiskLevel::Low,
        overrides_for(
            "alice",
            vec![
                ("book_travel", None, RiskLevel::High),
                ("book_travel", Some("legal"), RiskLevel::Medium),
            ],
            false,
        ),
    );
    let (risk, trace) = resolver
        .resolve(&op("alice"), "book_travel", "legal")
        .await
        .expect("resolve");
    assert_eq!(risk, RiskLevel::Medium);
    assert_eq!(trace.override_applied, Some(RiskLevel::Medium));
    assert_eq!(
        trace.override_scope,
        Some(OverrideScope::PerVertical("legal".to_string()))
    );
    // And the all-verticals override still applies when the
    // vertical does NOT match the per-vertical scope.
    let (risk, trace) = resolver
        .resolve(&op("alice"), "book_travel", "support")
        .await
        .expect("resolve");
    assert_eq!(risk, RiskLevel::High);
    assert_eq!(trace.override_scope, Some(OverrideScope::AllVerticals));
}

#[tokio::test]
async fn downgrade_blocked_without_allow_downgrade() {
    let resolver = resolver_with(
        RiskLevel::Medium,
        overrides_for(
            "alice",
            vec![("book_travel", None, RiskLevel::Low)],
            false,
        ),
    );
    let (risk, trace) = resolver
        .resolve(&op("alice"), "book_travel", "support")
        .await
        .expect("resolve");
    assert_eq!(risk, RiskLevel::Medium);
    assert_eq!(trace.override_applied, None);
    assert_eq!(trace.override_scope, None);
    assert_eq!(trace.downgrade_blocked, Some(RiskLevel::Low));
}

#[tokio::test]
async fn downgrade_allowed_with_allow_downgrade() {
    let resolver = resolver_with(
        RiskLevel::Medium,
        overrides_for(
            "alice",
            vec![("book_travel", None, RiskLevel::Low)],
            true,
        ),
    );
    let (risk, trace) = resolver
        .resolve(&op("alice"), "book_travel", "support")
        .await
        .expect("resolve");
    assert_eq!(risk, RiskLevel::Low);
    assert_eq!(trace.override_applied, Some(RiskLevel::Low));
    assert_eq!(trace.downgrade_blocked, None);
}

#[tokio::test]
async fn upgrade_always_allowed_even_without_allow_downgrade() {
    // Paranoia is free.
    let resolver = resolver_with(
        RiskLevel::Low,
        overrides_for(
            "alice",
            vec![("send_email", None, RiskLevel::High)],
            false,
        ),
    );
    let (risk, trace) = resolver
        .resolve(&op("alice"), "send_email", "support")
        .await
        .expect("resolve");
    assert_eq!(risk, RiskLevel::High);
    assert_eq!(trace.override_applied, Some(RiskLevel::High));
    assert_eq!(trace.downgrade_blocked, None);
}

#[tokio::test]
async fn trace_shape_when_override_applied() {
    let resolver = resolver_with(
        RiskLevel::Medium,
        overrides_for(
            "alice",
            vec![("send_email", Some("support"), RiskLevel::High)],
            false,
        ),
    );
    let (_risk, trace) = resolver
        .resolve(&op("alice"), "send_email", "support")
        .await
        .expect("resolve");
    let body = serde_json::to_value(&trace).expect("serialize");
    assert_eq!(body["default"], "medium");
    assert_eq!(body["override_applied"], "high");
    assert_eq!(body["override_scope"]["PerVertical"], "support");
    assert!(body["downgrade_blocked"].is_null());
}

#[tokio::test]
async fn trace_shape_when_downgrade_blocked() {
    let resolver = resolver_with(
        RiskLevel::High,
        overrides_for(
            "alice",
            vec![("book_travel", None, RiskLevel::Low)],
            false,
        ),
    );
    let (_risk, trace) = resolver
        .resolve(&op("alice"), "book_travel", "support")
        .await
        .expect("resolve");
    let body = serde_json::to_value(&trace).expect("serialize");
    assert_eq!(body["default"], "high");
    assert!(body["override_applied"].is_null());
    assert!(body["override_scope"].is_null());
    assert_eq!(body["downgrade_blocked"], "low");
}

#[tokio::test]
async fn toml_source_parses_and_reloads_on_mtime_change() {
    use std::io::Write;
    use std::time::Duration;
    use tempfile::NamedTempFile;

    let mut tmp = NamedTempFile::new().expect("temp");
    writeln!(
        tmp,
        r#"
[operator.alice.risk_overrides]
allow_downgrade_below_default = false
entries = [
    {{ action_kind = "send_email", risk_level = "high" }},
    {{ action_kind = "book_travel", risk_level = "medium", vertical_id = "legal" }},
]
"#
    )
    .expect("write");
    let path = tmp.path().to_path_buf();
    let source = TomlOperatorRiskOverridesSource::new(&path);

    let loaded = source.for_operator(&op("alice")).await.expect("first read");
    assert_eq!(loaded.overrides.len(), 2);
    assert_eq!(
        loaded.overrides.get(&("send_email".to_string(), None)),
        Some(&RiskLevel::High),
    );
    assert_eq!(
        loaded
            .overrides
            .get(&("book_travel".to_string(), Some("legal".to_string()))),
        Some(&RiskLevel::Medium),
    );

    // Missing operator returns an empty record, not an error.
    let empty = source.for_operator(&op("bob")).await.expect("empty read");
    assert!(empty.overrides.is_empty());

    // Bump mtime so the cache invalidates, then rewrite with a
    // different value.
    std::thread::sleep(Duration::from_millis(1100));
    std::fs::write(
        &path,
        r#"
[operator.alice.risk_overrides]
allow_downgrade_below_default = true
entries = [
    { action_kind = "send_email", risk_level = "low" },
]
"#,
    )
    .expect("rewrite");
    let reloaded = source
        .for_operator(&op("alice"))
        .await
        .expect("reloaded read");
    assert!(reloaded.allow_downgrade_below_default);
    assert_eq!(
        reloaded.overrides.get(&("send_email".to_string(), None)),
        Some(&RiskLevel::Low),
    );
}
