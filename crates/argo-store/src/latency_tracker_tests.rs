//! Unit tests for the per-executor latency tracker.
use super::*;

fn empty_buckets() -> [u64; N_BUCKETS] {
    [0u64; N_BUCKETS]
}

#[test]
fn empty_histogram_yields_no_percentiles() {
    let buckets = empty_buckets();
    assert_eq!(percentile(&buckets, 0.5), None);
    assert_eq!(percentile(&buckets, 0.95), None);
    assert_eq!(percentile(&buckets, 0.99), None);
    assert_eq!(max_observed(&buckets), None);
    assert_eq!(total_samples(&buckets), 0);
}

#[test]
fn single_sample_collapses_all_percentiles_to_that_bucket() {
    // 8ms → bucket 3 (range [8ms, 16ms))
    let mut buckets = empty_buckets();
    buckets[bucket_index(Duration::from_millis(8))] = 1;
    let p50 = percentile(&buckets, 0.5).expect("p50");
    let p95 = percentile(&buckets, 0.95).expect("p95");
    let p99 = percentile(&buckets, 0.99).expect("p99");
    // All three percentiles fall in the same bucket — interpolation
    // collapses to the bucket midpoint (single sample).
    assert_eq!(p50, p95);
    assert_eq!(p95, p99);
    assert!(p50 >= Duration::from_millis(8));
    assert!(p50 < Duration::from_millis(16));
    assert_eq!(total_samples(&buckets), 1);
}

#[test]
fn uniform_samples_place_p50_in_expected_range() {
    // 100 samples spread across 1ms..16ms (buckets 0..=3).
    let mut buckets = empty_buckets();
    for i in 0..100u64 {
        let ms = (i % 16).max(1);
        buckets[bucket_index(Duration::from_millis(ms))] += 1;
    }
    let p50 = percentile(&buckets, 0.5).expect("p50");
    // p50 should land somewhere in the 4ms..16ms range with this
    // distribution — be permissive: lower bound at 1ms, upper at
    // 32ms (the next bucket boundary above the highest sample).
    assert!(
        p50 >= Duration::from_millis(1) && p50 < Duration::from_millis(32),
        "p50={p50:?}",
    );
    let p99 = percentile(&buckets, 0.99).expect("p99");
    assert!(p99 >= p50, "p99 must be >= p50, got p99={p99:?} p50={p50:?}");
}

#[test]
fn bucket_boundaries_map_correctly() {
    assert_eq!(bucket_index(Duration::from_micros(500)), 0); // < 1ms → bucket 0
    assert_eq!(bucket_index(Duration::from_millis(1)), 0);
    assert_eq!(bucket_index(Duration::from_millis(2)), 1);
    assert_eq!(bucket_index(Duration::from_millis(3)), 1); // [2, 4)
    assert_eq!(bucket_index(Duration::from_millis(4)), 2);
    assert_eq!(bucket_index(Duration::from_millis(8)), 3);
    assert_eq!(bucket_index(Duration::from_millis(16)), 4);
    assert_eq!(bucket_index(Duration::from_millis(8192)), N_BUCKETS - 3);
    // Overflow bucket: anything >= 2^(N_BUCKETS - 1) ms.
    let overflow = Duration::from_millis(1u64 << (N_BUCKETS - 1));
    assert_eq!(bucket_index(overflow), N_BUCKETS - 1);
    assert_eq!(bucket_index(overflow + Duration::from_secs(60)), N_BUCKETS - 1);
    // Lower/upper bounds.
    assert_eq!(bucket_lower_ms(0), 1);
    assert_eq!(bucket_upper_ms(0), 2);
    assert_eq!(bucket_lower_ms(3), 8);
    assert_eq!(bucket_upper_ms(3), 16);
    // Overflow upper bound saturates at its lower bound.
    assert_eq!(
        bucket_upper_ms(N_BUCKETS - 1),
        bucket_lower_ms(N_BUCKETS - 1)
    );
}

#[tokio::test]
async fn flush_then_reload_round_trips_counts() {
    let pool = sqlx::SqlitePool::connect(":memory:").await.expect("pool");
    let tracker = LatencyTracker::new();
    tracker.record("human", Duration::from_millis(2));
    tracker.record("human", Duration::from_millis(2));
    tracker.record("human", Duration::from_millis(64));
    tracker.record("mcp", Duration::from_millis(8));
    let snap = tracker.drain();
    flush_to_store(&pool, &snap).await.expect("flush");
    // Drain emptied the in-memory tracker.
    assert!(tracker.snapshot().is_empty());
    // Reload from the store and confirm counts match.
    let reloaded = load_persisted(&pool).await.expect("load");
    let human = reloaded.get("human").expect("human row");
    assert_eq!(human[bucket_index(Duration::from_millis(2))], 2);
    assert_eq!(human[bucket_index(Duration::from_millis(64))], 1);
    let mcp = reloaded.get("mcp").expect("mcp row");
    assert_eq!(mcp[bucket_index(Duration::from_millis(8))], 1);
    // A second flush adds on top (ON CONFLICT DO UPDATE).
    tracker.record("human", Duration::from_millis(2));
    let snap2 = tracker.drain();
    flush_to_store(&pool, &snap2).await.expect("flush 2");
    let reloaded2 = load_persisted(&pool).await.expect("load 2");
    let human2 = reloaded2.get("human").expect("human row");
    assert_eq!(human2[bucket_index(Duration::from_millis(2))], 3);
}

#[test]
fn per_kind_buckets_are_isolated() {
    let tracker = LatencyTracker::new();
    tracker.record("human", Duration::from_millis(4));
    tracker.record("human", Duration::from_millis(4));
    tracker.record("mcp", Duration::from_millis(1024));
    let snap = tracker.snapshot();
    let human = snap.get("human").expect("human");
    let mcp = snap.get("mcp").expect("mcp");
    assert_eq!(human[bucket_index(Duration::from_millis(4))], 2);
    // Recording on `human` must not have leaked into `mcp`.
    assert_eq!(mcp[bucket_index(Duration::from_millis(4))], 0);
    assert_eq!(mcp[bucket_index(Duration::from_millis(1024))], 1);
    // Totals.
    assert_eq!(total_samples(human), 2);
    assert_eq!(total_samples(mcp), 1);
}

#[tokio::test]
async fn concurrent_records_are_lossless() {
    let tracker = Arc::new(LatencyTracker::new());
    // 4 tasks × 1k samples each → 4000 total.
    let mut handles = Vec::new();
    for _ in 0..4 {
        let t = tracker.clone();
        handles.push(tokio::spawn(async move {
            for _ in 0..1_000 {
                t.record("human", Duration::from_millis(8));
            }
        }));
    }
    for h in handles {
        h.await.expect("task");
    }
    let snap = tracker.snapshot();
    let human = snap.get("human").expect("human");
    assert_eq!(
        total_samples(human),
        4_000,
        "all 4000 concurrent records must land",
    );
}
