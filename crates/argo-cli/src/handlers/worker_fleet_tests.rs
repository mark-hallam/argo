use super::worker_fleet::parse_duration_secs;

#[test]
fn parse_duration_accepts_supported_units() {
    assert_eq!(parse_duration_secs("60s").unwrap(), 60);
    assert_eq!(parse_duration_secs("30m").unwrap(), 1_800);
    assert_eq!(parse_duration_secs("12h").unwrap(), 43_200);
    assert_eq!(parse_duration_secs("7d").unwrap(), 604_800);
}

#[test]
fn parse_duration_rejects_missing_or_unknown_units() {
    assert!(parse_duration_secs("").is_err());
    assert!(parse_duration_secs("10").is_err());
    assert!(parse_duration_secs("2w").is_err());
    assert!(parse_duration_secs("abc").is_err());
}

#[test]
fn parse_duration_rejects_overflow() {
    let huge = format!("{}d", i64::MAX);
    assert!(parse_duration_secs(&huge).is_err());
}
