# pricing-quota-seam-resolve-request-tenant

**Owner**: claude or codex — **P1 BILLING + WS1 TENANT-CORRECTNESS. GATED-INVITE BLOCKER** (must land before multiple design partners share the gateway).
**Depends on**: #2871 (`launch-wire-tier-quota-and-usage-metering` — the seam this fixes).
**Authored against**: origin/main `f52e93689` on `2026-06-12`.
**Validate before claim**: bash scripts/check-brief-freshness.sh pricing-quota-seam-resolve-request-tenant

## Problem (found in the #2871 conductor adversarial review)

`quota_middleware::check_quota_or_reject` resolves its tenant from the
GATEWAY PROCESS's cwd (`std::env::current_dir()` → `ensure_workspace(name)`),
not from the request's authenticated session. The flaw predates #2871 (the
override gate it replaced did the same — this is another instance of the
WS1-1 cwd-vs-JWT split, NOT a #2871 regression), but #2871 made the meter
real, so the wrong key now has real consequences in multi-tenant:

- Every request meters against ONE tenant (the gateway workspace's), so all
  tenants share one usage bucket: tenant A's traffic consumes tenant B's
  free-tier cap (cross-tenant denial of service at the 100-action boundary).
- `GET /api/v1/tenant/usage`, the `/billing` meter, and the #2870 overage
  report job all read `tenant_usage` PER TENANT — but only the cwd-tenant's
  row is ever written, so real tenants under-report (under-BILL on Pro
  overage) while the cwd-tenant over-reports.

Single-tenant deployments are unaffected (cwd tenant == the only tenant),
which is why the gated URL can go live before this lands — but NOT
multi-partner invites.

## Required change

1. Thread the REQUEST-resolved tenant into `check_quota_or_reject(store,
   tenant, kind)` instead of resolving cwd inside. Use the canonical
   request-tenant resolver the WS1-1 fix established (grep
   `resolve_tenant_for_request` in `crates/argo-cli/src/handlers/gateway/` —
   reuse it, do not parallel-author).
2. Update the 4 call sites (`POST /sessions`, forge optimize, skill refine,
   session messages — see `routes_dispatch_post*.rs`). For any call site
   that genuinely has NO authenticated tenant (pre-auth), decide explicitly:
   either meter against the fixed anonymous tenant
   (`argo_agent_runner::transport_gate::anonymous_tenant_id()` precedent,
   #2865) or skip metering with a doc comment saying why — never silently
   fall back to the cwd tenant.
3. Keep the fail-open-on-store-error direction (documented in the helper) —
   that call is correct for an availability-vs-metering trade.
4. Tests: two tenants on one gateway — A's actions meter A's row only; B at
   its free cap is refused while A keeps dispatching; the usage endpoint
   returns per-tenant truth. Pin that cwd no longer influences the key
   (e.g. run the check from a renamed cwd and assert the metered tenant is
   the session's).

## Verify

- New multi-tenant seam tests green; existing quota_middleware 16 tests stay green.
- `cargo clippy -p argo-cli --all-targets -- -D warnings` clean.

## May touch

- `crates/argo-cli/src/handlers/gateway/quota_middleware.rs` (+ tests)
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_post.rs` / `routes_dispatch_post_paths.rs` (call sites)
