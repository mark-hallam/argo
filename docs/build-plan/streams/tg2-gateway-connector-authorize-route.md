# tg2-gateway-connector-authorize-route

**Owner**: claude or codex — gateway / tiered gating (Tier-2 connectors); conductor review + adversarial review before merge (credential surface).
**Depends on**: none (CLI connector-authorize subsystem shipped).
**Authored against**: origin/main `e6a52811f` on `2026-06-12`.
**Validate before claim**: bash scripts/check-brief-freshness.sh tg2-gateway-connector-authorize-route

## Why (operator-directed 2026-06-11: tiered-gating backend)

Tier-2 = connectors/MCP. The connector authorize flow EXISTS but is CLI-only
(verified 2026-06-11: `crates/argo-cli/src/handlers/connector_authorize.rs`
`pub async fn run(cmd: ConnectorCmd)` + `connector_authorize_specs.rs` /
`_paths.rs` / `connector_list_status.rs`; `rg 'connector.*authorize'
crates/argo-cli/src/handlers/gateway/` → zero hits). The locked grant card's
**Connect** button has no HTTP endpoint. A non-technical user cannot run a CLI.

## Scope

1. **Expose authorize over the gateway.** `POST /api/v1/connectors/{id}/authorize`
   + `GET /api/v1/connectors` (list w/ status) routes that REUSE the existing
   handler internals (refactor `connector_authorize.rs` internals into callable
   fns if currently CLI-arg-shaped; do NOT duplicate the spec/path logic).
   Tenant-bound: credentials stored via the existing per-tenant credential
   store path the CLI uses (verify: `argo_config::cred` scoping) — never
   process-global when a tenant is in scope.
2. **RBAC + perimeter**: owner-role required (`Action::` — pick the existing
   nearest action or extend), normal `require_identity` perimeter, CSRF applies
   (cookie callers send `X-CSRF-Token` — the SPA client already auto-retries).
3. **Secrets hygiene**: API keys/tokens in the POST body, NEVER query params;
   never echoed back in responses or audit payloads (audit row records THAT a
   connector was authorized + by whom, not the secret).
4. **Audit row** per authorize (extend existing `AuditKind` only if no fit).

## Acceptance

- SPA-shaped `POST .../authorize` with a body credential authorizes the
  connector for THAT tenant only; second tenant sees it unauthorized
  (cross-tenant negative pin).
- Guest/anonymous → 401/403; secret never appears in response/audit/logs
  (test greps the audit payload).
- CLI path keeps working unchanged (regression).
- 🔴 ADVERSARIAL REVIEW BEFORE MERGE (credential-handling surface).

## Parallel-surface check

`rg 'connector' crates/argo-cli/src/handlers/gateway/` → none today. Reuse
`connector_authorize*.rs` internals + the gateway route-dispatch pattern
(`routes_dispatch_post.rs`); do not fork a second authorize implementation.
