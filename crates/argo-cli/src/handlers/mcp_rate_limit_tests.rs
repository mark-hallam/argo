use super::*;

fn config_for(server: &str, capacity: f64, refill: f64) -> McpRateLimitConfig {
    let mut rate_limit = HashMap::new();
    rate_limit.insert(
        server.into(),
        BucketConfig {
            capacity,
            refill_rate_per_sec: refill,
        },
    );
    McpRateLimitConfig {
        default_capacity: 2.0,
        default_refill_rate_per_sec: 0.0,
        max_wait_ms: 20,
        rate_limit,
    }
}

#[test]
fn try_acquire_returns_true_while_tokens_available() {
    let bucket = TokenBucket::new(
        "gmail",
        BucketConfig {
            capacity: 2.0,
            refill_rate_per_sec: 0.0,
        },
    );
    assert!(bucket.try_acquire(1.0));
    assert!(bucket.try_acquire(1.0));
}

#[test]
fn try_acquire_returns_false_when_bucket_empty() {
    let bucket = TokenBucket::new(
        "gmail",
        BucketConfig {
            capacity: 1.0,
            refill_rate_per_sec: 0.0,
        },
    );
    assert!(bucket.try_acquire(1.0));
    assert!(!bucket.try_acquire(1.0));
}

#[tokio::test]
async fn acquire_with_timeout_waits_then_succeeds_when_bucket_refills() {
    let bucket = TokenBucket::new(
        "gmail",
        BucketConfig {
            capacity: 1.0,
            refill_rate_per_sec: 100.0,
        },
    );
    assert!(bucket.try_acquire(1.0));
    bucket
        .acquire_with_timeout(1.0, Duration::from_millis(50))
        .await
        .expect("refill should satisfy wait");
}

#[tokio::test]
async fn acquire_with_timeout_returns_rate_limit_error_after_timeout() {
    let bucket = TokenBucket::new(
        "gmail",
        BucketConfig {
            capacity: 0.0,
            refill_rate_per_sec: 0.0,
        },
    );
    let err = bucket
        .acquire_with_timeout(1.0, Duration::from_millis(5))
        .await
        .expect_err("zero-capacity bucket should time out");
    assert!(err.to_string().contains("rate limit exceeded for gmail"));
}

#[tokio::test]
async fn per_server_isolation_keeps_one_throttle_from_blocking_another() {
    let limiter = RateLimiter::from_config(&config_for("blocked", 0.0, 0.0));
    assert!(
        limiter
            .acquire_with_timeout("blocked", 1.0, Duration::from_millis(5))
            .await
            .is_err()
    );
    limiter
        .acquire_with_timeout("other", 1.0, Duration::from_millis(5))
        .await
        .expect("default bucket for other server should have tokens");
}

#[tokio::test]
async fn default_bucket_applies_to_unconfigured_servers() {
    let limiter = RateLimiter::from_config(&McpRateLimitConfig::default());
    limiter
        .acquire_with_timeout("unconfigured", 1.0, Duration::from_millis(5))
        .await
        .expect("default bucket starts full");
}

#[tokio::test]
async fn refill_rate_is_honored_after_elapsed_time() {
    let bucket = TokenBucket::new(
        "gmail",
        BucketConfig {
            capacity: 2.0,
            refill_rate_per_sec: 50.0,
        },
    );
    assert!(bucket.try_acquire(2.0));
    assert!(!bucket.try_acquire(1.0));
    tokio::time::sleep(Duration::from_millis(30)).await;
    assert!(bucket.try_acquire(1.0));
}

#[tokio::test]
async fn capacity_ceiling_prevents_overflow_after_long_idle() {
    let bucket = TokenBucket::new(
        "gmail",
        BucketConfig {
            capacity: 2.0,
            refill_rate_per_sec: 100.0,
        },
    );
    assert!(bucket.try_acquire(2.0));
    tokio::time::sleep(Duration::from_millis(40)).await;
    assert!(bucket.try_acquire(1.0));
    assert!(bucket.try_acquire(1.0));
    assert!(!bucket.try_acquire(1.0));
}

#[test]
fn config_loads_per_server_bucket() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("mcp_rate_limit.toml");
    std::fs::write(
        &path,
        r#"
default_capacity = 3
default_refill_rate_per_sec = 1
max_wait_ms = 7

[rate_limit.gmail]
capacity = 9
refill_rate_per_sec = 2
"#,
    )
    .expect("write config");
    let config = McpRateLimitConfig::load(&path).expect("parse config");
    assert_eq!(config.default_capacity, 3.0);
    assert_eq!(config.max_wait_ms, 7);
    assert_eq!(config.rate_limit["gmail"].capacity, 9.0);
}
