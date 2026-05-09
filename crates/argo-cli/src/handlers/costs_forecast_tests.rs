//! Unit coverage for the pure forecasting math in
//! [`super::compute_forecast`]. Each test pins one acceptance
//! criterion from the brief: horizon shape, empty-data safety,
//! lookback clamping, leap-year EOM, and JSON shape.

use super::{compute_forecast, Forecast, Horizon, SpendRow};
use chrono::{Duration, TimeZone, Utc};

fn row(at_us: i64, micros: u64) -> SpendRow {
    SpendRow {
        charged_at_micros: at_us,
        amount_usd_micros: micros,
    }
}

#[test]
fn eod_horizon_with_1h_lookback_projects_correctly() {
    // now = 2026-05-09T12:00:00Z (12h into the day, 12h remaining).
    let now = Utc.with_ymd_and_hms(2026, 5, 9, 12, 0, 0).unwrap();
    // Anchor a row at exactly `now - 1h` so the lookback window is
    // fully populated and the clamp-to-available path does not fire.
    // Total spend in lookback = $1.00.
    let one_hour_ago = (now - Duration::hours(1)).timestamp_micros();
    let spend = vec![row(one_hour_ago, 1_000_000)];
    let f = compute_forecast(&spend, now, Horizon::Eod, Duration::hours(1));
    // Rate = 1_000_000 micros / 3600 s. 12h = 43200s remain.
    // projected_additional ≈ 1_000_000 * 43200 / 3600 = 12_000_000.
    assert_eq!(f.actual_so_far_usd_micros, 1_000_000);
    assert!(
        (f.projected_additional_usd_micros as i64 - 12_000_000).abs() <= 100,
        "projected_additional={} expected ~12_000_000",
        f.projected_additional_usd_micros
    );
    assert_eq!(
        f.projected_total_usd_micros,
        f.actual_so_far_usd_micros + f.projected_additional_usd_micros
    );
    // ±20% band sanity check.
    assert!(f.band_lower_usd_micros < f.projected_total_usd_micros);
    assert!(f.band_upper_usd_micros > f.projected_total_usd_micros);
}

#[test]
fn eom_horizon_spans_month_boundary() {
    // now = 2026-05-09T00:00:00Z. Period = May 1 .. June 1.
    let now = Utc.with_ymd_and_hms(2026, 5, 9, 0, 0, 0).unwrap();
    let f = compute_forecast(&[], now, Horizon::Eom, Duration::hours(24));
    let expected_start = Utc
        .with_ymd_and_hms(2026, 5, 1, 0, 0, 0)
        .unwrap()
        .timestamp_micros();
    let expected_end = Utc
        .with_ymd_and_hms(2026, 6, 1, 0, 0, 0)
        .unwrap()
        .timestamp_micros();
    assert_eq!(f.period_start_micros, expected_start);
    assert_eq!(f.period_end_micros, expected_end);
}

#[test]
fn eoq_horizon_spans_quarter_boundary() {
    // now = 2026-05-09 (Q2). Period = April 1 .. July 1.
    let now = Utc.with_ymd_and_hms(2026, 5, 9, 12, 0, 0).unwrap();
    let f = compute_forecast(&[], now, Horizon::Eoq, Duration::hours(24));
    let expected_start = Utc
        .with_ymd_and_hms(2026, 4, 1, 0, 0, 0)
        .unwrap()
        .timestamp_micros();
    let expected_end = Utc
        .with_ymd_and_hms(2026, 7, 1, 0, 0, 0)
        .unwrap()
        .timestamp_micros();
    assert_eq!(f.period_start_micros, expected_start);
    assert_eq!(f.period_end_micros, expected_end);

    // Q4 wraps year boundary: now = 2026-11-15 → Oct 1 .. Jan 1, 2027.
    let q4_now = Utc.with_ymd_and_hms(2026, 11, 15, 0, 0, 0).unwrap();
    let f4 = compute_forecast(&[], q4_now, Horizon::Eoq, Duration::hours(24));
    assert_eq!(
        f4.period_start_micros,
        Utc.with_ymd_and_hms(2026, 10, 1, 0, 0, 0)
            .unwrap()
            .timestamp_micros()
    );
    assert_eq!(
        f4.period_end_micros,
        Utc.with_ymd_and_hms(2027, 1, 1, 0, 0, 0)
            .unwrap()
            .timestamp_micros()
    );
}

#[test]
fn empty_spend_rows_returns_zero_forecast_without_panic() {
    let now = Utc.with_ymd_and_hms(2026, 5, 9, 12, 0, 0).unwrap();
    let f = compute_forecast(&[], now, Horizon::Eod, Duration::hours(24));
    assert_eq!(f.actual_so_far_usd_micros, 0);
    assert_eq!(f.projected_additional_usd_micros, 0);
    assert_eq!(f.projected_total_usd_micros, 0);
    assert_eq!(f.band_lower_usd_micros, 0);
    assert_eq!(f.band_upper_usd_micros, 0);
    assert!(f.no_data);
    assert_eq!(f.rows_in_lookback, 0);
}

#[test]
fn lookback_longer_than_available_data_clamps_to_available() {
    // Only 30 minutes of data, but lookback = 24h. Effective rate
    // should use the 30-minute span (so rate is HIGHER than the
    // unclamped 24h calculation would yield).
    let now = Utc.with_ymd_and_hms(2026, 5, 9, 12, 0, 0).unwrap();
    let oldest = (now - Duration::minutes(30)).timestamp_micros();
    let spend = vec![
        row(oldest, 600_000),
        row(oldest + 600_000_000, 600_000),
        row(oldest + 1_200_000_000, 600_000),
    ];
    let f = compute_forecast(&spend, now, Horizon::Eod, Duration::hours(24));
    // Effective lookback ≈ 30 min = 1800 s. Sum = 1_800_000 micros.
    // Rate ≈ 1000 micros/sec. With ~30 min = 1800 s clamp, the
    // computed lookback_seconds should be ~1800, not 86_400.
    assert!(
        f.lookback_seconds <= 1_900,
        "lookback_seconds={} expected ≤ 1900 (clamped)",
        f.lookback_seconds
    );
    assert!(
        f.lookback_seconds >= 1_700,
        "lookback_seconds={} expected ≥ 1700 (clamped)",
        f.lookback_seconds
    );
    assert_eq!(f.rows_in_lookback, 3);
}

#[test]
fn leap_year_eom_spans_feb_29() {
    // 2028 is a leap year. now = Feb 15, 2028 → period end = Mar 1.
    let now = Utc.with_ymd_and_hms(2028, 2, 15, 0, 0, 0).unwrap();
    let f = compute_forecast(&[], now, Horizon::Eom, Duration::hours(24));
    let expected_end = Utc
        .with_ymd_and_hms(2028, 3, 1, 0, 0, 0)
        .unwrap()
        .timestamp_micros();
    assert_eq!(f.period_end_micros, expected_end);
    // Period is 29 days = 2_505_600 s.
    let span_seconds = (f.period_end_micros - f.period_start_micros) / 1_000_000;
    assert_eq!(span_seconds, 29 * 24 * 3600);

    // Non-leap 2026: Feb has 28 days.
    let non_leap = Utc.with_ymd_and_hms(2026, 2, 15, 0, 0, 0).unwrap();
    let f2 = compute_forecast(&[], non_leap, Horizon::Eom, Duration::hours(24));
    let span2 = (f2.period_end_micros - f2.period_start_micros) / 1_000_000;
    assert_eq!(span2, 28 * 24 * 3600);
}

#[test]
fn json_format_round_trips() {
    // Serialize → parse → verify the public-shape fields land where
    // a downstream script would expect them.
    let now = Utc.with_ymd_and_hms(2026, 5, 9, 12, 0, 0).unwrap();
    let spend = vec![row((now - Duration::minutes(30)).timestamp_micros(), 500_000)];
    let f = compute_forecast(&spend, now, Horizon::Eom, Duration::hours(1));
    let json = serde_json::to_string(&f).expect("forecast serializes");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("forecast parses");
    assert_eq!(parsed["horizon"], "eom");
    assert_eq!(
        parsed["actual_so_far_usd_micros"]
            .as_u64()
            .expect("u64 actual"),
        f.actual_so_far_usd_micros
    );
    assert_eq!(
        parsed["projected_total_usd_micros"]
            .as_u64()
            .expect("u64 total"),
        f.projected_total_usd_micros
    );
    assert!(parsed["band_lower_usd_micros"].is_u64());
    assert!(parsed["band_upper_usd_micros"].is_u64());
    assert!(parsed["period_start_micros"].is_i64());
    assert!(parsed["period_end_micros"].is_i64());

    // Round-trip via Forecast itself would require Deserialize; we
    // verify the public JSON shape instead, which is the contract
    // downstream scripts depend on.
    let _: Forecast = f;
}
