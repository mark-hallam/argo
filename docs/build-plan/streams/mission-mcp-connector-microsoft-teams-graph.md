# Mission load-bearing wire — Microsoft Teams MCP connector (Graph API)

**Owner**: claude.
**Authored against**: origin/main `1cca6a69` on `2026-05-12`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 5 #108. Companion to the existing Zoom connector. Teams is the dominant collaboration platform in Microsoft-shop enterprises. Channel sibling (`mission-channel-microsoft-teams` in backlog #125) handles the notification surface; this brief covers the MCP-tool surface (search channels, post messages, list members, schedule meetings).

## Scope

NEW crate `crates/argo-mcp-connector-microsoft-teams/` via Microsoft Graph:

```rust
pub struct TeamsConnector { client: Client, oauth_token: String }
impl TeamsConnector {
    pub fn new(oauth_token: String) -> Self;
    pub async fn list_teams(&self) -> ArgoResult<Vec<Team>>;
    pub async fn list_channels(&self, team_id: &str) -> ArgoResult<Vec<Channel>>;
    pub async fn post_message(&self, team_id: &str, channel_id: &str, body: &str) -> ArgoResult<Message>;
    pub async fn search_messages(&self, query: &str) -> ArgoResult<Vec<Message>>;
    pub async fn schedule_meeting(&self, subject: &str, attendees: Vec<String>, start: DateTime<Utc>, duration_min: u32) -> ArgoResult<OnlineMeeting>;
    pub async fn list_team_members(&self, team_id: &str) -> ArgoResult<Vec<TeamMember>>;
}
```

This is NOT in scope: Teams app/bot framework (different developer surface); calling / VoIP; presence updates; admin APIs; channel-creation (avoid letting workers spawn channels).

## Files you may touch

- `crates/argo-mcp-connector-microsoft-teams/` (NEW crate, ≤ 1000 LOC)
  - `Cargo.toml`, `src/lib.rs`, `src/client.rs` (≤ 420 LOC), `src/types.rs` (≤ 280 LOC)
  - `src/client_tests.rs` (sibling, ≤ 320 LOC) — 8 wiremock tests
  - `tests/teams_smoke.rs` (≤ 180 LOC) — env-gated by `ARGO_TEAMS_TEST_TOKEN`
- `Cargo.toml` workspace

## Files you must NOT touch

- `argo-mcp-connector-onedrive` (sibling Graph-based connector — share patterns but don't merge state)
- Channel-side `mission-channel-microsoft-teams` (different brief)

## Acceptance criteria

- [ ] All 6 methods + tests
- [ ] Bearer OAuth auth; 401 → typed error with refresh hint
- [ ] `schedule_meeting` creates a Teams Online Meeting (NOT a calendar event — that's the Calendar API; the meeting object includes a join URL)
- [ ] Message body supports both plain text + HTML (Graph allows both; document the difference)
- [ ] `cargo clippy -p argo-mcp-connector-microsoft-teams --all-targets -- -D warnings` clean
- [ ] Every file < 500 LOC; BUILD_STATUS.md updated

## Branch + PR

- Branch: `claude-mission-mcp-connector-microsoft-teams-graph`
- PR title: `mcp-connector: Microsoft Teams via Graph (mission-mcp-connector-microsoft-teams-graph)`

## Notes

- Auth scopes: `Team.ReadBasic.All`, `Channel.ReadBasic.All`, `ChatMessage.Send`, `Calendars.ReadWrite`, `OnlineMeetings.ReadWrite`. Document in `# Notes`.
- Search messages requires the `Mail.Read` + `Chat.Read` scopes — Graph's search-API is unified across mail + Teams.
- Don't expose channel-creation. Workers posting to existing channels is the safety posture; spawning channels invites spam.
