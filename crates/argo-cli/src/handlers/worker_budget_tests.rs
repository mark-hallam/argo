//! Unit tests for `argo worker budget` parsing and rendering.

use super::*;

#[test]
fn parse_usd_integer_dollars() {
    assert_eq!(parse_usd("5").unwrap(), 5_000_000);
    assert_eq!(parse_usd("100").unwrap(), 100_000_000);
}

#[test]
fn parse_usd_zero_allowed() {
    assert_eq!(parse_usd("0").unwrap(), 0);
    assert_eq!(parse_usd("0.00").unwrap(), 0);
}

#[test]
fn parse_usd_decimal_dollars() {
    assert_eq!(parse_usd("5.50").unwrap(), 5_500_000);
    assert_eq!(parse_usd("0.01").unwrap(), 10_000);
    assert_eq!(parse_usd("1.234567").unwrap(), 1_234_567);
}

#[test]
fn parse_usd_trailing_zero_truncation() {
    assert_eq!(parse_usd("12.5").unwrap(), 12_500_000);
    assert_eq!(parse_usd("12.50").unwrap(), 12_500_000);
    assert_eq!(parse_usd("12.500000").unwrap(), 12_500_000);
}

#[test]
fn parse_usd_rejects_negative() {
    let err = parse_usd("-1").unwrap_err();
    assert!(err.to_string().contains("negative"));
}

#[test]
fn parse_usd_rejects_scientific() {
    let err = parse_usd("1e3").unwrap_err();
    assert!(err.to_string().contains("scientific"));
    let err = parse_usd("2.5E2").unwrap_err();
    assert!(err.to_string().contains("scientific"));
}

#[test]
fn parse_usd_rejects_leading_plus() {
    let err = parse_usd("+5").unwrap_err();
    assert!(err.to_string().contains("'+'"));
}

#[test]
fn parse_usd_rejects_garbage() {
    assert!(parse_usd("").is_err());
    assert!(parse_usd("abc").is_err());
    assert!(parse_usd("5.5x").is_err());
    assert!(parse_usd(".").is_err());
    assert!(parse_usd("5.1234567").is_err());
}

#[test]
fn parse_usd_handles_leading_decimal() {
    assert_eq!(parse_usd(".50").unwrap(), 500_000);
    assert_eq!(parse_usd("0.5").unwrap(), 500_000);
}

#[test]
fn render_cap_unset_label() {
    assert_eq!(render_cap(None), "(unset — tenant default applies)");
}

#[test]
fn render_cap_zero_label() {
    assert_eq!(render_cap(Some(0)), "$0.00 (deny all)");
}

#[test]
fn render_cap_dollar_amount() {
    assert_eq!(render_cap(Some(5_500_000)), "$5.50");
    assert_eq!(render_cap(Some(100_000_000)), "$100.00");
}

#[test]
fn render_micros_truncates_to_cents() {
    assert_eq!(render_micros(1_999_999), "$1.99");
    assert_eq!(render_micros(0), "$0.00");
    assert_eq!(render_micros(990_000), "$0.99");
}
