# Mission load-bearing wire — Clearbit MCP connector

**Owner**: claude. **Depends on**: existing MCP connector scaffold (`crates/argo-mcp-connector-hubspot/` as canonical Basic-auth CRM enrichment pattern; `crates/argo-mcp-connector-shared/` retry primitive).
**Authored against**: origin/main `55047304` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-mcp-connector-clearbit`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 5 #111 ("Add MCP connector ZoomInfo/Apollo/Clearbit"). The lead-research vertical (just-merged #1331) calls out clearbit as an optional MCP. Clearbit is the smallest of the three (single REST API, public docs) — ZoomInfo and Apollo are separate follow-up briefs once this template lands.

## Scope

NEW crate `crates/argo-mcp-connector-clearbit/` mirroring `argo-mcp-connector-hubspot` shape (Basic-auth with API key, person + company enrichment endpoints):

```rust
// crates/argo-mcp-connector-clearbit/src/lib.rs
pub struct ClearbitConnector { client: Client, api_key: String }
impl ClearbitConnector {
    pub fn new(api_key: String) -> Self;
    pub async fn enrich_person_by_email(&self, email: &str) -> ArgoResult<Option<ClearbitPerson>>;
    pub async fn enrich_company_by_domain(&self, domain: &str) -> ArgoResult<Option<ClearbitCompany>>;
    pub async fn combined_lookup(&self, email: &str) -> ArgoResult<ClearbitCombinedLookup>;  // person + their company in one call
    pub async fn discovery_search(&self, query: &str, limit: u8) -> ArgoResult<Vec<ClearbitCompany>>;
}
pub struct ClearbitPerson {
    pub id: String,
    pub full_name: String,
    pub email: String,
    pub linkedin_handle: Option<String>,
    pub twitter_handle: Option<String>,
    pub company_name: Option<String>,
    pub company_domain: Option<String>,
    pub title: Option<String>,
    pub seniority: Option<String>,
    pub role: Option<String>,
    pub location: Option<String>,
}
pub struct ClearbitCompany {
    pub id: String,
    pub name: String,
    pub domain: String,
    pub industry: Option<String>,
    pub category_industry: Option<String>,
    pub employees: Option<u32>,
    pub annual_revenue_usd: Option<u64>,
    pub funding_total_usd: Option<u64>,
    pub tech_stack: Vec<String>,
    pub linkedin_handle: Option<String>,
    pub twitter_handle: Option<String>,
}
pub struct ClearbitCombinedLookup { pub person: Option<ClearbitPerson>, pub company: Option<ClearbitCompany> }
```

Clearbit returns 202 (Accepted) when enrichment is queued asynchronously. The connector polls with backoff until 200 or returns `None` after 30s timeout (typed `ArgoError::Provider` w/ hint).

This is NOT in scope: ZoomInfo (separate follow-up brief — different auth + data shape); Apollo (separate); Clearbit Reveal (IP-to-company); Clearbit Prospector (list-building); webhook-events ingest; bulk enrichment endpoints (per-row only for v1); subscribe-to-changes feed.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-mcp-connector-clearbit/` (NEW crate, ≤ 900 LOC total)
  - `Cargo.toml` (NEW, ≤ 30 LOC)
  - `src/lib.rs` (NEW, ≤ 120 LOC) — re-exports
  - `src/client.rs` (NEW, ≤ 350 LOC) — `ClearbitConnector` + REST (`https://person.clearbit.com/v2/people/find`, `https://company.clearbit.com/v2/companies/find`, `https://discovery.clearbit.com/v1/companies/search`)
  - `src/types.rs` (NEW, ≤ 250 LOC) — DTOs + serde derives
  - `src/client_tests.rs` (NEW, sibling, ≤ 250 LOC) — 6 unit tests w/ wiremock: person-found, person-not-found-202-polling-eventual-404, company-by-domain, combined-lookup, discovery-search, 202-async-polls-then-200
  - `tests/clearbit_smoke.rs` (NEW, ≤ 150 LOC) — gated by `ARGO_CLEARBIT_TEST_KEY` + `ARGO_CLEARBIT_TEST_EMAIL`; enrich a known email, assert person + company populate
- `Cargo.toml` (workspace) — narrowly scoped: add `crates/argo-mcp-connector-clearbit` to `members`

## Files you must NOT touch

- `crates/argo-mcp-connector-hubspot/**` — sibling stable, mirror its Basic-auth pattern, don't modify
- `verticals/lead-research/**` — wiring the vertical's `optional_mcp` is a follow-up
- `crates/argo-store/migrations/**` — no schema change

## Acceptance criteria

- [ ] NEW `crates/argo-mcp-connector-clearbit/` crate compiles standalone
- [ ] `ClearbitConnector::new` constructs with api_key
- [ ] Basic auth header: `Authorization: Basic <base64(api_key:)>` (empty password)
- [ ] 200 → returns `Some(...)`; 404 → returns `None` (NOT an error — caller decides if absent person is a problem); 202 → poll with backoff (1s, 2s, 4s, 8s, 15s) up to 30s ceiling
- [ ] `combined_lookup` uses `https://person.clearbit.com/v2/combined/find?email=<email>`
- [ ] `discovery_search` uses `https://discovery.clearbit.com/v1/companies/search?query=<q>&limit=<n>`
- [ ] 6 unit tests with wiremock; integration smoke gated by 2 env vars
- [ ] `cargo clippy -p argo-mcp-connector-clearbit --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-mcp-connector-clearbit` passes
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every new file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-mcp-connector-clearbit`
- PR title: `mcp-connector: Clearbit person + company enrichment (mission-mcp-connector-clearbit)`

## Notes for the implementer

- Clearbit uses three distinct subdomains (`person.`, `company.`, `discovery.`) — host is a function of the operation. Route in client.rs based on which endpoint is called.
- 202-Accepted polling: this is Clearbit's "we don't have the data cached but are fetching it now" signal. Don't conflate with 429 (rate limit). Both can return `Retry-After` header — honor it.
- Privacy note: Clearbit lookups are GDPR-sensitive — the connector itself is dumb (no consent management), but the calling vertical's prompts/persona should establish operator-side basis. Document this in module-level docs.
- `None` vs error: a 404 means "person not in Clearbit" — that's expected, not an error. A 401 means "your key is bad" — that IS an error. Don't conflate the two.
- The integration smoke uses a real Clearbit key + real email; pick a stable public-figure email (e.g. founder of a well-known co) so the test doesn't flake on absent data.
