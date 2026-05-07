//! Unit tests for `dreams_backend`. Stays in a sibling file via
//! `#[path] mod tests;` so the impl module keeps headroom under
//! the 500-LOC invariant.
use super::*;
use crate::{MemoryFile, MemoryStore};
use argo_identity::{MemoryNs, MemoryScope, WorkerId};
use argo_store::SessionStore;
use argo_types::{TenantId, WorkspaceId};
use chrono::{TimeZone, Utc};
use std::sync::{Arc, Mutex};

async fn fresh() -> (MemoryStore, WorkspaceId, TenantId) {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let tenant = TenantId::new();
    let ws = WorkspaceId::new();
    sqlx::query(
        "INSERT INTO workspace (id, tenant_id, name, root_path, created_at)
         VALUES (?, ?, 'dreams-test', '.', strftime('%s','now'))",
    )
    .bind(ws.0.as_bytes().as_slice())
    .bind(tenant.0.as_bytes().as_slice())
    .execute(store.pool())
    .await
    .unwrap();
    (MemoryStore::new(Arc::new(store)), ws, tenant)
}

/// Test stub — captures call count and serves canned results.
struct StubCaller {
    summary: String,
    cost_micros: i64,
    model: String,
    calls: Mutex<Vec<(String, String)>>, // (worker_id, month_label)
}

impl StubCaller {
    fn new(summary: impl Into<String>, cost_micros: i64) -> Self {
        Self {
            summary: summary.into(),
            cost_micros,
            model: "claude-dreams-1-stub".into(),
            calls: Mutex::new(Vec::new()),
        }
    }

    fn call_count(&self) -> usize {
        self.calls.lock().unwrap().len()
    }
}

#[async_trait]
impl DreamsCaller for StubCaller {
    async fn dream(
        &self,
        ns: &MemoryNs,
        month: YearMonth,
        _rows: &[MemoryEntry],
    ) -> ArgoResult<DreamResult> {
        self.calls
            .lock()
            .unwrap()
            .push((ns.worker_id.0.to_string(), month.label()));
        Ok(DreamResult {
            summary: self.summary.clone(),
            cost_micros: self.cost_micros,
            model: self.model.clone(),
        })
    }
}

#[test]
fn from_env_errors_without_api_key() {
    let prev = std::env::var("ANTHROPIC_API_KEY").ok();
    // SAFETY: tests run sequentially per-mod when default test
    // harness is single-threaded for env mutation; cargo defaults
    // to multi-thread test runner so we save+restore in each test
    // that touches env vars.
    std::env::remove_var("ANTHROPIC_API_KEY");
    let err = EnvDreamsCaller::from_env().expect_err("should error without key");
    let msg = format!("{err}");
    assert!(
        msg.contains("ANTHROPIC_API_KEY"),
        "error mentions the missing env var: {msg}"
    );
    if let Some(v) = prev {
        std::env::set_var("ANTHROPIC_API_KEY", v);
    }
}

#[test]
fn parse_response_extracts_summary_cost_model() {
    let v = serde_json::json!({
        "dream": {
            "summary": "Maya resolved 3 escalations and asked for SOC2 docs.",
            "cost_usd_micros": 812_000,
            "model": "claude-dreams-1.2",
        }
    });
    let r = parse_response(&v).expect("valid response parses");
    assert_eq!(r.summary, "Maya resolved 3 escalations and asked for SOC2 docs.");
    assert_eq!(r.cost_micros, 812_000);
    assert_eq!(r.model, "claude-dreams-1.2");
}

#[test]
fn parse_response_falls_back_when_cost_and_model_omitted() {
    let v = serde_json::json!({"dream": {"summary": "ok"}});
    let r = parse_response(&v).expect("partial-but-valid response parses");
    assert_eq!(r.summary, "ok");
    assert_eq!(r.cost_micros, DEFAULT_COST_USD_MICROS);
    assert_eq!(r.model, "claude-dreams-1");
}

#[test]
fn parse_response_errors_when_summary_missing() {
    let v = serde_json::json!({"dream": {"cost_usd_micros": 500_000}});
    let err = parse_response(&v).expect_err("missing summary errors");
    assert!(format!("{err}").contains("missing /dream/summary"));
}

#[test]
fn daily_cap_accumulates_and_rejects_past_cap() {
    let dir = tempfile::tempdir().unwrap();
    let prev = std::env::var(ENV_DAILY_CAP).ok();
    std::env::remove_var(ENV_DAILY_CAP);
    let tracker = DailyCapTracker::new(dir.path(), "worker-a");
    assert_eq!(tracker.cap_micros, DEFAULT_DAILY_CAP_USD_MICROS);
    let day = chrono::NaiveDate::from_ymd_opt(2026, 5, 7).unwrap();

    let after_first = tracker.check_and_charge(day, 750_000).unwrap();
    assert_eq!(after_first, 750_000);
    let after_second = tracker.check_and_charge(day, 750_000).unwrap();
    assert_eq!(after_second, 1_500_000);

    // Drive cumulative to just under the cap, then push past it.
    let _ = tracker.check_and_charge(day, 3_000_000).unwrap();
    let _ = tracker.check_and_charge(day, 500_000).unwrap();
    let err = tracker
        .check_and_charge(day, 1_000_000)
        .expect_err("must exceed cap");
    assert!(format!("{err}").contains("dreams daily cap exceeded"));

    if let Some(v) = prev {
        std::env::set_var(ENV_DAILY_CAP, v);
    }
}

#[test]
fn daily_cap_env_override_lowers_cap() {
    let dir = tempfile::tempdir().unwrap();
    let prev = std::env::var(ENV_DAILY_CAP).ok();
    std::env::set_var(ENV_DAILY_CAP, "1000000"); // $1.00
    let tracker = DailyCapTracker::new(dir.path(), "worker-b");
    assert_eq!(tracker.cap_micros, 1_000_000);
    let day = chrono::NaiveDate::from_ymd_opt(2026, 5, 7).unwrap();
    tracker.check_and_charge(day, 800_000).unwrap();
    let err = tracker
        .check_and_charge(day, 300_000)
        .expect_err("env-override cap respected");
    assert!(format!("{err}").contains("dreams daily cap exceeded"));
    match prev {
        Some(v) => std::env::set_var(ENV_DAILY_CAP, v),
        None => std::env::remove_var(ENV_DAILY_CAP),
    }
}

#[tokio::test]
async fn anomaly_guard_blocks_when_event_present() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    argo_store::anomaly::ensure_schema(store.pool()).await.unwrap();
    let mut baseline = argo_store::anomaly::BehaviorBaseline::empty(
        "worker-anomaly", "send_email",
    );
    let sample = argo_store::anomaly::BehaviorSample {
        latency_ms: 2400,
        retry_count: 0,
    };
    baseline.fold(&sample, Utc::now());
    argo_store::anomaly::insert_event(
        store.pool(),
        "worker-anomaly",
        "action-a",
        "send_email",
        "latency_ms",
        2400.0,
        &baseline,
        16.0,
        4.0,
        Utc::now(),
    )
    .await
    .unwrap();

    let err = anomaly_guard(store.pool(), "worker-anomaly")
        .await
        .expect_err("guard refuses worker with open anomaly");
    let msg = format!("{err}");
    assert!(msg.contains("dreams anomaly guard"));
    assert!(msg.contains("worker-anomaly"));
}

#[tokio::test]
async fn anomaly_guard_passes_for_clean_worker() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    argo_store::anomaly::ensure_schema(store.pool()).await.unwrap();
    anomaly_guard(store.pool(), "worker-clean")
        .await
        .expect("clean worker passes");
}

#[tokio::test]
async fn run_dream_for_worker_persists_summary_with_marker() {
    let (memory, ws, tenant) = fresh().await;
    argo_store::anomaly::ensure_schema(memory.pool()).await.unwrap();
    let ns = MemoryNs::new(tenant, WorkerId::new(), MemoryScope::WorkerSelf);

    // Seed a single recent row inside the previous calendar month.
    let now = Utc::now();
    let month = YearMonth::previous(now);
    let mid_month = Utc
        .with_ymd_and_hms(month.year, month.month, 15, 12, 0, 0)
        .single()
        .unwrap();
    let row = memory
        .append_in_namespace(&ns, ws, MemoryFile::Memory, "Customer asked for SOC2".into(), None)
        .await
        .unwrap();
    sqlx::query("UPDATE memory_entry SET learned_at = ? WHERE id = ?")
        .bind(mid_month.timestamp_micros())
        .bind(row.id.as_bytes().as_slice())
        .execute(memory.pool())
        .await
        .unwrap();

    let dir = tempfile::tempdir().unwrap();
    std::env::remove_var(ENV_DAILY_CAP);
    let caller = StubCaller::new("Polished Dreams summary.", 750_000);
    let entry = run_dream_for_worker(&memory, ws, dir.path(), &ns, now, &caller)
        .await
        .expect("happy path produces a memory entry");

    assert!(
        entry.body.contains("argo-memory-dream-summary"),
        "marker present: {}",
        entry.body
    );
    assert!(
        entry.body.contains("source=dreams_api"),
        "source tag present: {}",
        entry.body
    );
    assert!(entry.body.contains("Polished Dreams summary."));
    assert_eq!(caller.call_count(), 1);

    // Second call same day with cost=4_500_000 would exceed the
    // remaining $4.25/$5.00 cap and refuse.
    let big_caller = StubCaller::new("would-be-second-summary", 4_500_000);
    let err = run_dream_for_worker(&memory, ws, dir.path(), &ns, now, &big_caller)
        .await
        .expect_err("second dream past cap is refused");
    assert!(format!("{err}").contains("dreams daily cap exceeded"));
}

#[tokio::test]
async fn run_dream_for_worker_errors_on_empty_window() {
    let (memory, ws, tenant) = fresh().await;
    argo_store::anomaly::ensure_schema(memory.pool()).await.unwrap();
    let ns = MemoryNs::new(tenant, WorkerId::new(), MemoryScope::WorkerSelf);
    let dir = tempfile::tempdir().unwrap();
    let caller = StubCaller::new("would-not-be-saved", 750_000);

    let err = run_dream_for_worker(&memory, ws, dir.path(), &ns, Utc::now(), &caller)
        .await
        .expect_err("no rows -> error");
    let msg = format!("{err}");
    assert!(
        msg.contains("no recent memory rows to dream over"),
        "expected empty-window error, got: {msg}"
    );
    assert_eq!(caller.call_count(), 0, "no call when nothing to dream over");
}

#[test]
fn dream_summary_marker_includes_namespace_and_model() {
    let ns = MemoryNs::new(
        TenantId::new(),
        WorkerId::new(),
        MemoryScope::SharedTeam,
    );
    let month = YearMonth { year: 2026, month: 5 };
    let m = dream_summary_marker(&ns, month, "claude-dreams-1");
    assert!(m.contains("argo-memory-dream-summary"));
    assert!(m.contains("source=dreams_api"));
    assert!(m.contains("month=2026-05"));
    assert!(m.contains("scope=shared_team"));
    assert!(m.contains("model=claude-dreams-1"));
}

#[test]
fn build_request_body_includes_namespace_and_rows() {
    let ns = MemoryNs::new(
        TenantId::new(),
        WorkerId::new(),
        MemoryScope::WorkerSelf,
    );
    let month = YearMonth { year: 2026, month: 5 };
    let row = MemoryEntry {
        id: uuid::Uuid::now_v7(),
        workspace_id: WorkspaceId::new(),
        file: MemoryFile::Memory,
        body: "test row body".into(),
        learned_in: None,
        learned_at: Utc::now(),
        hash: [0u8; 32],
        superseded_by: None,
        lock: argo_types::Lock::None,
        vertical_id: None,
    };
    let body = build_request_body(&ns, month, std::slice::from_ref(&row));
    assert_eq!(body["scope"], "memory_summary");
    assert_eq!(body["month"], "2026-05");
    assert_eq!(body["namespace"]["memory_scope"], "worker_self");
    assert_eq!(body["rows"][0]["body"], "test row body");
    assert!(body["rows"][0]["learned_at_micros"].is_i64());
}
