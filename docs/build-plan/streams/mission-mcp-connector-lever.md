# Mission load-bearing wire — Lever ATS MCP connector

**Owner**: claude. **Depends on**: existing ATS connector scaffold (`argo-mcp-connector-greenhouse/`).
**Authored against**: origin/main `1cca6a69` on `2026-05-12`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 5 #110. Lever is the second major ATS (after Greenhouse) for tech-company recruiting. Paired with the existing Greenhouse connector, this closes the major-ATS gap for the recruiter + recruiter-screener verticals.

## Scope

NEW crate `crates/argo-mcp-connector-lever/`:

```rust
pub struct LeverConnector { client: Client, api_key: String }
impl LeverConnector {
    pub fn new(api_key: String) -> Self;
    pub async fn list_candidates(&self, since: Option<DateTime<Utc>>) -> ArgoResult<Vec<LeverCandidate>>;
    pub async fn get_candidate(&self, candidate_id: &str) -> ArgoResult<Option<LeverCandidate>>;
    pub async fn add_note(&self, candidate_id: &str, note: &str) -> ArgoResult<()>;
    pub async fn advance_stage(&self, candidate_id: &str, stage_id: &str) -> ArgoResult<()>;
    pub async fn list_postings(&self) -> ArgoResult<Vec<LeverPosting>>;
    pub async fn list_stages(&self) -> ArgoResult<Vec<LeverStage>>;
}
```

This is NOT in scope: Lever Hire vs Lever Nurture API split (this is Hire only); offer-letter generation; bulk-import; admin/permissions APIs.

## Files you may touch

- `crates/argo-mcp-connector-lever/` (NEW crate, ≤ 900 LOC)
  - `Cargo.toml`, `src/lib.rs`, `src/client.rs` (≤ 380 LOC), `src/types.rs` (≤ 280 LOC)
  - `src/client_tests.rs` (sibling, ≤ 280 LOC) — 7 wiremock tests
  - `tests/lever_smoke.rs` (≤ 150 LOC) — env-gated
- `Cargo.toml` workspace

## Files you must NOT touch

- `argo-mcp-connector-greenhouse` (sibling — different API shape; don't drift either)

## Acceptance criteria

- [ ] All 6 methods + tests
- [ ] Basic auth: `Authorization: Basic <base64(api_key:)>` (Lever's pattern — empty password)
- [ ] Pagination handled via `cursor` query param (Lever's pagination style)
- [ ] `advance_stage` validates the target stage exists (else typed error)
- [ ] Integration smoke env-gated by `ARGO_LEVER_TEST_API_KEY`
- [ ] `cargo clippy -p argo-mcp-connector-lever --all-targets -- -D warnings` clean
- [ ] Every file < 500 LOC; BUILD_STATUS.md updated

## Branch + PR

- Branch: `claude-mission-mcp-connector-lever`
- PR title: `mcp-connector: Lever ATS (mission-mcp-connector-lever)`

## Notes

- Lever's auth pattern (Basic auth with empty password) is shared with Clearbit — copy the pattern from `argo-mcp-connector-clearbit/src/client.rs`.
- Stage IDs are per-tenant — don't hardcode any. List them via `list_stages` then pick by name match.
- Pair with `argo-mcp-connector-greenhouse` for full major-ATS coverage in `recruiter` + `recruiter-screener` verticals.
