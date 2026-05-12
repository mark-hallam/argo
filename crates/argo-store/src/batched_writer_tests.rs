//! Unit tests for [`super::BatchedWriter`] — flush triggers,
//! shutdown semantics, env-knob respect, error propagation.
#![cfg(test)]
use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex as StdMutex;
use std::time::Duration;

/// Concurrent submits across `count` items. Returns the joined
/// per-submit results in submit order.
async fn submit_concurrent<T>(
    writer: Arc<BatchedWriter<T>>,
    items: Vec<T>,
) -> Vec<ArgoResult<()>>
where
    T: Send + 'static,
{
    let mut handles = Vec::with_capacity(items.len());
    for item in items {
        let w = writer.clone();
        handles.push(tokio::spawn(async move { w.submit(item).await }));
    }
    let mut out = Vec::with_capacity(handles.len());
    for h in handles {
        out.push(h.await.unwrap());
    }
    out
}

#[tokio::test]
async fn flush_on_size() {
    let flushes: Arc<StdMutex<Vec<usize>>> = Arc::new(StdMutex::new(Vec::new()));
    let captured = flushes.clone();
    let writer = Arc::new(BatchedWriter::new(
        move |batch: Vec<u32>| {
            let captured = captured.clone();
            async move {
                captured
                    .lock()
                    .map_err(|e| ArgoError::Provider(format!("test mutex: {e}")))?
                    .push(batch.len());
                Ok(())
            }
        },
        4,
        Duration::from_secs(60), // latency window long enough not to fire
    ));

    let results = submit_concurrent::<u32>(writer.clone(), (0..4).collect()).await;
    for r in results {
        r.unwrap();
    }

    let recorded = flushes.lock().unwrap();
    let total: usize = recorded.iter().sum();
    assert_eq!(total, 4, "all four items flushed");
    assert!(
        recorded.contains(&4),
        "size threshold should produce one batch of 4 (saw {:?})",
        *recorded
    );
}

#[tokio::test]
async fn flush_on_latency() {
    let count = Arc::new(AtomicUsize::new(0));
    let observed = count.clone();
    let writer = Arc::new(BatchedWriter::new(
        move |batch: Vec<u32>| {
            let observed = observed.clone();
            async move {
                observed.fetch_add(batch.len(), Ordering::SeqCst);
                Ok(())
            }
        },
        1024, // size won't trigger
        Duration::from_millis(10),
    ));

    // Two items in flight — they fit under the size threshold so
    // the latency window is what eventually drains them.
    let results = submit_concurrent::<u32>(writer.clone(), vec![1, 2]).await;
    for r in results {
        r.unwrap();
    }
    assert_eq!(
        count.load(Ordering::SeqCst),
        2,
        "latency window must drain submitted rows"
    );
}

#[tokio::test]
async fn flush_on_drop() {
    let flushed = Arc::new(AtomicUsize::new(0));
    let observed = flushed.clone();
    let writer = BatchedWriter::new(
        move |batch: Vec<u8>| {
            let observed = observed.clone();
            async move {
                observed.fetch_add(batch.len(), Ordering::SeqCst);
                Ok(())
            }
        },
        1024,
        Duration::from_secs(60),
    );
    writer.submit_fire_and_forget(7).unwrap();
    writer.submit_fire_and_forget(8).unwrap();
    writer.shutdown().await;
    assert_eq!(
        flushed.load(Ordering::SeqCst),
        2,
        "shutdown must drain pending rows"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn env_knob_respected() {
    let key_size = "ARGO_STORE_BATCH_SIZE";
    let key_lat = "ARGO_STORE_BATCH_LATENCY_MS";
    let key_opt = "ARGO_STORE_BATCHED_WRITES";
    // Snapshot prior values so the test leaves the env unchanged.
    let prior_size = std::env::var(key_size).ok();
    let prior_lat = std::env::var(key_lat).ok();
    let prior_opt = std::env::var(key_opt).ok();

    // SAFETY: this test is single-threaded (current_thread runtime)
    // and the env keys are scoped to this module's tests.
    unsafe { std::env::set_var(key_size, "5") };
    assert_eq!(batch_size_from_env(), 5);
    unsafe { std::env::set_var(key_size, "0") };
    assert_eq!(batch_size_from_env(), MAX_BATCH_SIZE, "zero falls back");
    unsafe { std::env::set_var(key_size, "garbage") };
    assert_eq!(batch_size_from_env(), MAX_BATCH_SIZE, "junk falls back");

    unsafe { std::env::set_var(key_lat, "42") };
    assert_eq!(batch_latency_from_env(), Duration::from_millis(42));

    unsafe { std::env::set_var(key_opt, "0") };
    assert!(!is_batching_enabled(), "0 disables batching");
    unsafe { std::env::set_var(key_opt, "1") };
    assert!(is_batching_enabled(), "non-zero enables");
    unsafe { std::env::remove_var(key_opt) };
    assert!(is_batching_enabled(), "default is on");

    // Restore.
    let restore = |k: &str, v: Option<String>| match v {
        Some(v) => unsafe { std::env::set_var(k, v) },
        None => unsafe { std::env::remove_var(k) },
    };
    restore(key_size, prior_size);
    restore(key_lat, prior_lat);
    restore(key_opt, prior_opt);
}

#[tokio::test]
async fn error_from_flush_fn_propagated() {
    let writer = Arc::new(BatchedWriter::new(
        |_batch: Vec<u32>| async {
            Err::<(), _>(ArgoError::Provider("flush exploded".into()))
        },
        2,
        Duration::from_secs(60),
    ));

    let results = submit_concurrent::<u32>(writer.clone(), vec![1, 2]).await;
    assert_eq!(results.len(), 2);
    for r in &results {
        assert!(r.is_err(), "expected per-item error from flush");
        let msg = format!("{}", r.as_ref().err().unwrap());
        assert!(
            msg.contains("flush exploded"),
            "flush error should pass through (got `{msg}`)"
        );
    }
}
