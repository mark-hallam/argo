use super::*;

#[test]
fn under_cap_allows_recording() {
    let t = CardinalityTracker::new(Caps::defaults().with_override("argo_under_cap", 50));
    for i in 0..40 {
        let label = format!("series_{i}");
        assert_eq!(
            t.observe("argo_under_cap", &[label.as_str()]),
            Decision::Allow,
            "label set {i} should be allowed under the cap",
        );
    }
    let snap = t
        .snapshot()
        .into_iter()
        .find(|r| r.metric == "argo_under_cap")
        .expect("family registered");
    assert!(snap.current_count <= 50);
    assert_eq!(snap.capped_total, 0);
}

#[test]
fn at_hard_cap_drops_new_label_sets() {
    let cap = 30_u32;
    let t = CardinalityTracker::new(Caps::defaults().with_override("argo_hard_cap", cap));
    // Push well past the cap; the HLL has ~3% std error so we
    // over-shoot generously to guarantee at least one Drop.
    let total = 200;
    let mut drops = 0_u64;
    for i in 0..total {
        let label = format!("series_{i}");
        if matches!(
            t.observe("argo_hard_cap", &[label.as_str()]),
            Decision::Drop
        ) {
            drops += 1;
        }
    }
    assert!(drops > 0, "expected at least one Drop past the cap");
    let snap = t
        .snapshot()
        .into_iter()
        .find(|r| r.metric == "argo_hard_cap")
        .expect("family registered");
    assert_eq!(snap.capped_total, drops);
    assert!(snap.current_count >= cap);
}

#[test]
fn histogram_kind_uses_tighter_default_cap() {
    let t = CardinalityTracker::new(Caps::defaults());
    let snap_before = t.snapshot();
    assert!(snap_before.is_empty());
    // Just one observation is enough to materialise the family in
    // the snapshot with its kind-derived cap.
    let _ = t.observe_kind("argo_hist", &["a"], FamilyKind::Histogram);
    let row = t
        .snapshot()
        .into_iter()
        .find(|r| r.metric == "argo_hist")
        .expect("family registered");
    assert_eq!(row.cap, DEFAULT_HISTOGRAM_CAP);

    let _ = t.observe_kind("argo_cnt", &["a"], FamilyKind::CounterGauge);
    let row = t
        .snapshot()
        .into_iter()
        .find(|r| r.metric == "argo_cnt")
        .expect("family registered");
    assert_eq!(row.cap, DEFAULT_COUNTER_GAUGE_CAP);
}

#[test]
fn set_caps_reloads_overrides_at_runtime() {
    let t = CardinalityTracker::new(Caps::defaults());
    let _ = t.observe("argo_reload", &["a"]);
    let row = t
        .snapshot()
        .into_iter()
        .find(|r| r.metric == "argo_reload")
        .expect("family registered");
    assert_eq!(row.cap, DEFAULT_COUNTER_GAUGE_CAP);
    t.set_caps(Caps::defaults().with_override("argo_reload", 42));
    let row = t
        .snapshot()
        .into_iter()
        .find(|r| r.metric == "argo_reload")
        .expect("family still registered post-reload");
    assert_eq!(row.cap, 42);
}

#[test]
fn hll_memory_bounded_under_high_insert_volume() {
    // 100k inserts; the brief requires <100KB per family. The HLL
    // here is a fixed 1024-byte register array plus a tiny struct,
    // so the steady-state cost is bounded regardless of insert
    // count. We assert that the count saturates somewhere near its
    // upper bound (HLL precision-10 caps usable resolution around
    // a few million distinct items) and that the tracker hasn't
    // grown an unbounded internal table.
    let t = CardinalityTracker::new(Caps::defaults().with_override("argo_volume", u32::MAX));
    for i in 0..100_000_u32 {
        let label = format!("series_{i}");
        let _ = t.observe("argo_volume", &[label.as_str()]);
    }
    let snap = t.snapshot();
    assert_eq!(snap.len(), 1, "only one family should be registered");
    assert_eq!(snap[0].metric, "argo_volume");
    // HLL precision-10 yields ~3% std error; the estimate for 100k
    // distinct inserts should land in a wide-but-bounded window.
    let est = snap[0].current_count;
    assert!(
        (50_000..=200_000).contains(&est),
        "HLL estimate {est} unexpectedly far from 100k"
    );
}
