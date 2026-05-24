//! Pin tests for `eval-gateway-uuid-bound-routes-tenant-ownership-sweep`.
//!
//! Companion to the inventory report at
//! `docs/skycot/evals/scheduled/2026-05-23/eval-gateway-uuid-bound-
//! routes-tenant-ownership-sweep.md` (§6 of that doc).
//!
//! Test shape per the brief's spec:
//!
//! - **1 positive regression** locking the canonical 🟢 CHECKED
//!   handler — the SQL predicate in `routes_audit_replay.rs`.
//!   Implemented as a **structural-assertion** test: reads the
//!   handler source and asserts that the SELECT against
//!   `audit_record` still binds `AND tenant_id = ?` alongside
//!   `WHERE id = ?`. The integration-style shape (boot gateway →
//!   POST → assert 404) was rejected because gateway-boot
//!   integration tests across this crate hit a pre-existing
//!   `GatewayCrashed` listener-signal flake on this machine
//!   (e.g. `audit_by_vertical_gateway::routes_catalog_*` fails
//!   identically). A structural assertion catches the same drift
//!   (the SELECT mutation) without taking on the flake.
//!
//! - **4 `#[ignore]`'d FINDINGS-ONLY pins** — one per RED cluster
//!   in the sweep inventory (§3). The fix brief that lands a
//!   cluster's tenant binding un-`#[ignore]`s the corresponding
//!   pin and inverts the leak-assertion into a regression. These
//!   panic with a descriptive message so `cargo test --ignored`
//!   surfaces the work queue.
use std::path::PathBuf;

fn read_manifest_source(rel: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// 🟢 Positive regression — the SELECT in `routes_audit_replay.rs`
/// MUST still bind tenant alongside id.
///
/// Locks the canonical CHECKED handler against silent drift.
/// Dropping `AND tenant_id = ?` from that SELECT would, for a
/// random hex_id, either find no row (silent leak — test against
/// the response code wouldn't catch it) or find another tenant's
/// row. This structural assertion catches BOTH because the SQL
/// literal is the lock.
#[test]
fn audit_replay_handler_still_binds_tenant_id_in_sql() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src/handlers/gateway/routes_audit_replay.rs");
    let source =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));

    // Strip whitespace so a reformat that wraps the SQL across
    // lines doesn't false-trip the test.
    let normalised: String = source.split_whitespace().collect::<Vec<_>>().join(" ");

    assert!(
        normalised.contains("FROM audit_record WHERE id = ? AND tenant_id = ?"),
        "routes_audit_replay.rs no longer binds `AND tenant_id = ?` \
         alongside `WHERE id = ?` in the audit_record SELECT — \
         cross-tenant audit-replay leak regressed. See \
         docs/skycot/evals/scheduled/2026-05-23/\
         eval-gateway-uuid-bound-routes-tenant-ownership-sweep.md §3 \
         (canonical 🟢 CHECKED pattern)."
    );

    // Belt-and-braces: the handler must still call
    // `request_tenant_id(req)` upstream of the bind. A refactor
    // that hardcodes a tenant constant or drops the header read
    // would defeat the SELECT-level filter even if the SQL string
    // looks unchanged.
    assert!(
        source.contains("request_tenant_id(req)"),
        "routes_audit_replay.rs no longer derives tenant via \
         request_tenant_id(req) — header-binding upstream of the \
         SELECT regressed. See sweep report §1 for the binding \
         pattern reference."
    );
}

// === FINDINGS-ONLY pins (un-ignore when fix brief lands) ===
//
// Each #[ignore]'d test below documents one RED cluster from the
// inventory report (§3). Body asserts the leak-as-shipped today
// (the route returns row data ignoring tenant). The corresponding
// fix brief inverts the assertion: response status becomes 404
// (or 403) and the body contains the not-found envelope.
//
// To make these runnable today (proving the leak before the fix
// brief lands) the test would need: (a) two distinct tenant
// workspaces on disk with seeded fixtures and (b) the gateway
// rebooted with `--workspace`/`ARGO_TENANT_ID` overrides matching
// each tenant. The existing `common::boot` helper boots a fresh
// per-test tempdir under a default tenant only, which is sufficient
// for the positive case above but not for cross-tenant fixtures.
// Building those fixtures is the fix-brief's job, not the sweep's
// — so these stay `#[ignore]`'d with a panic body that documents
// the expected experiment.

/// Cluster 1 — `SessionStore::list_messages(SessionId)` leak.
/// Fix brief: `mission-gateway-session-export-tenant-bind`
/// (widened per §5 of the sweep report).
#[ignore = "FINDINGS-ONLY pin; un-ignore once mission-gateway-session-export-tenant-bind lands"]
#[test]
fn pin_cluster_1_session_messages_route_leaks_across_tenants() {
    panic!(
        "FINDINGS pin awaits fix-brief mission-gateway-session-export-tenant-bind. \
         See docs/skycot/evals/scheduled/2026-05-23/\
         eval-gateway-uuid-bound-routes-tenant-ownership-sweep.md §3 cluster 1. \
         Expected post-fix: hit GET /sessions/<tenant_a_sid>/messages with \
         x-argo-tenant-id: tenant_b, assert HTTP/1.1 404 Not Found (not 200)."
    );
}

/// Cluster 2 — `SessionStore::get_session(SessionId)` leak — INVERTED.
///
/// Fix brief `mission-gateway-session-routes-tenant-bind` shipped the
/// SQL-level filter (`get_session_for_tenant` +
/// `session_belongs_to_tenant`) plus a tenant pre-flight in every
/// cluster-2 handler. This test now locks the shape — dropping the
/// filter or the `request_tenant_id` derivation in any cluster-2
/// file regresses the test.
///
/// Structural shape (same family as
/// `audit_replay_handler_still_binds_tenant_id_in_sql` above):
/// asserts each cluster-2 source file calls `request_tenant_id(req)`
/// AND routes the session id through one of the tenant-aware store
/// methods. Catches the silent "drop the AND tenant_id = ?"
/// regression that an integration-style probe would also catch,
/// without taking on the dual-tenant fixture lift.
#[test]
fn pin_cluster_2_session_show_route_leaks_across_tenants() {
    // (source-file, label) — every entry is one of the 11 cluster-2
    // handlers per the sweep report §3 cluster 2 table.
    let files = [
        ("src/handlers/gateway/routes_session.rs", "routes_session"),
        (
            "src/handlers/gateway/routes_session_stats.rs",
            "routes_session_stats",
        ),
        (
            "src/handlers/gateway/routes_session_audit_rollups.rs",
            "routes_session_audit_rollups",
        ),
        (
            "src/handlers/gateway/routes_session_clone.rs",
            "routes_session_clone",
        ),
        (
            "src/handlers/gateway/routes_audit_session.rs",
            "routes_audit_session",
        ),
    ];

    for (rel, label) in files {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel);
        let source = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));

        // Each cluster-2 source file MUST derive tenant from the
        // request header. Hardcoded sentinel or dropped read
        // regresses the bind.
        assert!(
            source.contains("request_tenant_id(req)"),
            "{label} no longer derives tenant via request_tenant_id(req) — \
             the cluster-2 fix (mission-gateway-session-routes-tenant-bind) \
             regressed. See docs/skycot/evals/scheduled/2026-05-23/\
             eval-gateway-uuid-bound-routes-tenant-ownership-sweep.md §3 cluster 2."
        );

        // …and MUST route the session id through one of the
        // tenant-aware store methods. The bare `store.get_session(`
        // call was the leak; the additive `get_session_for_tenant`
        // / `session_belongs_to_tenant` siblings are the fix.
        let normalised: String = source.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(
            normalised.contains("get_session_for_tenant")
                || normalised.contains("session_belongs_to_tenant"),
            "{label} no longer calls get_session_for_tenant or \
             session_belongs_to_tenant — cluster-2 cross-tenant filter \
             regressed. See sweep report §3 cluster 2 for the canonical \
             tenant-aware store methods."
        );
    }
}

/// Cluster 4 — `MemoryStore::set_lock`/`supersede`/`revert`/`history`
/// leak — INVERTED.
///
/// Fix brief `mission-gateway-memory-routes-tenant-bind` shipped
/// tenant-aware MemoryStore siblings plus request-header plumbing
/// across the cluster-4 handlers. This structural test pins that
/// shape without relying on gateway boot, which the sweep report
/// called out as flaky for this test family.
#[test]
fn pin_cluster_4_memory_lock_route_leaks_across_tenants() {
    let handler_files = [
        (
            "src/handlers/gateway/routes_post_admin.rs",
            "routes_post_admin",
            "set_lock_for_tenant",
        ),
        (
            "src/handlers/gateway/routes_post_admin.rs",
            "routes_post_admin",
            "supersede_for_tenant",
        ),
        (
            "src/handlers/gateway/routes_post_admin.rs",
            "routes_post_admin",
            "revert_for_tenant",
        ),
        (
            "src/handlers/gateway/routes_meta.rs",
            "routes_meta",
            "get_for_tenant",
        ),
        (
            "src/handlers/gateway/routes_lists_memory.rs",
            "routes_lists_memory",
            "versions_for_entry_for_tenant",
        ),
    ];

    for (rel, label, method) in handler_files {
        let source = read_manifest_source(rel);
        assert!(
            source.contains("request_tenant_id(req)"),
            "{label} no longer derives tenant via request_tenant_id(req) — \
             cluster-4 memory tenant binding regressed."
        );
        assert!(
            source
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .contains(method),
            "{label} no longer calls MemoryStore::{method}; cluster-4 \
             cross-tenant memory binding regressed."
        );
    }

    let post_dispatch = read_manifest_source("src/handlers/gateway/routes_dispatch_post_memory.rs");
    assert!(
        post_dispatch.contains("memory_unlock_response(id, req, store)")
            && post_dispatch.contains("memory_forget_response(id, req, store)"),
        "memory POST dispatch no longer passes the raw request into \
         unlock/forget, so those handlers cannot derive X-Argo-Tenant-Id."
    );

    let get_dispatch =
        read_manifest_source("src/handlers/gateway/routes_dispatch_get_catalog_groups.rs");
    assert!(
        get_dispatch.contains("memory_versions_response(id, req, store")
            && get_dispatch.contains("memory_show_response_with_format(")
            && get_dispatch.contains("req,"),
        "memory GET dispatch no longer passes the raw request into \
         show/versions, so those handlers cannot derive X-Argo-Tenant-Id."
    );
}

/// Cluster 5 — `SessionStore::get_forge_run` leak.
/// Fix brief: `mission-gateway-forge-runs-tenant-bind`.
#[ignore = "FINDINGS-ONLY pin; un-ignore once mission-gateway-forge-runs-tenant-bind lands"]
#[test]
fn pin_cluster_5_forge_run_promote_leaks_across_tenants() {
    panic!(
        "FINDINGS pin awaits fix-brief mission-gateway-forge-runs-tenant-bind. \
         See docs/skycot/evals/scheduled/2026-05-23/\
         eval-gateway-uuid-bound-routes-tenant-ownership-sweep.md §3 cluster 5. \
         Expected post-fix: hit POST /forge/runs/<tenant_a_run_id>/promote with \
         x-argo-tenant-id: tenant_b, assert HTTP/1.1 404 Not Found (not 200), \
         and verify no skill was promoted in tenant_b's skill registry."
    );
}

/// Cluster 6 — `SkillStore::attach_to_session` leak.
/// Fix brief: `mission-gateway-session-skills-tenant-bind`.
#[ignore = "FINDINGS-ONLY pin; un-ignore once mission-gateway-session-skills-tenant-bind lands"]
#[test]
fn pin_cluster_6_session_attach_skill_leaks_across_tenants() {
    panic!(
        "FINDINGS pin awaits fix-brief mission-gateway-session-skills-tenant-bind. \
         See docs/skycot/evals/scheduled/2026-05-23/\
         eval-gateway-uuid-bound-routes-tenant-ownership-sweep.md §3 cluster 6. \
         Expected post-fix: hit POST /sessions/<tenant_a_sid>/skills/attach with \
         x-argo-tenant-id: tenant_b, assert HTTP/1.1 404 Not Found (not 200), \
         and verify session_skill row was NOT inserted."
    );
}
