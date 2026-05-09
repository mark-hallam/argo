//! Formatter / parse tests for `worker_kpi` handler. The full
//! record→list→series CLI round-trip is covered by
//! `tests/worker_kpi_smoke.rs`; these focus on the small Rust
//! helpers that are awkward to assert on through stdout alone.

use super::*;
use chrono::TimeZone;

#[test]
fn parse_since_accepts_rfc3339_z() {
    let micros = parse_since("2026-05-01T00:00:00Z").unwrap();
    let expected = Utc
        .with_ymd_and_hms(2026, 5, 1, 0, 0, 0)
        .unwrap()
        .timestamp_micros();
    assert_eq!(micros, expected);
}

#[test]
fn parse_since_accepts_rfc3339_with_offset() {
    let micros = parse_since("2026-05-01T05:30:00+05:30").unwrap();
    let expected = Utc
        .with_ymd_and_hms(2026, 5, 1, 0, 0, 0)
        .unwrap()
        .timestamp_micros();
    assert_eq!(micros, expected);
}

#[test]
fn parse_since_rejects_bare_date() {
    let err = parse_since("2026-05-01").unwrap_err();
    assert!(err.to_string().contains("RFC3339"));
}

#[test]
fn format_iso_round_trips_through_micros() {
    let now = Utc.with_ymd_and_hms(2026, 5, 9, 12, 34, 56).unwrap();
    let label = format_iso(now.timestamp_micros());
    assert!(label.starts_with("2026-05-09T12:34:56"));
}

#[test]
fn bucket_label_covers_all_variants() {
    assert_eq!(bucket_label(Bucket::Day), "day");
    assert_eq!(bucket_label(Bucket::Week), "week");
    assert_eq!(bucket_label(Bucket::Month), "month");
}
