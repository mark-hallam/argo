//! Wiremock-backed unit tests for `TeamsConnector`.
//!
//! Eight scenarios:
//!  1. `list_teams` GETs `/me/joinedTeams` and decodes `value[]`.
//!  2. `list_channels` GETs the team-scoped channels path.
//!  3. `post_message` POSTs `body.contentType=text` to the channel.
//!  4. `post_message_html` POSTs `body.contentType=html`.
//!  5. `search_messages` POSTs `/search/query` with `chatMessage` and
//!     flattens `value[].hitsContainers[].hits[].resource`.
//!  6. `schedule_meeting` POSTs `/me/onlineMeetings` with end =
//!     start + duration_min, NOT a calendar event.
//!  7. `list_team_members` GETs the team-scoped members path.
//!  8. A 401 from any endpoint returns `ArgoError::Provider` carrying
//!     the "refresh access token" hint.

use super::*;
use crate::types::MessageContentType;
use chrono::{DateTime, Utc};
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn connector(server: &MockServer) -> TeamsConnector {
    TeamsConnector::new("seed-access-token".into()).with_base_url(server.uri())
}

fn sample_start() -> DateTime<Utc> {
    DateTime::<Utc>::from_timestamp(1_900_000_000, 0).expect("ts")
}

#[tokio::test]
async fn list_teams_decodes_value_array() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/me/joinedTeams"))
        .and(header("authorization", "Bearer seed-access-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "value": [
                { "id": "t-1", "displayName": "Skycot Eng", "description": "core" },
                { "id": "t-2", "displayName": "Skycot Ops", "description": "" }
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let teams = connector(&server).list_teams().await.unwrap();
    assert_eq!(teams.len(), 2);
    assert_eq!(teams[0].id, "t-1");
    assert_eq!(teams[0].display_name, "Skycot Eng");
    assert_eq!(teams[0].description.as_deref(), Some("core"));
    // Empty description normalised to None.
    assert_eq!(teams[1].description, None);
}

#[tokio::test]
async fn list_channels_uses_team_scoped_path() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/teams/t-1/channels"))
        .and(header("authorization", "Bearer seed-access-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "value": [
                {
                    "id": "19:abc@thread.tacv2",
                    "displayName": "General",
                    "membershipType": "standard"
                },
                {
                    "id": "19:def@thread.tacv2",
                    "displayName": "Incidents",
                    "membershipType": "private"
                }
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let channels = connector(&server).list_channels("t-1").await.unwrap();
    assert_eq!(channels.len(), 2);
    assert_eq!(channels[0].display_name, "General");
    assert_eq!(channels[0].membership_type, "standard");
    assert_eq!(channels[1].membership_type, "private");
}

#[tokio::test]
async fn post_message_sends_text_content_type() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/teams/t-1/channels/19:abc@thread.tacv2/messages"))
        .and(header("authorization", "Bearer seed-access-token"))
        .and(header("content-type", "application/json"))
        .and(body_json(serde_json::json!({
            "body": { "contentType": "text", "content": "ship it" }
        })))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "id": "msg-1",
            "createdDateTime": "2026-05-13T10:00:00Z",
            "body": { "contentType": "text", "content": "ship it" },
            "from": { "user": { "displayName": "Mark" } },
            "webUrl": "https://teams.microsoft.com/l/message/19:abc/msg-1"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let msg = connector(&server)
        .post_message("t-1", "19:abc@thread.tacv2", "ship it")
        .await
        .unwrap();
    assert_eq!(msg.id, "msg-1");
    assert_eq!(msg.content_type, MessageContentType::Text);
    assert_eq!(msg.body, "ship it");
    assert_eq!(msg.from_display_name.as_deref(), Some("Mark"));
}

#[tokio::test]
async fn post_message_html_sets_html_content_type() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/teams/t-1/channels/19:abc@thread.tacv2/messages"))
        .and(body_json(serde_json::json!({
            "body": {
                "contentType": "html",
                "content": "<b>ship it</b>"
            }
        })))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "id": "msg-2",
            "body": { "contentType": "html", "content": "<b>ship it</b>" }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let msg = connector(&server)
        .post_message_html("t-1", "19:abc@thread.tacv2", "<b>ship it</b>")
        .await
        .unwrap();
    assert_eq!(msg.content_type, MessageContentType::Html);
    assert_eq!(msg.body, "<b>ship it</b>");
}

#[tokio::test]
async fn search_messages_flattens_hits_containers() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/search/query"))
        .and(header("authorization", "Bearer seed-access-token"))
        .and(body_json(serde_json::json!({
            "requests": [
                {
                    "entityTypes": ["chatMessage"],
                    "query": { "queryString": "rollout plan" }
                }
            ]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "value": [
                {
                    "hitsContainers": [
                        {
                            "hits": [
                                {
                                    "resource": {
                                        "id": "msg-7",
                                        "body": {
                                            "contentType": "text",
                                            "content": "rollout plan for monday"
                                        },
                                        "createdDateTime": "2026-05-12T09:00:00Z",
                                        "from": { "user": { "displayName": "Sue" } },
                                        "webUrl": "https://teams.microsoft.com/l/message/msg-7"
                                    }
                                },
                                {
                                    "resource": {
                                        "id": "msg-8",
                                        "body": {
                                            "contentType": "html",
                                            "content": "<i>rollout plan v2</i>"
                                        }
                                    }
                                }
                            ]
                        }
                    ]
                }
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let hits = connector(&server)
        .search_messages("rollout plan")
        .await
        .unwrap();
    assert_eq!(hits.len(), 2);
    assert_eq!(hits[0].id, "msg-7");
    assert_eq!(hits[0].from_display_name.as_deref(), Some("Sue"));
    assert_eq!(hits[1].id, "msg-8");
    assert_eq!(hits[1].content_type, MessageContentType::Html);
}

#[tokio::test]
async fn schedule_meeting_creates_online_meeting_not_calendar_event() {
    let server = MockServer::start().await;
    let start = sample_start();
    Mock::given(method("POST"))
        .and(path("/me/onlineMeetings"))
        .and(header("authorization", "Bearer seed-access-token"))
        .and(body_json(serde_json::json!({
            "startDateTime": "2030-03-17T17:46:40Z",
            "endDateTime": "2030-03-17T18:16:40Z",
            "subject": "Outage post-mortem",
            "participants": {
                "attendees": [
                    { "upn": "ali@example.com", "role": "attendee" },
                    { "upn": "bo@example.com", "role": "attendee" }
                ]
            }
        })))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "id": "om-1",
            "subject": "Outage post-mortem",
            "startDateTime": "2030-03-17T17:46:40Z",
            "endDateTime": "2030-03-17T18:16:40Z",
            "joinWebUrl": "https://teams.microsoft.com/l/meetup-join/om-1"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let meeting = connector(&server)
        .schedule_meeting(
            "Outage post-mortem",
            vec!["ali@example.com".into(), "bo@example.com".into()],
            start,
            30,
        )
        .await
        .unwrap();
    assert_eq!(meeting.id, "om-1");
    assert_eq!(meeting.subject, "Outage post-mortem");
    assert!(meeting.join_web_url.contains("teams.microsoft.com"));
    // 30-minute window.
    assert_eq!((meeting.end - meeting.start).num_minutes(), 30);
}

#[tokio::test]
async fn list_team_members_uses_team_scoped_path() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/teams/t-1/members"))
        .and(header("authorization", "Bearer seed-access-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "value": [
                {
                    "id": "mem-1",
                    "displayName": "Ali",
                    "email": "ali@example.com",
                    "roles": ["owner"]
                },
                {
                    "id": "mem-2",
                    "displayName": "Bo",
                    "email": "",
                    "roles": []
                }
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let members = connector(&server).list_team_members("t-1").await.unwrap();
    assert_eq!(members.len(), 2);
    assert_eq!(members[0].email.as_deref(), Some("ali@example.com"));
    assert_eq!(members[0].roles, vec!["owner".to_string()]);
    // Empty email normalised to None; empty roles preserved as empty Vec.
    assert_eq!(members[1].email, None);
    assert!(members[1].roles.is_empty());
}

#[tokio::test]
async fn unauthorized_returns_typed_error_with_refresh_hint() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/me/joinedTeams"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "error": { "code": "InvalidAuthenticationToken", "message": "Lifetime validation failed." }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let err = connector(&server).list_teams().await.unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("401"), "should mention 401: {msg}");
    assert!(
        msg.contains("refresh access token"),
        "should hint at refresh: {msg}"
    );
}
