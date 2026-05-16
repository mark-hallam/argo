//! Unit tests for [`crate::tenant`].

use super::{Tenant, TenantStatus};
use std::str::FromStr;

#[test]
fn new_initializes_active_with_matching_timestamps() {
    let t = Tenant::new("Acme".to_string(), "billing@acme.com".to_string());
    assert_eq!(t.name, "Acme");
    assert_eq!(t.billing_email, "billing@acme.com");
    assert_eq!(t.status, TenantStatus::Active);
    assert_eq!(t.created_at, t.updated_at);
    assert!(t.created_at > 0);
}

#[test]
fn set_status_bumps_updated_at_and_preserves_created_at() {
    let mut t = Tenant::new("Acme".to_string(), "billing@acme.com".to_string());
    let created = t.created_at;
    t.updated_at -= 5;
    t.set_status(TenantStatus::Suspended);
    assert_eq!(t.status, TenantStatus::Suspended);
    assert_eq!(t.created_at, created);
    assert!(t.updated_at >= created);
}

#[test]
fn status_round_trips_through_string() {
    for status in [
        TenantStatus::Active,
        TenantStatus::Suspended,
        TenantStatus::Deleted,
    ] {
        assert_eq!(TenantStatus::from_str(status.as_str()), Ok(status));
    }
    assert!(TenantStatus::from_str("ACTIVE").is_err());
    assert!(TenantStatus::from_str("nope").is_err());
}

#[test]
fn serde_round_trip_uses_snake_case() {
    let t = Tenant::new("Acme".to_string(), "billing@acme.com".to_string());
    let j = serde_json::to_string(&t).expect("encode");
    assert!(j.contains("\"status\":\"active\""), "got: {j}");
    let decoded: Tenant = serde_json::from_str(&j).expect("decode");
    assert_eq!(decoded, t);
}
