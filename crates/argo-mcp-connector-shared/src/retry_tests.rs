//! Unit tests for `retry`.
//!
//! Held under `cfg(test)` so they don't bloat the public crate surface and
//! so dev-only helpers can live alongside without `#[allow(dead_code)]`.
use super::retry::{
    backoff_delay_ms, default_http_policy, is_http_retriable, is_tonic_retriable,
    retry_with_policy, RetryPolicy,
};
use argo_mcp::McpError;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

fn fast_policy() -> RetryPolicy {
    RetryPolicy {
        max_attempts: 3,
        initial_delay_ms: 1,
        multiplier: 2.0,
        max_delay_ms: 4,
        jitter_ms: 0,
    }
}

fn retriable_err() -> McpError {
    McpError::Rpc {
        code: 503,
        message: "transient".into(),
        data: None,
    }
}

fn non_retriable_err() -> McpError {
    McpError::Rpc {
        code: 400,
        message: "client error".into(),
        data: None,
    }
}

#[tokio::test]
async fn succeeds_first_try_no_retry() {
    let calls = Arc::new(AtomicU32::new(0));
    let calls2 = calls.clone();
    let result: Result<&'static str, McpError> =
        retry_with_policy(&fast_policy(), is_http_retriable, move || {
            let calls = calls2.clone();
            async move {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok("ok")
            }
        })
        .await;
    assert_eq!(result.unwrap(), "ok");
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn retries_on_retriable_then_succeeds() {
    let calls = Arc::new(AtomicU32::new(0));
    let calls2 = calls.clone();
    let result: Result<&'static str, McpError> =
        retry_with_policy(&fast_policy(), is_http_retriable, move || {
            let calls = calls2.clone();
            async move {
                let n = calls.fetch_add(1, Ordering::SeqCst) + 1;
                if n < 3 {
                    Err(retriable_err())
                } else {
                    Ok("ok")
                }
            }
        })
        .await;
    assert_eq!(result.unwrap(), "ok");
    assert_eq!(calls.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn gives_up_after_max_attempts() {
    let calls = Arc::new(AtomicU32::new(0));
    let calls2 = calls.clone();
    let result: Result<(), McpError> =
        retry_with_policy(&fast_policy(), is_http_retriable, move || {
            let calls = calls2.clone();
            async move {
                calls.fetch_add(1, Ordering::SeqCst);
                Err(retriable_err())
            }
        })
        .await;
    assert!(result.is_err());
    assert_eq!(calls.load(Ordering::SeqCst), fast_policy().max_attempts);
}

#[test]
fn exponential_backoff_respected() {
    let policy = RetryPolicy {
        max_attempts: 5,
        initial_delay_ms: 100,
        multiplier: 2.0,
        max_delay_ms: 10_000,
        jitter_ms: 0,
    };
    assert_eq!(backoff_delay_ms(&policy, 1, 0), 100);
    assert_eq!(backoff_delay_ms(&policy, 2, 0), 200);
    assert_eq!(backoff_delay_ms(&policy, 3, 0), 400);
    assert_eq!(backoff_delay_ms(&policy, 4, 0), 800);
}

#[test]
fn jitter_applied_within_bounds() {
    let policy = RetryPolicy {
        max_attempts: 3,
        initial_delay_ms: 100,
        multiplier: 2.0,
        max_delay_ms: 10_000,
        jitter_ms: 50,
    };
    for jitter in [0u64, 17, 49, 50, 99, 1000] {
        let d = backoff_delay_ms(&policy, 1, jitter);
        assert!((100..150).contains(&d), "delay {d} out of bounds for jitter {jitter}");
    }
    assert_ne!(
        backoff_delay_ms(&policy, 1, 0),
        backoff_delay_ms(&policy, 1, 33),
        "jitter must change the resulting delay"
    );
}

#[test]
fn max_delay_capped() {
    let policy = RetryPolicy {
        max_attempts: 10,
        initial_delay_ms: 250,
        multiplier: 2.0,
        max_delay_ms: 5_000,
        jitter_ms: 0,
    };
    assert_eq!(backoff_delay_ms(&policy, 10, 0), 5_000);
    assert_eq!(backoff_delay_ms(&policy, 30, 0), 5_000);
}

#[tokio::test]
async fn non_retriable_fails_immediately() {
    let calls = Arc::new(AtomicU32::new(0));
    let calls2 = calls.clone();
    let result: Result<(), McpError> =
        retry_with_policy(&fast_policy(), is_http_retriable, move || {
            let calls = calls2.clone();
            async move {
                calls.fetch_add(1, Ordering::SeqCst);
                Err(non_retriable_err())
            }
        })
        .await;
    assert!(result.is_err());
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "non-retriable error must not retry"
    );
}

#[tokio::test]
async fn concurrent_retries_no_collision() {
    let mut handles = Vec::new();
    for task in 0..8u32 {
        handles.push(tokio::spawn(async move {
            let calls = Arc::new(AtomicU32::new(0));
            let calls2 = calls.clone();
            let result: Result<u32, McpError> =
                retry_with_policy(&fast_policy(), is_http_retriable, move || {
                    let calls = calls2.clone();
                    async move {
                        let n = calls.fetch_add(1, Ordering::SeqCst) + 1;
                        if n < 2 {
                            Err(retriable_err())
                        } else {
                            Ok(task)
                        }
                    }
                })
                .await;
            (result.unwrap(), calls.load(Ordering::SeqCst))
        }));
    }
    for (i, h) in handles.into_iter().enumerate() {
        let (got_task, attempts) = h.await.unwrap();
        assert_eq!(got_task as usize, i, "each retry must keep its own task id");
        assert_eq!(attempts, 2, "each retry must use its own attempt counter");
    }
}

#[test]
fn http_classifier_axes() {
    assert!(is_http_retriable(&McpError::Io("conn reset".into())));
    assert!(is_http_retriable(&McpError::Timeout(5000)));
    assert!(is_http_retriable(&McpError::Rpc {
        code: 503,
        message: "x".into(),
        data: None
    }));
    assert!(is_http_retriable(&McpError::Rpc {
        code: 429,
        message: "x".into(),
        data: None
    }));
    assert!(!is_http_retriable(&McpError::Rpc {
        code: 400,
        message: "x".into(),
        data: None
    }));
    assert!(!is_http_retriable(&McpError::Rpc {
        code: 404,
        message: "x".into(),
        data: None
    }));
    assert!(!is_http_retriable(&McpError::Decode("bad json".into())));
}

#[test]
fn tonic_classifier_axes() {
    assert!(is_tonic_retriable(&McpError::Rpc {
        code: 14,
        message: "Unavailable".into(),
        data: None
    }));
    assert!(is_tonic_retriable(&McpError::Rpc {
        code: 4,
        message: "DeadlineExceeded".into(),
        data: None
    }));
    assert!(is_tonic_retriable(&McpError::Rpc {
        code: 8,
        message: "ResourceExhausted".into(),
        data: None
    }));
    assert!(!is_tonic_retriable(&McpError::Rpc {
        code: 3,
        message: "InvalidArgument".into(),
        data: None
    }));
}

#[test]
fn defaults_are_documented() {
    let p = default_http_policy();
    assert_eq!(p.max_attempts, 3);
    assert_eq!(p.initial_delay_ms, 250);
    assert!((p.multiplier - 2.0).abs() < f64::EPSILON);
    assert_eq!(p.max_delay_ms, 5_000);
    assert_eq!(p.jitter_ms, 100);
}
