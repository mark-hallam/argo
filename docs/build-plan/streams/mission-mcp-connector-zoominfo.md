# Mission load-bearing wire — MCP connector for ZoomInfo

**Owner**: claude. **Depends on**: SHIPPED `crates/argo-mcp-connector-clearbit/` and authored `crates/argo-mcp-connector-apollo/` (BRIEF: mission-mcp-connector-apollo, parallel) as canonical 4-file enrichment-connector patterns; `crates/argo-mcp-connector-shared/` for `McpClient` + JSON-RPC scaffolding.
**Authored against**: origin/main `84e47150` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-mcp-connector-zoominfo`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 5 #111 ("Add MCP connector ZoomInfo/Apollo/Clearbit" — Clearbit SHIPPED; Apollo briefed; ZoomInfo is the remaining unshipped item). **Tier A (tool).** ZoomInfo Sales Insights REST API for prospect-data enrichment + intent-data lookup. Read-only enrichment in v1; outbound-campaign / intent-subscription flow is operator-side (ZoomInfo dashboard).

## Scope

NEW `crates/argo-mcp-connector-zoominfo/` crate (≤ 850 LOC across 4 files):

- `crates/argo-mcp-connector-zoominfo/Cargo.toml` (NEW, ≤ 25 LOC) — `reqwest`, `serde`, `serde_json`, `tokio`, `thiserror`, `tracing`, depend on `argo-mcp-connector-shared`. Dev-dep `wiremock`.
- `crates/argo-mcp-connector-zoominfo/src/lib.rs` (NEW, ≤ 60 LOC) — re-exports + JSON-RPC method declarations.
- `crates/argo-mcp-connector-zoominfo/src/client.rs` (NEW, ≤ 400 LOC) — `ZoomInfoClient` against `https://api.zoominfo.com/`:
  - `enrich_person(email | first_name+last_name+company_name | linkedin_url) -> ZoomInfoPerson` — endpoint `/lookup/contact`
  - `enrich_company(domain | company_name | duns) -> ZoomInfoCompany` — endpoint `/lookup/company`
  - `search_people(filters) -> ZoomInfoPersonSearchResult` — endpoint `/search/contact` (paginated; up to 25 per page)
  - `search_companies(filters) -> ZoomInfoCompanySearchResult` — endpoint `/search/company`
  - Auth: ZoomInfo uses **OAuth2 client-credentials** (different from Apollo / Clearbit's simple API-key model). Token endpoint: `POST /authenticate` with `{ username, password, client_id }` returns a 1-hour bearer; the connector caches the token in-memory + auto-refreshes ~5 min before expiry.
  - Env-indirect: `ARGO_ZOOMINFO_USERNAME` / `ARGO_ZOOMINFO_PASSWORD` / `ARGO_ZOOMINFO_CLIENT_ID`.
  - Rate-limit handling: ZoomInfo returns `429` with `Retry-After`; back-off w/ jitter `[0, 1s]`.
  - Credit-quota awareness: ZoomInfo charges credits per lookup; the connector logs `X-RateLimit-Remaining-Credits` from response headers via `tracing::debug!` for operator observability.
- `crates/argo-mcp-connector-zoominfo/src/types.rs` (NEW, ≤ 200 LOC) — `ZoomInfoPerson { id, first_name, last_name, email, direct_phone, mobile_phone, job_title, seniority_level, department, company_id, linkedin_url }`, `ZoomInfoCompany { id, name, domain, industry, employee_count, revenue_band, hq_city, hq_country, founded_year, duns }`, `ZoomInfoPersonSearchResult { entries, page, total_pages, total_entries }`, `ZoomInfoError` enum (`AuthFailed`, `TokenExpired`, `RateLimited { retry_after_secs }`, `CreditsExhausted`, `PersonNotFound`, `CompanyNotFound`, `InvalidFilter`, etc.).
- `crates/argo-mcp-connector-zoominfo/src/client_tests.rs` (NEW, ≤ 200 LOC, included via `#[path]`) — 8 unit tests using `wiremock`: enrich-person-by-email-happy, enrich-person-not-found, enrich-company-by-domain-happy, search-people-paginated-2-pages, oauth-token-cached-and-reused, 429-back-off-honors-retry-after, 401-bubbles-as-AuthFailed (vs 403=CreditsExhausted), token-refresh-on-401-then-retry.

NEW `crates/argo-mcp-connector-zoominfo/tests/zoominfo_smoke.rs` (NEW, ≤ 95 LOC) — env-gated smoke (`ARGO_TEST_ZOOMINFO_USERNAME` + `ARGO_TEST_ZOOMINFO_PASSWORD` + `ARGO_TEST_ZOOMINFO_CLIENT_ID` + `ARGO_TEST_ZOOMINFO_EMAIL_FOR_ENRICH`). Enriches one person + one company; verifies typed fields populated; no cleanup needed (read-only).

EXTEND `Cargo.toml` (workspace) — add `argo-mcp-connector-zoominfo` to `members`.

NO migration. NO Rust changes in core crates. Connector is fully self-contained.

This is NOT in scope: ZoomInfo Intent / WebSights (visitor-identification — separate API, requires JS pixel deployment); ZoomInfo Engage (outbound sequence management — operator uses ZoomInfo UI); ZoomInfo Chat (live-chat product — out-of-scope); bulk enrichment by CSV upload (v2; one-by-one is sufficient for v1 lead-research-style usage); ZoomInfo's TalentOS (recruiting-vertical product, separate API surface) — out-of-scope.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-mcp-connector-zoominfo/Cargo.toml` (NEW, ≤ 25 LOC)
- `crates/argo-mcp-connector-zoominfo/src/lib.rs` (NEW, ≤ 60 LOC)
- `crates/argo-mcp-connector-zoominfo/src/client.rs` (NEW, ≤ 400 LOC)
- `crates/argo-mcp-connector-zoominfo/src/types.rs` (NEW, ≤ 200 LOC)
- `crates/argo-mcp-connector-zoominfo/src/client_tests.rs` (NEW, ≤ 200 LOC)
- `crates/argo-mcp-connector-zoominfo/tests/zoominfo_smoke.rs` (NEW, ≤ 95 LOC)
- `Cargo.toml` (workspace EXTEND — add to `members`)

## Files you must NOT touch

- Other connector crates (sibling pattern stays)
- `crates/argo-store/migrations/**` (no migration)
- `crates/argo-channels/**` (ZoomInfo is enrichment, not channel)

## Acceptance criteria

- [ ] `cargo build -p argo-mcp-connector-zoominfo` succeeds
- [ ] `cargo test -p argo-mcp-connector-zoominfo` passes all 8 unit tests
- [ ] OAuth token cached in-memory + auto-refreshed before 1h expiry
- [ ] On 401 the client refreshes token + retries once (handles silent server-side rotation)
- [ ] 429 back-off honors `Retry-After` header w/ jitter
- [ ] `CreditsExhausted` distinguishes per-account-quota from per-second-rate-limit
- [ ] Smoke test (env-gated) enriches a person + company + returns typed structs
- [ ] All file sizes within budget; per-file ≤ briefed; total ≤ 850 LOC
- [ ] No migration; no Rust changes outside the new crate + workspace `members`
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-mcp-connector-zoominfo`
- PR title: `mcp-connector: ZoomInfo (person + company enrichment + search; OAuth2 client-credentials; read-only v1) (mission-mcp-connector-zoominfo)`

## Notes for the implementer

- Treat just-shipped `crates/argo-mcp-connector-clearbit/` (#1352) and the parallel `mission-mcp-connector-apollo` brief as canonical siblings — ZoomInfo overlaps significantly in shape (person + company enrichment + search), but auth model differs.
- ZoomInfo's auth is OAuth2 **client-credentials**, NOT a simple API key — the connector must hold a token + auto-refresh. Cache the token + its expiry timestamp; refresh ~5 min before expiry. On unexpected 401 (server-side rotation / revocation), force-refresh + retry once. Don't retry on the second 401 — bubble `AuthFailed`.
- ZoomInfo uses `/lookup/contact` and `/lookup/company` (singular), not `/people/match` like Apollo. Don't translate endpoint names from Apollo — verify against ZoomInfo's API docs at claim time.
- `seniority_level` in `ZoomInfoPerson` is a string enum (`"C_Suite"`, `"VP"`, `"Director"`, `"Manager"`, `"Senior"`, `"IC"`, `"Entry"`) — keep as `String`; do NOT try to parse into a Rust enum (the set is open-ended and ZoomInfo adds new tiers periodically).
- `revenue_band` in `ZoomInfoCompany` is a string range (`"$1M-$10M"`, `"$10M-$50M"`, etc.) — keep as `String`; don't try to parse to a numeric range.
- Credit tracking: ZoomInfo returns `X-RateLimit-Remaining-Credits` (verify exact header name at claim time — ZoomInfo's API surface has changed naming several times). Log via `tracing::debug!`; do NOT fail if the header is absent.
- DO NOT implement bulk-enrichment (CSV upload). v1 is one-at-a-time; bulk is v2 if operators ask.
- DO NOT auto-rotate the OAuth credentials (`ARGO_ZOOMINFO_USERNAME` / etc.). Operator manages credentials in ZoomInfo dashboard; env-var refresh on Argo restart.
- DO NOT implement ZoomInfo Engage / Intent / WebSights — out of scope.
- The smoke test should use a known-public email + a known-real domain. Don't pre-bake personal-identifying data into the test fixture.
- Per ZoomInfo TOS: don't bulk-scrape or store enrichment results long-term beyond the operator's own use case. Document this in the connector's rustdoc as an operator-side compliance note.
- Coordinate with `mission-mcp-connector-apollo`: both are enrichment connectors with similar shape. Shipping order doesn't matter; both can land independently. Don't share code between them — the sibling-crate pattern keeps connectors isolated.
