//! Wiremock-backed unit tests for `ZoomConnector`.
//!
//! Six scenarios per the brief:
//!  1. `create_meeting` posts `type=2` to `/users/{host}/meetings`.
//!  2. `list_recordings` GETs with `from`/`to` query params; ≥31d errors.
//!  3. `delete_meeting` issues `DELETE` with the reminder flag.
//!  4. `list_users` forwards a `status` filter when supplied.
//!  5. Cached token is reused — no OAuth refresh while it is still valid.
//!  6. A 401 triggers exactly one auth refresh, then the retried call wins.

use super::*;
use crate::auth::ZoomS2sAuth;
use crate::types::{ZoomMeetingDraft, ZoomMeetingKind, ZoomUserStatus};
use chrono::{DateTime, NaiveDate, Utc};
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn connector(server: &MockServer) -> ZoomConnector {
    let auth = ZoomS2sAuth::new("acct-id".into(), "cid".into(), "secret".into())
        .with_token_url(format!("{}/oauth/token", server.uri()));
    auth.set_access_token("seed-access-token", 3600);
    ZoomConnector::with_auth(auth)
        .expect("connector")
        .with_base_url(server.uri())
}

fn sample_start() -> DateTime<Utc> {
    DateTime::<Utc>::from_timestamp(1_900_000_000, 0).expect("ts")
}

#[tokio::test]
async fn create_meeting_posts_type_two_to_users_path() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/users/host-user-id/meetings"))
        .and(header("authorization", "Bearer seed-access-token"))
        .and(header("content-type", "application/json"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "id": 81234567890_i64,
            "uuid": "abc==",
            "host_email": "host@example.com",
            "topic": "Quarterly review",
            "start_time": "2030-04-30T17:00:00Z",
            "duration": 30,
            "join_url": "https://zoom.us/j/81234567890?pwd=xyz",
            "password": "xyz"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let zoom = connector(&server);
    let draft = ZoomMeetingDraft {
        topic: "Quarterly review".into(),
        start_time: sample_start(),
        duration_minutes: 30,
        timezone: "America/Los_Angeles".into(),
        agenda: Some("Roll up the deck".into()),
        waiting_room: true,
        join_before_host: false,
    };
    let meeting = zoom.create_meeting("host-user-id", draft).await.unwrap();
    assert_eq!(meeting.id, 81234567890_i64);
    assert_eq!(meeting.uuid, "abc==");
    assert_eq!(meeting.host_email, "host@example.com");
    assert_eq!(meeting.duration_minutes, 30);
    assert_eq!(meeting.password.as_deref(), Some("xyz"));
}

#[tokio::test]
async fn list_recordings_forwards_window_and_rejects_oversize() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/users/me/recordings"))
        .and(query_param("from", "2026-05-01"))
        .and(query_param("to", "2026-05-15"))
        .and(header("authorization", "Bearer seed-access-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "from": "2026-05-01",
            "to": "2026-05-15",
            "meetings": [
                {
                    "id": 99_i64,
                    "uuid": "rec-uuid",
                    "recording_start": "2026-05-04T13:00:00Z",
                    "recording_files": [
                        {
                            "id": "f1",
                            "file_type": "MP4",
                            "download_url": "https://api.zoom.us/recordings/f1",
                            "file_size": 1048576_u64
                        }
                    ]
                }
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let zoom = connector(&server);
    let recordings = zoom
        .list_recordings(
            "me",
            NaiveDate::from_ymd_opt(2026, 5, 1).unwrap(),
            NaiveDate::from_ymd_opt(2026, 5, 15).unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(recordings.len(), 1);
    assert_eq!(recordings[0].meeting_id, 99);
    assert_eq!(recordings[0].recording_files.len(), 1);
    assert_eq!(recordings[0].recording_files[0].file_type, "MP4");
    assert_eq!(recordings[0].recording_files[0].file_size_bytes, 1048576);

    // 31-day window: rejected client-side, never hits the server.
    let err = zoom
        .list_recordings(
            "me",
            NaiveDate::from_ymd_opt(2026, 5, 1).unwrap(),
            NaiveDate::from_ymd_opt(2026, 6, 5).unwrap(),
        )
        .await
        .unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("30d cap"), "unexpected error: {msg}");
}

#[tokio::test]
async fn delete_meeting_issues_delete_with_reminder_flag() {
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/meetings/777"))
        .and(query_param("schedule_for_reminder", "true"))
        .and(header("authorization", "Bearer seed-access-token"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;

    let zoom = connector(&server);
    zoom.delete_meeting(777, true).await.unwrap();
}

#[tokio::test]
async fn list_users_forwards_status_filter() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/users"))
        .and(query_param("status", "pending"))
        .and(header("authorization", "Bearer seed-access-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "users": [
                {
                    "id": "u-1",
                    "email": "pending@example.com",
                    "first_name": "Pat",
                    "last_name": "Pending",
                    "status": "pending"
                }
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let zoom = connector(&server);
    let users = zoom
        .list_users(Some(ZoomUserStatus::Pending))
        .await
        .unwrap();
    assert_eq!(users.len(), 1);
    assert_eq!(users[0].email, "pending@example.com");
    assert_eq!(users[0].status, ZoomUserStatus::Pending);
}

#[tokio::test]
async fn cached_token_is_reused_without_refresh() {
    let server = MockServer::start().await;
    // No mock on /oauth/token — if the connector ever calls it the test
    // fails (wiremock returns a 404 for unmounted paths and the response
    // would not parse).
    Mock::given(method("GET"))
        .and(path("/users/me/meetings"))
        .and(query_param("type", "upcoming"))
        .and(header("authorization", "Bearer seed-access-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meetings": []
        })))
        .expect(2)
        .mount(&server)
        .await;

    let zoom = connector(&server);
    let _first = zoom
        .list_meetings("me", ZoomMeetingKind::Upcoming)
        .await
        .unwrap();
    let _second = zoom
        .list_meetings("me", ZoomMeetingKind::Upcoming)
        .await
        .unwrap();
    // Cached token still resolves — proves no refresh happened.
    assert_eq!(zoom.auth().cached_token().as_deref(), Some("seed-access-token"));
}

#[tokio::test]
async fn unauthorized_triggers_one_refresh_then_succeeds() {
    let server = MockServer::start().await;
    // First call with the seed bearer: 401.
    Mock::given(method("GET"))
        .and(path("/meetings/42"))
        .and(header("authorization", "Bearer seed-access-token"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "code": 124,
            "message": "Access token is expired."
        })))
        .expect(1)
        .mount(&server)
        .await;
    // OAuth refresh issues a new bearer.
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .and(query_param("grant_type", "account_credentials"))
        .and(query_param("account_id", "acct-id"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "refreshed-token",
            "token_type": "bearer",
            "expires_in": 3600,
            "scope": "meeting:read:admin"
        })))
        .expect(1)
        .mount(&server)
        .await;
    // Retry with the new bearer succeeds.
    Mock::given(method("GET"))
        .and(path("/meetings/42"))
        .and(header("authorization", "Bearer refreshed-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": 42,
            "uuid": "post-refresh",
            "host_email": "host@example.com",
            "topic": "After refresh",
            "start_time": "2030-04-30T18:00:00Z",
            "duration": 15,
            "join_url": "https://zoom.us/j/42"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let zoom = connector(&server);
    let meeting = zoom.get_meeting(42).await.unwrap();
    assert_eq!(meeting.uuid, "post-refresh");
    assert_eq!(zoom.auth().cached_token().as_deref(), Some("refreshed-token"));
}
