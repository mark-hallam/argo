//! Live Zoom Server-to-Server smoke test.
//!
//! Gated by four env vars; skips silently when any is missing so the
//! crate's `cargo test -p argo-mcp-connector-zoom` stays green in CI
//! without leaking credentials:
//!
//!  - `ARGO_ZOOM_ACCOUNT_ID`
//!  - `ARGO_ZOOM_CLIENT_ID`
//!  - `ARGO_ZOOM_CLIENT_SECRET`
//!  - `ARGO_ZOOM_TEST_USER_ID` (Zoom user UUID or email of the host)
//!
//! The test creates a single meeting 5 minutes in the future, reads it
//! back via `get_meeting`, then deletes it. Failure to delete is treated
//! as a test failure because leaking meetings into the operator's
//! account is worse than a flaky pass.

use argo_mcp_connector_zoom::{ZoomConnector, ZoomMeetingDraft};
use chrono::{Duration, Utc};

fn env_or_skip(name: &str) -> Option<String> {
    match std::env::var(name) {
        Ok(v) if !v.trim().is_empty() => Some(v),
        _ => None,
    }
}

#[tokio::test]
async fn round_trip_create_get_delete() {
    let Some(account_id) = env_or_skip("ARGO_ZOOM_ACCOUNT_ID") else {
        eprintln!("skip: ARGO_ZOOM_ACCOUNT_ID unset");
        return;
    };
    let Some(client_id) = env_or_skip("ARGO_ZOOM_CLIENT_ID") else {
        eprintln!("skip: ARGO_ZOOM_CLIENT_ID unset");
        return;
    };
    let Some(client_secret) = env_or_skip("ARGO_ZOOM_CLIENT_SECRET") else {
        eprintln!("skip: ARGO_ZOOM_CLIENT_SECRET unset");
        return;
    };
    let Some(host_user_id) = env_or_skip("ARGO_ZOOM_TEST_USER_ID") else {
        eprintln!("skip: ARGO_ZOOM_TEST_USER_ID unset");
        return;
    };

    let zoom =
        ZoomConnector::new(account_id, client_id, client_secret).expect("build zoom connector");

    let draft = ZoomMeetingDraft {
        topic: "argo zoom smoke".into(),
        start_time: Utc::now() + Duration::minutes(5),
        duration_minutes: 5,
        timezone: "UTC".into(),
        agenda: Some("smoke test — safe to delete".into()),
        waiting_room: true,
        join_before_host: false,
    };

    let created = zoom
        .create_meeting(&host_user_id, draft)
        .await
        .expect("create meeting");
    assert!(created.id > 0, "id should be assigned");

    // Always attempt cleanup even if the read-back fails.
    let read = zoom.get_meeting(created.id).await;
    let delete = zoom.delete_meeting(created.id, false).await;

    let meeting = read.expect("get meeting");
    assert_eq!(meeting.id, created.id);
    delete.expect("delete meeting");
}
