//! Unit tests for [`crate::user`].

use super::{UserAccount, UserId, UserStatus};
use std::str::FromStr;

#[test]
fn user_id_new_is_unique() {
    let a = UserId::new();
    let b = UserId::new();
    assert_ne!(a, b);
    assert_eq!(a.to_string(), a.0.to_string());
}

#[test]
fn user_id_serde_round_trip() {
    let id = UserId::new();
    let json = serde_json::to_string(&id).expect("encode");
    let decoded: UserId = serde_json::from_str(&json).expect("decode");
    assert_eq!(decoded, id);
}

#[test]
fn new_initializes_active_with_no_login() {
    let u = UserAccount::new("alice@acme.com".to_string());
    assert_eq!(u.email, "alice@acme.com");
    assert!(u.display_name.is_none());
    assert!(u.last_login_at.is_none());
    assert_eq!(u.status, UserStatus::Active);
    assert!(u.created_at > 0);
}

#[test]
fn status_round_trips_through_string() {
    for status in [UserStatus::Active, UserStatus::Disabled] {
        assert_eq!(UserStatus::from_str(status.as_str()), Ok(status));
    }
    assert!(UserStatus::from_str("ACTIVE").is_err());
}

#[test]
fn serde_round_trip_uses_snake_case() {
    let u = UserAccount::new("alice@acme.com".to_string());
    let j = serde_json::to_string(&u).expect("encode");
    assert!(j.contains("\"status\":\"active\""), "got: {j}");
    let decoded: UserAccount = serde_json::from_str(&j).expect("decode");
    assert_eq!(decoded, u);
}
