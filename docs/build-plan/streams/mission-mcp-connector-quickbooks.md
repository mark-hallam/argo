# Mission load-bearing wire — QuickBooks MCP connector

**Owner**: claude. **Depends on**: existing MCP connector scaffold (`crates/argo-mcp-connector-stripe/` as canonical OAuth-app pattern; `crates/argo-mcp-connector-shared/` retry primitive); just-merged `verticals/bookkeeping-assistant/` (#1327) which calls out QuickBooks as the optional MCP integration target.
**Authored against**: origin/main `d7c20e4d` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-mcp-connector-quickbooks`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 5 #109 ("Add MCP connector QuickBooks/Xero"). The earlier `codex-connector-quickbooks` brief (PR #986) was closed-as-obsolete during pre-Mac-migration cleanup; this is the re-queue under the established connector template, narrowed to QuickBooks Online (Xero is a separate follow-up brief). The just-merged bookkeeping-assistant vertical (#1327) needs this connector to graduate from "ledger-only / paper-only" to "writes journal entries into a real GL".

## Scope

NEW crate `crates/argo-mcp-connector-quickbooks/` mirroring `argo-mcp-connector-stripe` shape (OAuth2-app auth, env-keyed access-token refresh, customer / invoice / expense endpoints):

```rust
// crates/argo-mcp-connector-quickbooks/src/lib.rs
pub struct QuickBooksConnector {
    client: Client,
    auth: QuickBooksAuth,        // wraps oauth2 refresh; reuses argo-mcp-connector-shared::oauth_refresh
    realm_id: String,             // QBO "company ID"
    base_url: QuickBooksHost,     // Sandbox | Production
}
pub enum QuickBooksHost { Sandbox, Production }

impl QuickBooksConnector {
    pub fn new(auth: QuickBooksAuth, realm_id: String, host: QuickBooksHost) -> Self;
    pub async fn list_customers(&self, query: Option<&str>) -> ArgoResult<Vec<QbCustomer>>;
    pub async fn create_invoice(&self, draft: QbInvoiceDraft) -> ArgoResult<QbInvoice>;
    pub async fn list_invoices(&self, since: Option<DateTime<Utc>>) -> ArgoResult<Vec<QbInvoice>>;
    pub async fn record_expense(&self, draft: QbExpenseDraft) -> ArgoResult<QbExpense>;
    pub async fn chart_of_accounts(&self) -> ArgoResult<Vec<QbAccount>>;
}
pub struct QbCustomer { pub id: String, pub display_name: String, pub email: Option<String>, pub balance_cents: i64 }
pub struct QbInvoiceDraft { pub customer_id: String, pub line_items: Vec<QbLine>, pub due_date: NaiveDate }
pub struct QbInvoice { pub id: String, pub doc_number: String, pub total_cents: i64, pub status: QbDocStatus, pub web_url: String }
pub struct QbExpenseDraft { pub account_id: String, pub vendor_id: Option<String>, pub amount_cents: i64, pub memo: Option<String>, pub txn_date: NaiveDate }
pub struct QbAccount { pub id: String, pub name: String, pub account_type: String, pub balance_cents: i64 }
```

This is NOT in scope: Xero (separate follow-up brief with similar pattern); bill-pay / vendor-payments (operator-driven approval surface needed); tax filings; QuickBooks Desktop (only Online is wired); inbound webhooks for QBO event push (separate brief once the outbound CRUD lands).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-mcp-connector-quickbooks/` (NEW crate, ≤ 1100 LOC total)
  - `Cargo.toml` (NEW, ≤ 35 LOC)
  - `src/lib.rs` (NEW, ≤ 130 LOC) — re-exports
  - `src/client.rs` (NEW, ≤ 400 LOC) — `QuickBooksConnector` + REST calls (`https://{sandbox-,}quickbooks.api.intuit.com/v3/company/{realm_id}/...`)
  - `src/auth.rs` (NEW, ≤ 150 LOC) — `QuickBooksAuth` w/ refresh flow
  - `src/types.rs` (NEW, ≤ 250 LOC) — all DTOs + serde derives
  - `src/client_tests.rs` (NEW, sibling, ≤ 300 LOC) — 6 unit tests w/ wiremock: list-customers, create-invoice, record-expense, chart-of-accounts, 401-refresh-retry, sandbox-vs-production URL routing
  - `tests/quickbooks_smoke.rs` (NEW, ≤ 200 LOC) — gated by `ARGO_QB_TEST_TOKEN` + `ARGO_QB_TEST_REALM` + `ARGO_QB_TEST_CUSTOMER_ID`; creates a small sandbox invoice, reads it back, asserts byte-round-trip
- `Cargo.toml` (workspace) — narrowly scoped: add `crates/argo-mcp-connector-quickbooks` to `members`

## Files you must NOT touch

- `crates/argo-mcp-connector-stripe/**` — sibling stable, mirror its OAuth pattern, don't modify
- `verticals/bookkeeping-assistant/**` — wiring the vertical's `optional_mcp` array to point at this connector is a follow-up
- `crates/argo-store/migrations/**` — no schema change

## Acceptance criteria

- [ ] NEW `crates/argo-mcp-connector-quickbooks/` crate compiles standalone
- [ ] `QuickBooksConnector::new` constructs with auth + realm + host
- [ ] `list_customers` GETs `/v3/company/{realm}/query?query=SELECT * FROM Customer` with bearer auth
- [ ] `create_invoice` POSTs `/v3/company/{realm}/invoice` with line-items JSON; returns `QbInvoice` with `web_url`
- [ ] `record_expense` POSTs `/v3/company/{realm}/purchase` (QBO expense entity)
- [ ] `chart_of_accounts` GETs `/v3/company/{realm}/query?query=SELECT * FROM Account`
- [ ] Sandbox vs Production host routing via `QuickBooksHost` enum
- [ ] 401 response triggers one access-token refresh attempt (via auth.rs) before propagating
- [ ] 6 unit tests with wiremock; integration smoke gated by 3 env vars
- [ ] `cargo clippy -p argo-mcp-connector-quickbooks --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-mcp-connector-quickbooks` passes
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every new file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-mcp-connector-quickbooks`
- PR title: `mcp-connector: QuickBooks Online (customers / invoices / expenses / accounts) (mission-mcp-connector-quickbooks)`

## Notes for the implementer

- QuickBooks Online uses OAuth2 with both access-token (1h) and refresh-token (100d) — the refresh handler in `auth.rs` should mirror Stripe's pattern.
- `realm_id` is the QBO "company ID" and is part of every URL — keep it as a struct field, not a per-call arg.
- Minor cent representation: QBO amounts are decimal dollars (e.g. `12.34`) — convert to/from cents (`i64`) at the boundary; don't expose floats publicly.
- All QBO entity reads are via the `?query=SELECT * FROM <Entity>` SQL-like syntax (no REST GET-by-ID for collections). For single-entity reads use `GET /v3/company/{realm}/<entity>/<id>` directly.
- Retriable errors: 429, 5xx, 401-on-first-call (token-expiry). Reuse `argo-mcp-connector-shared::retry` if it exists; otherwise leave retry to caller.
- The closed PR #986 left no salvageable code (was per-codex-template); start clean from the Stripe template.
