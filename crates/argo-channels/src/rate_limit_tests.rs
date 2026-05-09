use super::*;
use crate::ChannelError;
use std::time::Duration;

#[test]
fn unlimited_rate_limit_admits_every_acquire() {
    let limit = RateLimit::unlimited();
    for _ in 0..1_000 {
        limit.try_acquire().expect("unlimited never blocks");
    }
}

#[test]
fn slack_default_admits_first_five_then_blocks_with_retry_after() {
    let limit = RateLimit::from_config(RateLimitConfig::slack_default());
    for _ in 0..5 {
        limit.try_acquire().expect("first 5 must pass");
    }
    let wait = limit.try_acquire().expect_err("6th must rate-limit");
    assert!(wait > Duration::ZERO);
    assert!(
        wait <= Duration::from_secs(1),
        "per_sec=5 means refill within 1s",
    );
}

#[test]
fn override_partial_block_raises_only_the_supplied_window() {
    let toml_block = "per_sec = 20";
    let override_: RateLimitConfig = toml::from_str(toml_block).expect("parse override");
    let merged = RateLimitConfig::slack_default().with_override(override_);
    assert_eq!(merged.per_sec, Some(20));
    assert_eq!(merged.per_min, Some(60));
    assert_eq!(merged.per_hour, None);
    let limit = RateLimit::from_config(merged);
    for _ in 0..20 {
        limit.try_acquire().expect("override raises per_sec to 20");
    }
    let _err = limit.try_acquire().expect_err("21st must rate-limit");
}

#[test]
fn longest_window_drives_retry_after() {
    let cfg = RateLimitConfig::new(Some(5), Some(5), None);
    let limit = RateLimit::from_config(cfg);
    for _ in 0..5 {
        limit.try_acquire().expect("first 5 pass");
    }
    let wait = limit.try_acquire().expect_err("must rate-limit");
    assert!(
        wait >= Duration::from_secs(11),
        "per_min=5 needs ~12s to mint one token; got {wait:?}",
    );
}

#[test]
fn empty_config_with_zero_capacity_returns_long_wait() {
    let cfg = RateLimitConfig::new(Some(0), None, None);
    let limit = RateLimit::from_config(cfg);
    let wait = limit
        .try_acquire()
        .expect_err("zero-capacity bucket always rate-limits");
    assert!(wait >= Duration::from_secs(60));
}

#[tokio::test]
async fn token_bucket_refills_after_real_sleep() {
    let cfg = RateLimitConfig::new(Some(2), None, None);
    let limit = RateLimit::from_config(cfg);
    limit.try_acquire().expect("1st pass");
    limit.try_acquire().expect("2nd pass");
    let _err = limit.try_acquire().expect_err("3rd rate-limited");
    tokio::time::sleep(Duration::from_millis(600)).await;
    limit
        .try_acquire()
        .expect("after 600ms a per_sec=2 bucket has refilled at least one token");
}

#[test]
fn cloned_handles_share_bucket_state() {
    let limit = RateLimit::from_config(RateLimitConfig::new(Some(2), None, None));
    let clone = limit.clone();
    limit.try_acquire().expect("1");
    clone.try_acquire().expect("2");
    let _err = limit
        .try_acquire()
        .expect_err("clone must share the same exhausted bucket");
}

#[test]
fn unlimited_default_round_trips_through_toml() {
    let cfg: RateLimitConfig = toml::from_str("").expect("empty toml is default");
    assert_eq!(cfg, RateLimitConfig::unlimited());
}

#[test]
fn channel_error_rate_limited_carries_retry_after() {
    let err = ChannelError::RateLimited {
        retry_after: Duration::from_millis(750),
    };
    assert!(err.to_string().contains("750"));
}
