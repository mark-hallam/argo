# Mission load-bearing wire — Greenhouse MCP connector

**Owner**: claude. **Depends on**: existing MCP connector scaffold (`crates/argo-mcp-connector-hubspot/` as canonical Basic-auth CRM pattern; `crates/argo-mcp-connector-shared/` retry primitive); existing `verticals/recruiter-screener/` vertical which calls out Greenhouse as its optional MCP integration.
**Authored against**: origin/main `d7c20e4d` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-mcp-connector-greenhouse`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 5 #110 ("Add MCP connector Greenhouse/Lever"). The recruiter-screener vertical needs Greenhouse to graduate from "score-resume / draft-outreach in isolation" to "fetch the live candidate pipeline and write advance/reject decisions back to the ATS". Lever follows the same pattern and is a separate follow-up brief.

## Scope

NEW crate `crates/argo-mcp-connector-greenhouse/` mirroring `argo-mcp-connector-hubspot` shape (Basic-auth with API key, candidate / application / job endpoints, write-back of advance/reject):

```rust
// crates/argo-mcp-connector-greenhouse/src/lib.rs
pub struct GreenhouseConnector { client: Client, api_key: String }
impl GreenhouseConnector {
    pub fn new(api_key: String) -> Self;
    pub async fn list_candidates(&self, since: Option<DateTime<Utc>>) -> ArgoResult<Vec<GhCandidate>>;
    pub async fn get_candidate(&self, id: i64) -> ArgoResult<GhCandidate>;
    pub async fn list_applications(&self, candidate_id: Option<i64>) -> ArgoResult<Vec<GhApplication>>;
    pub async fn list_jobs(&self, status: Option<GhJobStatus>) -> ArgoResult<Vec<GhJob>>;
    pub async fn advance_application(&self, app_id: i64, to_stage_id: i64, on_behalf_of_user_id: i64) -> ArgoResult<GhApplication>;
    pub async fn reject_application(&self, app_id: i64, rejection_reason_id: i64, on_behalf_of_user_id: i64, notes: Option<&str>) -> ArgoResult<GhApplication>;
    pub async fn add_note(&self, candidate_id: i64, on_behalf_of_user_id: i64, body: &str, visibility: GhNoteVisibility) -> ArgoResult<()>;
}
pub struct GhCandidate { pub id: i64, pub first_name: String, pub last_name: String, pub email_addresses: Vec<String>, pub applications: Vec<i64> }
pub struct GhApplication { pub id: i64, pub candidate_id: i64, pub job_id: i64, pub current_stage: GhStage, pub status: GhApplicationStatus, pub source: Option<String> }
pub struct GhJob { pub id: i64, pub name: String, pub status: GhJobStatus, pub departments: Vec<String>, pub offices: Vec<String> }
pub enum GhApplicationStatus { Active, Hired, Rejected, Converted }
pub enum GhJobStatus { Open, Closed, Draft }
pub enum GhNoteVisibility { Admin, Public, Private }
```

This is NOT in scope: Lever (separate follow-up brief with similar pattern); scorecards/interview-kit endpoints (out for v1, scorecard writes go through `add_note` for now); webhook-event ingest from Greenhouse (separate brief once outbound CRUD lands); offers/onboarding endpoints; custom-fields beyond standard candidate/application.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-mcp-connector-greenhouse/` (NEW crate, ≤ 1000 LOC total)
  - `Cargo.toml` (NEW, ≤ 30 LOC)
  - `src/lib.rs` (NEW, ≤ 130 LOC) — re-exports
  - `src/client.rs` (NEW, ≤ 400 LOC) — `GreenhouseConnector` + REST (`https://harvest.greenhouse.io/v1/`) with Basic auth (`api_key:`, password empty)
  - `src/types.rs` (NEW, ≤ 250 LOC) — DTOs + serde derives
  - `src/client_tests.rs` (NEW, sibling, ≤ 300 LOC) — 6 unit tests w/ wiremock: list-candidates, advance-application, reject-application, add-note, On-Behalf-Of header roundtrip, 429-retry-after
  - `tests/greenhouse_smoke.rs` (NEW, ≤ 200 LOC) — gated by `ARGO_GH_TEST_API_KEY` + `ARGO_GH_TEST_ON_BEHALF_OF` + `ARGO_GH_TEST_CANDIDATE_ID`; reads back a candidate, adds a note, removes the note
- `Cargo.toml` (workspace) — narrowly scoped: add `crates/argo-mcp-connector-greenhouse` to `members`

## Files you must NOT touch

- `crates/argo-mcp-connector-hubspot/**` — sibling stable, mirror its Basic-auth pattern, don't modify
- `verticals/recruiter-screener/**` — wiring the vertical's `optional_mcp` to point at this connector is a follow-up
- `crates/argo-store/migrations/**` — no schema change

## Acceptance criteria

- [ ] NEW `crates/argo-mcp-connector-greenhouse/` crate compiles standalone
- [ ] `GreenhouseConnector::new` constructs with api_key
- [ ] All endpoints use Harvest base URL `https://harvest.greenhouse.io/v1/`
- [ ] Basic auth header: `Authorization: Basic <base64(api_key:)>` (note empty password)
- [ ] `advance_application` and `reject_application` POST with required `On-Behalf-Of: <user_id>` header
- [ ] `add_note` POST `/v1/candidates/<id>/activity_feed/notes` with visibility enum
- [ ] 429 response respects `Retry-After` header (parse seconds, sleep, single retry)
- [ ] 6 unit tests with wiremock; integration smoke gated by 3 env vars
- [ ] `cargo clippy -p argo-mcp-connector-greenhouse --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-mcp-connector-greenhouse` passes
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every new file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-mcp-connector-greenhouse`
- PR title: `mcp-connector: Greenhouse Harvest API (candidates / applications / jobs / decisions) (mission-mcp-connector-greenhouse)`

## Notes for the implementer

- Greenhouse uses two distinct APIs: **Harvest** (full CRUD; this brief) and **Job Board** (public-facing read-only). Stay on Harvest.
- Basic auth password is intentionally empty: `Authorization: Basic <base64("api_key:")>`. Document this in `auth.rs` doc-comment.
- `On-Behalf-Of` header is **required** for any write that has compliance audit implications (advance/reject/add-note). The connector takes this as an explicit parameter; if absent it returns a typed error before any network call.
- Pagination: Greenhouse returns `Link: <...>; rel="next"` header. For v1, expose `since: Option<DateTime<Utc>>` filtering and document the absence of cursor pagination as a limitation; full cursor support is a follow-up.
- The `add_note` body should support inline mentions (`@user-id`) per Greenhouse syntax — pass-through as opaque string for v1, don't try to parse.
