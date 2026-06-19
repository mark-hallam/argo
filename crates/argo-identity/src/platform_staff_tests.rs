//! Unit tests for the platform-staff identity types (DEC-057 Phase 1).

use super::*;
use crate::user::UserId;
use std::str::FromStr;

#[test]
fn role_labels_round_trip() {
    for role in [
        PlatformRole::Support,
        PlatformRole::BillingOps,
        PlatformRole::SuperAdmin,
    ] {
        assert_eq!(PlatformRole::from_str(role.as_str()).unwrap(), role);
    }
}

#[test]
fn unknown_role_is_rejected() {
    assert!(PlatformRole::from_str("owner").is_err());
    assert!(PlatformRole::from_str("").is_err());
}

#[test]
fn status_labels_round_trip() {
    for s in [PlatformStaffStatus::Active, PlatformStaffStatus::Disabled] {
        assert_eq!(PlatformStaffStatus::from_str(s.as_str()).unwrap(), s);
    }
}

#[test]
fn only_billing_and_super_admin_can_suspend() {
    assert!(!PlatformRole::Support.can_suspend());
    assert!(PlatformRole::BillingOps.can_suspend());
    assert!(PlatformRole::SuperAdmin.can_suspend());
}

#[test]
fn read_accounts_permitted_for_every_role() {
    for role in [
        PlatformRole::Support,
        PlatformRole::BillingOps,
        PlatformRole::SuperAdmin,
    ] {
        assert!(StaffAction::ReadAccounts.permitted(role));
    }
}

#[test]
fn suspend_action_gated_to_suspend_capable_roles() {
    assert!(!StaffAction::SuspendTenant.permitted(PlatformRole::Support));
    assert!(StaffAction::SuspendTenant.permitted(PlatformRole::BillingOps));
    assert!(StaffAction::SuspendTenant.permitted(PlatformRole::SuperAdmin));
}

#[test]
fn new_grant_is_active() {
    let s = PlatformStaff::new(UserId::new(), PlatformRole::Support, None);
    assert!(s.is_active());
    assert_eq!(s.status, PlatformStaffStatus::Active);
}

#[test]
fn disabled_grant_is_not_active() {
    let mut s = PlatformStaff::new(UserId::new(), PlatformRole::SuperAdmin, None);
    s.status = PlatformStaffStatus::Disabled;
    assert!(!s.is_active());
}
