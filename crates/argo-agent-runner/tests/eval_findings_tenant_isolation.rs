//! FINDINGS-ONLY (`#[ignore]`'d) cross-tenant negative tests
//! produced by `eval-argo-agent-runner-turn-lifecycle` (the
//! 2026-05-23 turn-lifecycle audit).
//!
//! Each test pins a load-bearing invariant the audit identified
//! as MISSING in `origin/main` at `5a8726465`. Tests are
//! `#[ignore]`'d so they don't break green CI — the follow-up
//! fix-brief un-ignores them as part of its acceptance.
//!
//! Read order:
//!
//!   - `skill_ab_test_run_must_be_tenant_scoped` — Finding 2.1
//!     (`turn_skills_runtime.rs:159-165`). Demonstrates that a
//!     same-named skill A/B test in tenant A is currently
//!     visible to tenant B's worker.
//!   - `available_for_worker_must_be_tenant_scoped` — Finding
//!     2.3 / 4.1 (`argo-skill/src/store_available.rs:55-70`).
//!     Demonstrates that tenant B's worker sees tenant A's
//!     skill rows when both share a workspace.
//!
//! Both tests are written to FAIL on current `main` once
//! un-ignored — they are the regression bar for the
//! `fix-skill-ab-test-run-tenant-bind` and
//! `fix-skill-table-tenant-column` briefs.

#![allow(unused_imports)]

use argo_store::SessionStore;
use argo_types::TenantId;
use std::sync::Arc;
use uuid::Uuid;

#[tokio::test]
#[ignore = "FINDINGS-ONLY: pins finding 2.1 (skill_ab_test_run tenant leak); un-ignore as part of fix-skill-ab-test-run-tenant-bind"]
async fn skill_ab_test_run_must_be_tenant_scoped() {
    // Scenario: two tenants share a workspace. Both have a skill
    // named "summarize" (workspace-global today — see finding
    // 2.3). Tenant A opens an A/B test on its "summarize"; the
    // expected behavior is that tenant B's worker — when it
    // resolves variants for a turn touching its own "summarize"
    // — does NOT pick from tenant A's run, and `record_outcome`
    // does NOT write trial counts onto tenant A's run row.
    //
    // The test stays as an explicit scaffold (no runtime call)
    // pending the fix-brief that introduces a `tenant_id`
    // column on `skill_ab_test_run` and a tenant-aware lookup
    // helper. Once the column exists, this test wires through
    // `find_active_ab_run(store, tenant_id, "summarize")` and
    // asserts the two lookups return distinct rows.
    let _tenant_a: TenantId = TenantId(Uuid::from_u128(0xA));
    let _tenant_b: TenantId = TenantId(Uuid::from_u128(0xB));
    panic!(
        "FINDINGS-ONLY: pending fix-skill-ab-test-run-tenant-bind. \
         See docs/skycot/evals/scheduled/2026-05-23/eval-argo-agent-runner-turn-lifecycle.md \
         §2.1."
    );
}

#[tokio::test]
#[ignore = "FINDINGS-ONLY: pins finding 2.3/4.1 (skill table workspace-global); un-ignore as part of fix-skill-table-tenant-column"]
async fn available_for_worker_must_be_tenant_scoped() {
    // Scenario: two tenants share a workspace. Tenant A
    // installs a skill "internal-runbook" with min_trust_grade
    // = NULL. Tenant B's worker_b has trust_grade = 5 (well
    // above the floor). Calling `available_for_worker(reader,
    // tenant_b, worker_b)` MUST NOT return tenant A's
    // "internal-runbook" row.
    //
    // Today the SELECT at
    // `argo-skill/src/store_available.rs:55-70` lacks a
    // `tenant_id = ?` predicate; the row leaks across tenants.
    //
    // Wire-up requires the additive migration introducing
    // `skill.tenant_id` (per the fix brief). Until then this
    // test is a scaffold pinning the invariant.
    let _store = match SessionStore::open(":memory:").await {
        Ok(s) => Arc::new(s),
        Err(e) => panic!("open in-memory store: {e}"),
    };
    panic!(
        "FINDINGS-ONLY: pending fix-skill-table-tenant-column. \
         See docs/skycot/evals/scheduled/2026-05-23/eval-argo-agent-runner-turn-lifecycle.md \
         §2.3 + §4.1."
    );
}

#[tokio::test]
#[ignore = "FINDINGS-ONLY: pins finding 4.4 (skill_canary workspace-global)"]
async fn skill_canary_must_be_tenant_scoped() {
    // Scenario: tenant A starts a canary rollout on
    // "summarize" (baseline v1, canary v2, 50% traffic).
    // Tenant B's worker_b should NOT see the canary — its
    // turns should continue to land on the baseline.
    //
    // Today `pick_canary_for_skill(store, "summarize", seed,
    // now)` reads `skill_canary WHERE skill_id = ?` with no
    // tenant filter. Same root cause as finding 4.1.
    panic!(
        "FINDINGS-ONLY: pending fix-skill-table-tenant-column (canary cascade). \
         See docs/skycot/evals/scheduled/2026-05-23/eval-argo-agent-runner-turn-lifecycle.md \
         §4.4."
    );
}

#[tokio::test]
#[ignore = "FINDINGS-ONLY: pins finding 6.3 (MCP server registration not per-vertical)"]
async fn mcp_server_registration_must_be_per_vertical() {
    // Scenario: workspace has verticals V1 and V2. V1's
    // manifest declares MCP server "linear-mcp"; V2's
    // manifest declares MCP server "salesforce-mcp". A
    // session bound to V1 should NOT see the schema for
    // "salesforce-mcp" advertised to the model.
    //
    // Today `register_manifest_mcp_servers` at
    // `dispatch.rs:106-126` loads every vertical's MCP
    // servers into the per-turn registry, so the model sees
    // the union. Per-tenant subprocess isolation (Finding 6.2)
    // is intact, but the *schema* leaks.
    panic!(
        "FINDINGS-ONLY: pending fix-mcp-server-registration-per-vertical. \
         See docs/skycot/evals/scheduled/2026-05-23/eval-argo-agent-runner-turn-lifecycle.md \
         §6.3."
    );
}
