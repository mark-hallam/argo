//! Shared tenant-scoped table catalogue for the `argo compliance`
//! family (export + delete).
//!
//! Three groupings reflect how each table scopes rows to a tenant:
//!
//! - [`BLOB_TENANT_TABLES`] — direct `tenant_id` BLOB column.
//! - [`TEXT_TENANT_TABLES`] — direct `tenant_id` TEXT column.
//! - [`SESSION_FK_TABLES`] — scoped via `session_id` → `session.tenant_id`.
//!
//! [`delete_order`] returns the tables in child-first order so a
//! tenant cascade DELETE inside a single transaction never trips a
//! foreign-key constraint. Audit rows are excluded from this list:
//! the delete handler tombstones them in place rather than deleting
//! them, preserving the hash-chain invariant.
//!
//! Both consumers (export, delete) silently skip a table that does
//! not exist in the current schema — migrations evolve and this
//! surface should not force-link to a particular version.

/// Tables that scope rows via a BLOB `tenant_id` column.
pub const BLOB_TENANT_TABLES: &[&str] = &[
    "agent",
    "memory_entry",
    "role",
    "role_binding",
    "role_duty",
    "session",
    "shared_team_memory",
    "spend_ledger",
    "worker_budget",
    "workspace",
];

/// Tables that scope rows via a TEXT `tenant_id` column.
pub const TEXT_TENANT_TABLES: &[&str] = &[
    "duty",
    "handoff_routing_rule",
    "org_member",
    "org_unit",
    "persona",
    "roster",
    "tenant_quota",
    "worker",
    "worker_archive",
];

/// Tables that scope rows via `session_id` → `session.tenant_id`.
///
/// `audit_record` is intentionally absent: the delete handler
/// tombstones audit rows in place; the export handler still walks
/// it via [`SESSION_FK_TABLES_FOR_EXPORT`].
pub const SESSION_FK_TABLES: &[&str] = &[
    "handoff",
    "judge_record",
    "message",
    "routing_decision",
    "tool_call",
];

/// Same as [`SESSION_FK_TABLES`] but includes `audit_record`. Used by
/// the export handler so the GDPR right-to-access envelope includes
/// the tenant's audit rows verbatim.
pub const SESSION_FK_TABLES_FOR_EXPORT: &[&str] = &[
    "audit_record",
    "handoff",
    "judge_record",
    "message",
    "routing_decision",
    "tool_call",
];

/// Tenant-scoped tables in delete-order: children before parents so
/// cascading inside one transaction never violates a foreign key.
///
/// The order is:
/// 1. session-FK children (handoff, message, tool_call, …)
/// 2. `session` itself (whose deletion would otherwise CASCADE the
///    children, but we issue the deletes explicitly so the dry-run
///    counts and rollback semantics are deterministic)
/// 3. tenant-direct tables (BLOB or TEXT `tenant_id`)
///
/// `audit_record` is intentionally NOT in this list — it is
/// tombstoned (UPDATE) in place by [`crate::delete`].
#[must_use]
pub fn delete_order() -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    out.extend(SESSION_FK_TABLES.iter().copied());
    out.push("session");
    for table in BLOB_TENANT_TABLES {
        if *table == "session" {
            continue;
        }
        out.push(table);
    }
    out.extend(TEXT_TENANT_TABLES.iter().copied());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delete_order_lists_children_before_session() {
        let order = delete_order();
        let session_idx = order
            .iter()
            .position(|t| *t == "session")
            .expect("session present");
        for child in SESSION_FK_TABLES {
            let child_idx = order
                .iter()
                .position(|t| t == child)
                .expect("child present");
            assert!(
                child_idx < session_idx,
                "{child} must precede session in delete order"
            );
        }
    }

    #[test]
    fn delete_order_excludes_audit_record() {
        assert!(!delete_order().contains(&"audit_record"));
    }

    #[test]
    fn delete_order_has_no_duplicates() {
        let order = delete_order();
        let mut sorted = order.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(
            order.len(),
            sorted.len(),
            "delete_order has duplicates: {order:?}"
        );
    }

    #[test]
    fn export_session_fk_tables_includes_audit_record() {
        assert!(SESSION_FK_TABLES_FOR_EXPORT.contains(&"audit_record"));
    }
}
