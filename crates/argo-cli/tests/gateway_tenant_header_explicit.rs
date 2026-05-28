//! Structural regression pins for `X-Argo-Tenant-Id` enforcement
//! in `crates/argo-cli/src/handlers/gateway/routes_auth.rs`.
//!
//! Why structural-assertion rather than boot-and-HTTP: this crate's
//! `common::boot` integration helpers carry a pre-existing
//! `GatewayCrashed` listener-signal flake on this machine (same
//! deferral that `gateway_uuid_routes_tenant_ownership.rs` calls
//! out at its header). The behaviours below are SQL-style literal
//! invariants — a structural assertion catches the same drift
//! (a refactor that drops the header read, the mismatch check, or
//! the default fallback) without taking on the flake.
//!
//! Companion to brief `fix-spa-client-forward-x-argo-tenant-id-
//! header`. The SPA-side wiring (`web/lib/api/tenant.ts` +
//! `web/lib/api/client.ts`) injects the header on every request
//! once the workspace-switcher track populates a tenant id. These
//! pins lock the gateway side so the contract documented in
//! `docs/skycot/architecture/tenant-resolution.md` cannot silently
//! drift.

use std::path::PathBuf;

fn routes_auth_source() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src/handlers/gateway/routes_auth.rs");
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// Header supplied → respected.
///
/// `dev_bypass_principal` MUST read `x-argo-tenant-id` and parse it
/// as a UUID. Dropping the read would silently downgrade every
/// caller to the workspace-default tenant — exactly the
/// "silent downgrade" the brief flags as the failure mode.
#[test]
fn dev_bypass_principal_reads_x_argo_tenant_id_header() {
    let source = routes_auth_source();
    let normalised: String = source.split_whitespace().collect::<Vec<_>>().join(" ");

    assert!(
        normalised.contains("header_value(req, \"x-argo-tenant-id\")"),
        "routes_auth.rs no longer reads `x-argo-tenant-id` in \
         dev_bypass_principal — every dev-bypass caller silently \
         downgrades to the workspace-default tenant. See brief \
         `fix-spa-client-forward-x-argo-tenant-id-header` §Fix recipe \
         step 3."
    );

    assert!(
        normalised.contains("Uuid::parse_str(&raw).ok()"),
        "routes_auth.rs no longer parses the `x-argo-tenant-id` \
         header value as a UUID — non-UUID labels would now be \
         accepted, breaking cross-tenant isolation."
    );
}

/// No header → fall back to principal-derived tenant.
///
/// The brief's acceptance criterion: "no header → fall back to
/// principal-derived tenant". `dev_bypass_principal` MUST call
/// `workspace_default_tenant()` (the principal-derived default for
/// dev-bypass) when the header is absent. Removing the fallback
/// would 401/500 every legacy caller that hasn't been updated to
/// send the header — breaking the SPA's single-workspace flow.
#[test]
fn missing_tenant_header_falls_back_to_workspace_default() {
    let source = routes_auth_source();
    let normalised: String = source.split_whitespace().collect::<Vec<_>>().join(" ");

    assert!(
        normalised.contains("map_or_else(workspace_default_tenant, TenantId)"),
        "routes_auth.rs no longer falls back to \
         workspace_default_tenant when `x-argo-tenant-id` is absent — \
         single-workspace callers (today's SPA, every legacy CLI) \
         would now fail rather than silently inherit the workspace \
         default. See brief acceptance criterion #3."
    );

    assert!(
        source.contains("fn workspace_default_tenant() -> TenantId"),
        "routes_auth.rs no longer defines workspace_default_tenant — \
         the principal-derived fallback used by `dev_bypass_principal` \
         is gone. Restore the fallback or the SPA's single-workspace \
         flow will regress."
    );
}

/// Header supplied + cross-tenant → 403.
///
/// `enforce_header_match` MUST 403 when the caller supplies an
/// `x-argo-tenant-id` that doesn't match the bound token's tenant.
/// The brief's negative pin: "Cross-tenant negative pin: a header
/// pointing at a tenant the principal does NOT have access to MUST
/// 403, not silently downgrade." Silent downgrade = the caller's
/// auth tenant wins and the request succeeds, leaking the wrong
/// tenant's view back to the operator.
#[test]
fn cross_tenant_header_returns_403() {
    let source = routes_auth_source();

    assert!(
        source.contains("fn enforce_header_match"),
        "routes_auth.rs no longer defines enforce_header_match — \
         the cross-check between caller-supplied principal/tenant \
         headers and the authenticated principal is gone. Every \
         bound-token route is now vulnerable to silent cross-tenant \
         downgrade. See brief negative pin."
    );

    let normalised: String = source.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        normalised.contains("forbidden(\"tenant-header mismatch\")"),
        "routes_auth.rs no longer 403s on `x-argo-tenant-id` \
         mismatch in enforce_header_match — cross-tenant header \
         silently downgrades to the bound token's tenant instead of \
         rejecting. See brief negative pin: the SPA-side wiring \
         depends on this 403 to surface workspace-switcher bugs \
         loudly rather than corrupting reads/writes."
    );

    assert!(
        source.contains("fn tenant_header_matches"),
        "routes_auth.rs no longer defines tenant_header_matches — \
         the UUID-parse-then-equate predicate that backs the 403 is \
         gone. A refactor that compares raw strings instead would \
         re-introduce the case-sensitivity / hyphen-format leak the \
         UUID parse closed."
    );

    assert!(
        normalised.contains("Uuid::parse_str(raw).is_ok_and(|u| u == tenant.0)"),
        "routes_auth.rs no longer enforces UUID-exact match in \
         tenant_header_matches — string equality would let raw \
         labels (`tenant_a` vs `tenant-a`) silently slip through."
    );
}

/// The SPA's resolver lives at `web/lib/api/tenant.ts`. This pin
/// confirms the file exists and re-exports the documented API the
/// client.ts wiring depends on. Drift here breaks the contract
/// documented in `docs/skycot/architecture/tenant-resolution.md`.
#[test]
fn spa_tenant_resolver_exports_documented_api() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .unwrap_or_else(|| panic!("locate repo root from CARGO_MANIFEST_DIR"))
        .to_path_buf();
    let tenant_ts = repo_root.join("web/lib/api/tenant.ts");
    let source = std::fs::read_to_string(&tenant_ts)
        .unwrap_or_else(|e| panic!("read {}: {e}", tenant_ts.display()));

    for needed in [
        "export function resolveTenantId",
        "export function setTenantOverride",
        "export function clearTenantOverride",
    ] {
        assert!(
            source.contains(needed),
            "web/lib/api/tenant.ts no longer exports `{needed}` — \
             the workspace-switcher contract documented in \
             `docs/skycot/architecture/tenant-resolution.md` is \
             broken. See brief `fix-spa-client-forward-x-argo-\
             tenant-id-header` §Fix recipe step 1."
        );
    }
}

/// `web/lib/api/client.ts` MUST wire the resolver in so the header
/// flows on every request. A regression that drops the call would
/// leave the resolver as dead code and the workspace-switcher
/// header would never reach the gateway.
#[test]
fn spa_client_wires_resolve_tenant_id_into_request_headers() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .unwrap_or_else(|| panic!("locate repo root from CARGO_MANIFEST_DIR"))
        .to_path_buf();
    let client_ts = repo_root.join("web/lib/api/client.ts");
    let source = std::fs::read_to_string(&client_ts)
        .unwrap_or_else(|e| panic!("read {}: {e}", client_ts.display()));

    assert!(
        source.contains("resolveTenantId"),
        "web/lib/api/client.ts no longer calls `resolveTenantId` — \
         the workspace-switcher header path is dead. See brief \
         §Fix recipe step 2."
    );

    assert!(
        source.contains("X-Argo-Tenant-Id"),
        "web/lib/api/client.ts no longer stamps the \
         `X-Argo-Tenant-Id` header. The gateway will fall back to \
         the principal-derived tenant on every request, defeating \
         the workspace-switcher track. See brief §Fix recipe step 2."
    );
}
