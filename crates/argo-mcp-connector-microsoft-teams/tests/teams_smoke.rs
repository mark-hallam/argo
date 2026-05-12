//! Live Microsoft Graph Teams smoke test.
//!
//! Gated by `ARGO_TEAMS_TEST_TOKEN`; skips silently when unset so
//! `cargo test -p argo-mcp-connector-microsoft-teams` stays green in CI
//! without leaking credentials.
//!
//! Optional vars narrow what gets exercised:
//!  - `ARGO_TEAMS_TEST_TEAM_ID` — if set, lists channels + members of
//!    this team after listing teams.
//!  - `ARGO_TEAMS_TEST_CHANNEL_ID` + `ARGO_TEAMS_TEST_TEAM_ID` — if both
//!    are set, posts a single message to that channel. (Skipped unless
//!    both are present — accidentally spamming a channel is worse than a
//!    flaky pass.)
//!
//! The test deliberately does NOT exercise `schedule_meeting` because
//! Online Meetings persist server-side and there is no idempotent cleanup
//! path from this connector; running it on every CI run would litter the
//! operator's calendar.

use argo_mcp_connector_microsoft_teams::TeamsConnector;

fn env_or_skip(name: &str) -> Option<String> {
    match std::env::var(name) {
        Ok(v) if !v.trim().is_empty() => Some(v),
        _ => None,
    }
}

#[tokio::test]
async fn live_read_path() {
    let Some(token) = env_or_skip("ARGO_TEAMS_TEST_TOKEN") else {
        eprintln!("skip: ARGO_TEAMS_TEST_TOKEN unset");
        return;
    };

    let teams = TeamsConnector::new(token);
    let joined = teams.list_teams().await.expect("list teams");
    eprintln!("joined teams: {}", joined.len());

    if let Some(team_id) = env_or_skip("ARGO_TEAMS_TEST_TEAM_ID") {
        let channels = teams.list_channels(&team_id).await.expect("list channels");
        eprintln!("channels in {team_id}: {}", channels.len());

        let members = teams
            .list_team_members(&team_id)
            .await
            .expect("list members");
        eprintln!("members in {team_id}: {}", members.len());

        if let Some(channel_id) = env_or_skip("ARGO_TEAMS_TEST_CHANNEL_ID") {
            let msg = teams
                .post_message(
                    &team_id,
                    &channel_id,
                    "argo teams smoke — safe to ignore",
                )
                .await
                .expect("post message");
            assert!(!msg.id.is_empty(), "id should be assigned");
            eprintln!("posted message id: {}", msg.id);
        }
    }
}
