use super::{AllowDecision, Tier, TierConfig};
use std::str::FromStr;

#[test]
fn default_config_carries_all_three_tiers() {
    let cfg = TierConfig::default_config();
    for tier in [Tier::Free, Tier::Pro, Tier::Enterprise] {
        assert!(cfg.limits(tier).is_some(), "missing limits for {tier:?}");
    }
}

#[test]
fn free_tier_has_documented_quotas() {
    let cfg = TierConfig::default_config();
    let free = cfg.limits(Tier::Free).expect("free");
    assert_eq!(free.max_workers, 1);
    assert_eq!(free.max_actions_per_month, 100);
    assert!(free.features.contains(&"provider_mock".to_string()));
}

#[test]
fn pro_tier_includes_marketplace_skills() {
    let cfg = TierConfig::default_config();
    let pro = cfg.limits(Tier::Pro).expect("pro");
    assert_eq!(pro.price_monthly_usd, 29);
    assert!(pro.features.contains(&"marketplace_skills".to_string()));
}

#[test]
fn enterprise_tier_is_unlimited() {
    let cfg = TierConfig::default_config();
    let ent = cfg.limits(Tier::Enterprise).expect("enterprise");
    assert_eq!(ent.max_workers, 0);
    assert_eq!(ent.max_actions_per_month, 0);
    assert!(ent.features.contains(&"sso".to_string()));
}

#[test]
fn allow_action_under_quota() {
    let cfg = TierConfig::default_config();
    let limits = cfg.limits(Tier::Pro).expect("pro");
    assert_eq!(Tier::Pro.allow_action("agent.run", 10, limits), AllowDecision::Allow);
}

#[test]
fn allow_action_blocks_when_cap_reached() {
    let cfg = TierConfig::default_config();
    let limits = cfg.limits(Tier::Free).expect("free");
    match Tier::Free.allow_action("agent.run", 100, limits) {
        AllowDecision::Deny(reason) => {
            assert!(reason.contains("monthly action cap"), "{reason}");
            assert!(reason.contains("100/100"), "{reason}");
        }
        AllowDecision::Allow => panic!("expected deny at cap"),
    }
}

#[test]
fn allow_action_blocks_feature_not_in_tier() {
    let cfg = TierConfig::default_config();
    let limits = cfg.limits(Tier::Free).expect("free");
    match Tier::Free.allow_action("feature:sso", 0, limits) {
        AllowDecision::Deny(reason) => assert!(reason.contains("sso"), "{reason}"),
        AllowDecision::Allow => panic!("expected deny on missing feature"),
    }
}

#[test]
fn allow_action_passes_feature_in_tier() {
    let cfg = TierConfig::default_config();
    let limits = cfg.limits(Tier::Enterprise).expect("enterprise");
    assert_eq!(
        Tier::Enterprise.allow_action("feature:sso", 0, limits),
        AllowDecision::Allow
    );
}

#[test]
fn enterprise_unlimited_passes_at_any_usage() {
    let cfg = TierConfig::default_config();
    let limits = cfg.limits(Tier::Enterprise).expect("enterprise");
    assert_eq!(
        Tier::Enterprise.allow_action("agent.run", 1_000_000, limits),
        AllowDecision::Allow
    );
}

#[test]
fn tier_parses_from_str() {
    assert_eq!(Tier::from_str("free"), Ok(Tier::Free));
    assert_eq!(Tier::from_str("pro"), Ok(Tier::Pro));
    assert_eq!(Tier::from_str("enterprise"), Ok(Tier::Enterprise));
    assert!(Tier::from_str("platinum").is_err());
}

#[test]
fn default_tier_is_free() {
    assert_eq!(Tier::default_tier(), Tier::Free);
}
