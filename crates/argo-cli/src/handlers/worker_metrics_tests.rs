//! Unit tests for the `argo worker success` rollup handler. The
//! end-to-end CLI surface is exercised by `tests/worker_success_smoke.rs`.

use super::*;
use argo_audit::AuditKind;
use argo_store::SessionStore;
use chrono::TimeZone;
use uuid::Uuid;

fn ts(year: i32, month: u32, day: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(year, month, day, 12, 0, 0).unwrap()
}

fn row(kind: AuditKind, decided_at: DateTime<Utc>) -> DecisionRow {
    DecisionRow { kind, decided_at }
}

#[test]
fn approval_rate_uses_granted_plus_revoked_only() {
    // 3 granted + 1 revoked + 5 consumed → approval_rate = 3 / (3+1) = 0.75.
    // Consumed rows must NOT influence the granted/revoked rate.
    let rows = [
        row(AuditKind::ApprovalGranted, ts(2026, 5, 1)),
        row(AuditKind::ApprovalGranted, ts(2026, 5, 2)),
        row(AuditKind::ApprovalGranted, ts(2026, 5, 3)),
        row(AuditKind::ApprovalRevoked, ts(2026, 5, 4)),
        row(AuditKind::ApprovalConsumed, ts(2026, 5, 5)),
        row(AuditKind::ApprovalConsumed, ts(2026, 5, 6)),
        row(AuditKind::ApprovalConsumed, ts(2026, 5, 7)),
        row(AuditKind::ApprovalConsumed, ts(2026, 5, 8)),
        row(AuditKind::ApprovalConsumed, ts(2026, 5, 9)),
    ];
    let s = Summary::from_rows(rows.iter());
    assert_eq!(s.granted, 3);
    assert_eq!(s.revoked, 1);
    assert_eq!(s.consumed, 5);
    assert_eq!(s.total, 9);
    assert!((s.approval_rate.unwrap() - 0.75).abs() < 1e-9);
}

#[test]
fn consumed_rate_is_consumed_over_granted() {
    // 4 granted, 2 consumed → 2/4 = 0.5.
    let rows = [
        row(AuditKind::ApprovalGranted, ts(2026, 5, 1)),
        row(AuditKind::ApprovalGranted, ts(2026, 5, 2)),
        row(AuditKind::ApprovalGranted, ts(2026, 5, 3)),
        row(AuditKind::ApprovalGranted, ts(2026, 5, 4)),
        row(AuditKind::ApprovalConsumed, ts(2026, 5, 5)),
        row(AuditKind::ApprovalConsumed, ts(2026, 5, 6)),
    ];
    let s = Summary::from_rows(rows.iter());
    assert_eq!(s.consumed_rate.unwrap(), 0.5);
}

#[test]
fn empty_window_returns_null_rates_not_div_by_zero() {
    let s = Summary::from_rows(std::iter::empty::<&DecisionRow>());
    assert_eq!(s.total, 0);
    assert_eq!(s.granted, 0);
    assert_eq!(s.revoked, 0);
    assert_eq!(s.consumed, 0);
    assert!(s.approval_rate.is_none());
    assert!(s.consumed_rate.is_none());
}

#[test]
fn only_consumed_yields_null_approval_rate_but_zero_consumed_rate_div() {
    // No granted, no revoked → approval_rate is null. consumed_rate
    // is also null because the denominator (granted) is zero — the
    // store must not pretend a rate of 1.0 from consumed-without-grant.
    let rows = [row(AuditKind::ApprovalConsumed, ts(2026, 5, 1))];
    let s = Summary::from_rows(rows.iter());
    assert!(s.approval_rate.is_none());
    assert!(s.consumed_rate.is_none());
    assert_eq!(s.total, 1);
}

#[test]
fn bucket_key_day_is_yyyy_mm_dd() {
    assert_eq!(bucket_key(ts(2026, 5, 9), Bucket::Day), "2026-05-09");
}

#[test]
fn bucket_key_week_is_iso_year_week() {
    // 2026-05-09 is a Saturday; ISO week 19.
    assert_eq!(bucket_key(ts(2026, 5, 9), Bucket::Week), "2026-W19");
    // Year boundary: 2025-12-29 belongs to ISO 2026-W01.
    assert_eq!(bucket_key(ts(2025, 12, 29), Bucket::Week), "2026-W01");
}

#[test]
fn bucket_key_month_is_yyyy_mm() {
    assert_eq!(bucket_key(ts(2026, 5, 9), Bucket::Month), "2026-05");
    assert_eq!(bucket_key(ts(2026, 12, 31), Bucket::Month), "2026-12");
}

#[test]
fn bucketize_produces_one_row_per_iso_week() {
    let rows = [
        row(AuditKind::ApprovalGranted, ts(2026, 5, 4)),  // W19
        row(AuditKind::ApprovalRevoked, ts(2026, 5, 7)),  // W19
        row(AuditKind::ApprovalGranted, ts(2026, 5, 12)), // W20
    ];
    let buckets = bucketize(&rows, Bucket::Week);
    assert_eq!(buckets.len(), 2);
    assert_eq!(buckets[0].bucket, "2026-W19");
    assert_eq!(buckets[0].summary.total, 2);
    assert_eq!(buckets[0].summary.granted, 1);
    assert_eq!(buckets[0].summary.revoked, 1);
    assert_eq!(buckets[1].bucket, "2026-W20");
    assert_eq!(buckets[1].summary.granted, 1);
}

#[test]
fn parse_since_accepts_common_suffixes() {
    // Each call must succeed and produce a cutoff in the past.
    let now = Utc::now().timestamp_micros();
    for raw in ["30d", "12h", "45m", "90s"] {
        let micros = parse_since(raw).unwrap();
        assert!(micros < now, "{raw} should produce a past cutoff");
    }
}

#[test]
fn parse_since_rejects_bad_input() {
    assert!(parse_since("0d").is_err());
    assert!(parse_since("-3d").is_err());
    assert!(parse_since("d").is_err());
    assert!(parse_since("12y").is_err());
    assert!(parse_since("abcd").is_err());
}

#[test]
fn bucket_parse_accepts_all_four_modes() {
    assert_eq!(Bucket::parse("none").unwrap(), Bucket::None);
    assert_eq!(Bucket::parse("day").unwrap(), Bucket::Day);
    assert_eq!(Bucket::parse("week").unwrap(), Bucket::Week);
    assert_eq!(Bucket::parse("month").unwrap(), Bucket::Month);
    assert!(Bucket::parse("hour").is_err());
}

#[test]
fn build_report_with_bucket_none_omits_bucket_rows() {
    let rows = [row(AuditKind::ApprovalGranted, ts(2026, 5, 9))];
    let r = build_report("w-1", None, Bucket::None, &rows);
    assert_eq!(r.bucket, "none");
    assert!(r.buckets.is_empty());
    assert_eq!(r.summary.granted, 1);
}

#[test]
fn render_markdown_snapshot_contains_table_for_weekly_bucket() {
    let rows = [
        row(AuditKind::ApprovalGranted, ts(2026, 5, 4)),
        row(AuditKind::ApprovalRevoked, ts(2026, 5, 7)),
        row(AuditKind::ApprovalConsumed, ts(2026, 5, 8)),
    ];
    let report = build_report("w-snap", None, Bucket::Week, &rows);
    let md = render_markdown(&report);
    // Heading + summary line + per-bucket table — locked surface
    // for downstream consumers.
    assert!(md.starts_with("# Worker w-snap success rate"));
    assert!(md.contains("**Total decisions**: 3"));
    assert!(md.contains("granted: 1"));
    assert!(md.contains("revoked: 1"));
    assert!(md.contains("consumed: 1"));
    assert!(md.contains("**Approval rate**: 50.00%"));
    assert!(md.contains("## Per-week rollup"));
    assert!(md.contains("| 2026-W19 |"));
    assert!(md.contains("| bucket | total | granted | revoked | consumed | approval_rate |"));
}

#[test]
fn format_rate_renders_percentage_or_na() {
    assert_eq!(format_rate(Some(0.0)), "0.00%");
    assert_eq!(format_rate(Some(0.5)), "50.00%");
    assert_eq!(format_rate(Some(1.0)), "100.00%");
    assert_eq!(format_rate(None), "n/a");
}

#[test]
fn format_parse_accepts_md_alias() {
    assert!(matches!(Format::parse("markdown").unwrap(), Format::Markdown));
    assert!(matches!(Format::parse("md").unwrap(), Format::Markdown));
    assert!(matches!(Format::parse("json").unwrap(), Format::Json));
    assert!(Format::parse("yaml").is_err());
}

// ─────────── Integration-flavoured DB tests ────────────────

async fn fresh_store() -> SessionStore {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    store
}

async fn insert_session(
    pool: &Pool<Sqlite>,
    session_id: Uuid,
    worker_id: Option<Uuid>,
) {
    // workspace + tenant prerequisites for the FK on session.
    let workspace_id: [u8; 16] = *Uuid::nil().as_bytes();
    let tenant_id: [u8; 16] = *Uuid::nil().as_bytes();
    sqlx::query(
        "INSERT OR IGNORE INTO workspace (id, tenant_id, name, root_path, created_at) \
         VALUES (?, ?, ?, ?, 0)",
    )
    .bind(workspace_id.as_slice())
    .bind(tenant_id.as_slice())
    .bind("test-ws")
    .bind("/tmp/test")
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO session (id, tenant_id, workspace_id, started_at, model, provider, worker_id) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(session_id.as_bytes().as_slice())
    .bind(tenant_id.as_slice())
    .bind(workspace_id.as_slice())
    .bind(0_i64)
    .bind("test-model")
    .bind("test-provider")
    .bind(worker_id.map(|w| w.as_bytes().to_vec()))
    .execute(pool)
    .await
    .unwrap();
}

async fn insert_audit(
    pool: &Pool<Sqlite>,
    session_id: Uuid,
    kind: AuditKind,
    micros: i64,
) {
    let id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO audit_record (id, session_id, kind, payload_json, parent_hash, self_hash, created_at) \
         VALUES (?, ?, ?, '{}', ?, ?, ?)",
    )
    .bind(id.as_bytes().as_slice())
    .bind(session_id.as_bytes().as_slice())
    .bind(kind.label())
    .bind([0u8; 32].as_slice())
    .bind([0u8; 32].as_slice())
    .bind(micros)
    .execute(pool)
    .await
    .unwrap();
}

#[tokio::test]
async fn fetch_decisions_filters_by_worker_session_join() {
    let store = fresh_store().await;
    let pool = store.pool();
    let target_worker = Uuid::now_v7();
    let other_worker = Uuid::now_v7();
    let target_session = Uuid::now_v7();
    let other_session = Uuid::now_v7();
    insert_session(pool, target_session, Some(target_worker)).await;
    insert_session(pool, other_session, Some(other_worker)).await;

    insert_audit(pool, target_session, AuditKind::ApprovalGranted, 100).await;
    insert_audit(pool, target_session, AuditKind::ApprovalRevoked, 200).await;
    insert_audit(pool, target_session, AuditKind::ApprovalConsumed, 300).await;
    // Off-target worker — must not appear.
    insert_audit(pool, other_session, AuditKind::ApprovalGranted, 150).await;
    // Non-approval kind — must not appear.
    insert_audit(pool, target_session, AuditKind::TurnStart, 50).await;

    let rows = fetch_decisions(pool, WorkerId(target_worker), None)
        .await
        .unwrap();
    assert_eq!(rows.len(), 3);
    let kinds: Vec<_> = rows.iter().map(|r| r.kind).collect();
    assert!(kinds.contains(&AuditKind::ApprovalGranted));
    assert!(kinds.contains(&AuditKind::ApprovalRevoked));
    assert!(kinds.contains(&AuditKind::ApprovalConsumed));
}

#[tokio::test]
async fn fetch_decisions_applies_since_cutoff() {
    let store = fresh_store().await;
    let pool = store.pool();
    let worker = Uuid::now_v7();
    let session = Uuid::now_v7();
    insert_session(pool, session, Some(worker)).await;

    insert_audit(pool, session, AuditKind::ApprovalGranted, 100).await;
    insert_audit(pool, session, AuditKind::ApprovalGranted, 500).await;
    insert_audit(pool, session, AuditKind::ApprovalGranted, 1_000).await;

    let rows = fetch_decisions(pool, WorkerId(worker), Some(400))
        .await
        .unwrap();
    assert_eq!(rows.len(), 2, "rows at 100 should be filtered out");
}

#[tokio::test]
async fn fetch_decisions_for_unknown_worker_yields_empty() {
    let store = fresh_store().await;
    let pool = store.pool();
    let rows = fetch_decisions(pool, WorkerId(Uuid::now_v7()), None)
        .await
        .unwrap();
    assert!(rows.is_empty());
}
