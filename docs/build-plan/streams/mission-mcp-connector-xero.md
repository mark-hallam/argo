# Mission load-bearing wire — Xero MCP connector

**Owner**: claude. **Depends on**: existing accounting-connector scaffold (`argo-mcp-connector-quickbooks/`).
**Authored against**: origin/main `1cca6a69` on `2026-05-12`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 5 #109. Xero is the dominant cloud accounting platform in AU/NZ/UK (and growing in US). Paired with the existing QuickBooks connector, this closes the small-business accounting MCP gap.

## Scope

NEW crate `crates/argo-mcp-connector-xero/` mirroring QuickBooks shape:

```rust
pub struct XeroConnector { client: Client, oauth_token: String, tenant_id: String }
impl XeroConnector {
    pub fn new(oauth_token: String, tenant_id: String) -> Self;
    pub async fn list_invoices(&self, status: Option<XeroInvoiceStatus>) -> ArgoResult<Vec<XeroInvoice>>;
    pub async fn get_invoice(&self, invoice_id: &str) -> ArgoResult<Option<XeroInvoice>>;
    pub async fn create_invoice(&self, draft: NewXeroInvoice) -> ArgoResult<XeroInvoice>;
    pub async fn list_contacts(&self) -> ArgoResult<Vec<XeroContact>>;
    pub async fn record_payment(&self, invoice_id: &str, payment: NewPayment) -> ArgoResult<XeroPayment>;
    pub async fn list_accounts(&self) -> ArgoResult<Vec<XeroAccount>>;
}
```

This is NOT in scope: Xero Payroll (separate API + jurisdiction-specific); reports / Xero Analytics; multi-currency conversion (operator-side); admin/practice-management APIs.

## Files you may touch

- `crates/argo-mcp-connector-xero/` (NEW crate, ≤ 1000 LOC)
  - `Cargo.toml`, `src/lib.rs`, `src/client.rs` (≤ 420 LOC), `src/types.rs` (≤ 280 LOC)
  - `src/client_tests.rs` (sibling, ≤ 280 LOC) — 7 wiremock tests
  - `tests/xero_smoke.rs` (≤ 150 LOC) — env-gated (`ARGO_XERO_TEST_TOKEN` + `ARGO_XERO_TEST_TENANT_ID`)
- `Cargo.toml` workspace

## Files you must NOT touch

- `argo-mcp-connector-quickbooks` (sibling stable, mirror its OAuth pattern)
- finance-ar vertical wiring (downstream brief)

## Acceptance criteria

- [ ] All 6 methods + tests
- [ ] `tenant_id` header included on every request (Xero multi-tenancy)
- [ ] Invoice status enum covers DRAFT/SUBMITTED/AUTHORISED/PAID/VOIDED
- [ ] OAuth 2.0 bearer auth; 401 → typed error
- [ ] Integration smoke env-gated by 2 vars
- [ ] `cargo clippy -p argo-mcp-connector-xero --all-targets -- -D warnings` clean
- [ ] Every file < 500 LOC; BUILD_STATUS.md updated

## Branch + PR

- Branch: `claude-mission-mcp-connector-xero`
- PR title: `mcp-connector: Xero accounting (mission-mcp-connector-xero)`

## Notes

- Xero requires `xero-tenant-id` header on every request — this is per-organization, NOT per-user. A consultant connecting multiple client orgs picks the tenant ID per call.
- Date format: Xero uses Microsoft .NET-style epoch (`/Date(1234567890)/`) in some responses. Serde tolerance helpers in `types.rs` need this.
- finance-ar vertical's `reconcile_invoice` skill is the highest-value downstream consumer.
