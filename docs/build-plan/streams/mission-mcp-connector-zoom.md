# Mission load-bearing wire — Zoom MCP connector

**Owner**: claude. **Depends on**: existing MCP connector scaffold (`crates/argo-mcp-connector-google-meet/` as canonical OAuth2 video-meet pattern; `crates/argo-mcp-connector-shared/` retry primitive).
**Authored against**: origin/main `55047304` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-mcp-connector-zoom`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 5 #108 ("Add MCP connector Zoom/Microsoft Teams"). The earlier `codex-connector-zoom` brief (PR #1006) was closed-as-obsolete during pre-Mac-migration cleanup; this is the re-queue under the established connector template, narrowed to Zoom Server-to-Server OAuth (Microsoft Teams is a separate follow-up brief — it uses MS Graph). Several verticals (ai-receptionist, customer-support, sales-sdr) need Zoom to schedule + retrieve meeting recordings without leaving the agent loop.

## Scope

NEW crate `crates/argo-mcp-connector-zoom/` mirroring `argo-mcp-connector-google-meet` shape (OAuth2 Server-to-Server auth, meetings + recordings + users endpoints):

```rust
// crates/argo-mcp-connector-zoom/src/lib.rs
pub struct ZoomConnector { client: Client, auth: ZoomS2sAuth }
impl ZoomConnector {
    pub fn new(account_id: String, client_id: String, client_secret: String) -> Self;
    pub async fn create_meeting(&self, host_user_id: &str, draft: ZoomMeetingDraft) -> ArgoResult<ZoomMeeting>;
    pub async fn get_meeting(&self, meeting_id: i64) -> ArgoResult<ZoomMeeting>;
    pub async fn list_meetings(&self, host_user_id: &str, kind: ZoomMeetingKind) -> ArgoResult<Vec<ZoomMeeting>>;
    pub async fn delete_meeting(&self, meeting_id: i64, schedule_reminder: bool) -> ArgoResult<()>;
    pub async fn list_recordings(&self, host_user_id: &str, since: NaiveDate, until: NaiveDate) -> ArgoResult<Vec<ZoomRecording>>;
    pub async fn list_users(&self, status: Option<ZoomUserStatus>) -> ArgoResult<Vec<ZoomUser>>;
}
pub enum ZoomMeetingKind { Scheduled, Live, Upcoming, Previous }
pub enum ZoomUserStatus { Active, Inactive, Pending }
pub struct ZoomMeetingDraft {
    pub topic: String,
    pub start_time: DateTime<Utc>,
    pub duration_minutes: u32,
    pub timezone: String,
    pub agenda: Option<String>,
    pub waiting_room: bool,
    pub join_before_host: bool,
}
pub struct ZoomMeeting { pub id: i64, pub uuid: String, pub host_email: String, pub topic: String, pub start_time: DateTime<Utc>, pub duration_minutes: u32, pub join_url: String, pub password: Option<String> }
pub struct ZoomRecording { pub meeting_id: i64, pub uuid: String, pub recording_start: DateTime<Utc>, pub recording_files: Vec<ZoomRecordingFile> }
pub struct ZoomRecordingFile { pub id: String, pub file_type: String, pub download_url: String, pub file_size_bytes: u64 }
pub struct ZoomUser { pub id: String, pub email: String, pub first_name: String, pub last_name: String, pub status: ZoomUserStatus, pub last_login_time: Option<DateTime<Utc>> }
```

This is NOT in scope: Microsoft Teams (separate brief — uses MS Graph stack); webinars (separate Pro-tier API); breakout rooms; Zoom Phone; Zoom Chat / Team Chat messaging; OAuth user-context (only Server-to-Server app credentials).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-mcp-connector-zoom/` (NEW crate, ≤ 1000 LOC total)
  - `Cargo.toml` (NEW, ≤ 35 LOC)
  - `src/lib.rs` (NEW, ≤ 130 LOC) — re-exports
  - `src/client.rs` (NEW, ≤ 350 LOC) — `ZoomConnector` + REST calls to `https://api.zoom.us/v2/...`
  - `src/auth.rs` (NEW, ≤ 150 LOC) — `ZoomS2sAuth` w/ account-credentials grant + token cache + refresh
  - `src/types.rs` (NEW, ≤ 250 LOC) — DTOs + serde derives
  - `src/client_tests.rs` (NEW, sibling, ≤ 300 LOC) — 6 unit tests w/ wiremock: create-meeting, list-recordings, delete-meeting, list-users-status-filter, token-cache-reuse, 401-refresh
  - `tests/zoom_smoke.rs` (NEW, ≤ 200 LOC) — gated by `ARGO_ZOOM_ACCOUNT_ID` + `ARGO_ZOOM_CLIENT_ID` + `ARGO_ZOOM_CLIENT_SECRET` + `ARGO_ZOOM_TEST_USER_ID`; creates a meeting 5min in future, reads it back, deletes
- `Cargo.toml` (workspace) — narrowly scoped: add `crates/argo-mcp-connector-zoom` to `members`

## Files you must NOT touch

- `crates/argo-mcp-connector-google-meet/**` — sibling stable, mirror its OAuth pattern, don't modify
- `crates/argo-store/migrations/**` — no schema change

## Acceptance criteria

- [ ] NEW `crates/argo-mcp-connector-zoom/` crate compiles standalone
- [ ] `ZoomConnector::new` constructs with account_id + client_id + client_secret
- [ ] Token endpoint: POST `https://zoom.us/oauth/token?grant_type=account_credentials&account_id=<id>` with Basic auth (client_id:client_secret)
- [ ] Token is cached in-memory; reused until `expires_in - 60s` elapsed
- [ ] `create_meeting` POSTs `/v2/users/<host>/meetings` with `type=2` (scheduled)
- [ ] `list_meetings` GETs `/v2/users/<host>/meetings?type=<kind>`
- [ ] `list_recordings` GETs `/v2/users/<host>/recordings?from=<since>&to=<until>` (max 30-day window enforced)
- [ ] 401 response triggers one token-refresh attempt before propagating
- [ ] 6 unit tests with wiremock; integration smoke gated by 4 env vars (cleans up the meeting after)
- [ ] `cargo clippy -p argo-mcp-connector-zoom --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-mcp-connector-zoom` passes
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every new file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-mcp-connector-zoom`
- PR title: `mcp-connector: Zoom Server-to-Server (meetings / recordings / users) (mission-mcp-connector-zoom)`

## Notes for the implementer

- Zoom Server-to-Server OAuth (not user-context OAuth) is the right choice for server-side agents: no consent screen, no refresh-token rotation, account-scoped.
- The Server-to-Server app must be created in the Zoom marketplace under the operator's account; document the setup in `auth.rs` doc-comment.
- Token expiry is ~1h; cache + refresh-on-expiry pattern mirrors stripe / quickbooks.
- 30-day recording-list window is a hard Zoom API constraint — return a typed error for windows larger than that.
- All meeting times are sent as ISO-8601 with `Z` suffix; the `timezone` field is a separate IANA tz string (e.g. `"America/Los_Angeles"`) that Zoom uses for the user-visible time. Document this in the `ZoomMeetingDraft` doc-comment.
- The closed PR #1006 left no salvageable code; start clean from the google-meet template.
