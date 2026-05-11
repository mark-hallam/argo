# Mission load-bearing wire — MCP connector for Apollo.io

**Owner**: claude. **Depends on**: SHIPPED `crates/argo-mcp-connector-clearbit/`, `crates/argo-mcp-connector-zoominfo/` (partial — verify), `crates/argo-mcp-connector-replicate/` as canonical 4-file connector patterns; `crates/argo-mcp-connector-shared/` for `McpClient` + JSON-RPC scaffolding.
**Authored against**: origin/main `c26e63e1` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-mcp-connector-apollo`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 5 #111 ("Add MCP connector ZoomInfo/Apollo/Clearbit" — Clearbit SHIPPED; ZoomInfo partial; Apollo is the remaining clearly-unshipped item). **Tier A (tool).** Apollo.io REST API for prospect-data enrichment, company-enrichment, and search-people. Read-only enrichment in v1; outbound campaigns / sequence management is operator-side (Apollo UI).

## Scope

NEW `crates/argo-mcp-connector-apollo/` crate (≤ 850 LOC across 4 files):

- `crates/argo-mcp-connector-apollo/Cargo.toml` (NEW, ≤ 25 LOC) — `reqwest`, `serde`, `serde_json`, `tokio`, `thiserror`, `tracing`, depend on `argo-mcp-connector-shared`. Dev-dep `wiremock`.
- `crates/argo-mcp-connector-apollo/src/lib.rs` (NEW, ≤ 60 LOC) — re-exports + JSON-RPC method declarations.
- `crates/argo-mcp-connector-apollo/src/client.rs` (NEW, ≤ 380 LOC) — `ApolloClient` against `https://api.apollo.io/api/v1/`:
  - `enrich_person(email | linkedin_url | name+company) -> ApolloPerson` — endpoint `/people/match`
  - `enrich_company(domain | name) -> ApolloCompany` — endpoint `/organizations/enrich`
  - `search_people(filters) -> ApolloPeopleSearchResult` — endpoint `/mixed_people/search` (paginated; up to 25 per page)
  - `search_companies(filters) -> ApolloCompanySearchResult` — endpoint `/mixed_companies/search`
  - Auth: API key header `X-Api-Key: <key>` (env-indirect `ARGO_APOLLO_API_KEY`); Apollo's per-account key
  - Rate-limit handling: Apollo returns `429` w/ `Retry-After`; back-off w/ jitter
  - Credit-quota awareness: Apollo charges credits per enrichment; the connector logs `credits_used` from response headers (tracing-debug) for operator observability
- `crates/argo-mcp-connector-apollo/src/types.rs` (NEW, ≤ 200 LOC) — `ApolloPerson { id, name, title, email, email_status, linkedin_url, phone, company_id, seniority, departments }`, `ApolloCompany { id, name, domain, industry, employee_count, revenue_band, headquarters_city, founded_year }`, `ApolloPeopleSearchResult { entries, page, total_pages, total_entries }`, `ApolloError` enum (`AuthFailed`, `RateLimited { retry_after_secs }`, `CreditsExhausted`, `PersonNotFound`, `CompanyNotFound`, `InvalidFilter`, etc.).
- `crates/argo-mcp-connector-apollo/src/client_tests.rs` (NEW, ≤ 200 LOC, included via `#[path]`) — 8 unit tests using `wiremock`: enrich-person-by-email-happy, enrich-person-by-linkedin-happy, enrich-person-not-found, enrich-company-by-domain-happy, search-people-paginated-2-pages, 429-back-off-w-retry-after, 401-bubbles-as-AuthFailed, credits-exhausted-typed-error.

NEW `crates/argo-mcp-connector-apollo/tests/apollo_smoke.rs` (NEW, ≤ 95 LOC) — env-gated smoke (`ARGO_TEST_APOLLO_API_KEY` + `ARGO_TEST_APOLLO_EMAIL_FOR_ENRICH`). Enriches one person + one company; verifies typed fields populated; no cleanup needed (read-only).

EXTEND `Cargo.toml` (workspace) — add `argo-mcp-connector-apollo` to `members`.

NO migration. NO Rust changes in core crates. Connector is fully self-contained.

This is NOT in scope: outbound email / sequence management (Apollo's `/emailer_messages` endpoint — out-of-scope; operators use Apollo UI); LinkedIn enrichment beyond what Apollo natively provides; bulk-enrichment by CSV upload (v2; one-by-one is sufficient for v1 lead-research-style usage); Apollo's `/dealers` (CRM-style account-management) — out-of-scope; webhook subscription for prospect-engagement updates — out-of-scope.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-mcp-connector-apollo/Cargo.toml` (NEW, ≤ 25 LOC)
- `crates/argo-mcp-connector-apollo/src/lib.rs` (NEW, ≤ 60 LOC)
- `crates/argo-mcp-connector-apollo/src/client.rs` (NEW, ≤ 380 LOC)
- `crates/argo-mcp-connector-apollo/src/types.rs` (NEW, ≤ 200 LOC)
- `crates/argo-mcp-connector-apollo/src/client_tests.rs` (NEW, ≤ 200 LOC)
- `crates/argo-mcp-connector-apollo/tests/apollo_smoke.rs` (NEW, ≤ 95 LOC)
- `Cargo.toml` (workspace EXTEND — add to `members`)

## Files you must NOT touch

- Other connector crates (sibling pattern stays)
- `crates/argo-store/migrations/**` (no migration)
- `crates/argo-channels/**` (Apollo is enrichment, not channel)

## Acceptance criteria

- [ ] `cargo build -p argo-mcp-connector-apollo` succeeds
- [ ] `cargo test -p argo-mcp-connector-apollo` passes all 8 unit tests
- [ ] 429 back-off honors `Retry-After` header w/ jitter
- [ ] 401 → typed `AuthFailed`
- [ ] `CreditsExhausted` distinguishes per-month-quota from per-second-rate-limit
- [ ] Smoke test (env-gated) enriches a person + company + returns typed structs
- [ ] All file sizes within budget; per-file ≤ briefed; total ≤ 850 LOC
- [ ] No migration; no Rust changes outside the new crate + workspace `members`
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-mcp-connector-apollo`
- PR title: `mcp-connector: Apollo.io (person + company enrichment + search; read-only v1) (mission-mcp-connector-apollo)`

## Notes for the implementer

- Treat just-shipped `crates/argo-mcp-connector-clearbit/` (#1352) as the canonical sibling pattern — Apollo overlaps significantly with Clearbit in shape (person enrichment + company enrichment).
- Apollo's `/people/match` endpoint accepts any combination of: `email`, `linkedin_url`, `first_name + last_name + organization_name + domain`. Document that operators can pass any subset; the connector dispatches accordingly.
- `email_status` field in `ApolloPerson` is critical for operators using this for outreach — Apollo returns `verified | unavailable | unverified | likely_to_engage | risky`. Surface it directly; do NOT collapse to a boolean.
- `revenue_band` in `ApolloCompany` is a string range (`"$1M-$10M"`, `"$10M-$50M"`, etc.) — keep as string; don't try to parse to a numeric range.
- Credit tracking: Apollo returns `X-Apollo-Credits-Used` (verify exact header name — check current Apollo API docs at claim time) per request. Log via `tracing::debug!` so operators with tracing-enabled can see credit consumption; do NOT fail if the header is absent.
- DO NOT implement bulk-enrichment (CSV upload). v1 is one-at-a-time; bulk is v2 if needed.
- DO NOT auto-rotate API keys. Operator manages keys in Apollo dashboard; env-var refresh on Argo restart.
- DO NOT implement Apollo's `engagement-tracking` APIs (sequence-step-sent, email-opened, etc.). Out of scope.
- The smoke test should use a known-public email (`bill.gates@microsoft.com` is a canonical example in many enrichment-API docs — operator can substitute) for the smoke recipient. Don't pre-bake personal-identifying data into the test fixture.
- Per Apollo TOS: don't bulk-scrape or store enrichment results long-term beyond the operator's own use case. Document this in the connector's rustdoc as an operator-side compliance note.
