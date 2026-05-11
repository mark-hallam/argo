//! Zoom v2 REST client.
//!
//! Holds a [`reqwest::Client`] and a [`ZoomS2sAuth`]; constructs typed
//! requests against `https://api.zoom.us/v2/...`, authenticates with the
//! cached Server-to-Server bearer token, and transparently refreshes the
//! token once on a 401 before propagating the error.

use crate::auth::ZoomS2sAuth;
use crate::types::{
    format_zoom_date, format_zoom_time, MeetingCreate, MeetingCreateSettings, MeetingWire,
    MeetingsListWire, RecordingsListWire, UsersListWire, ZoomMeeting, ZoomMeetingDraft,
    ZoomMeetingKind, ZoomRecording, ZoomUser, ZoomUserStatus,
};
use argo_types::{ArgoError, ArgoResult};
use chrono::NaiveDate;

/// Default base URL for the Zoom v2 REST API.
pub const ZOOM_API_BASE_URL: &str = "https://api.zoom.us/v2";

/// Maximum recording-list window Zoom accepts in a single call.
pub const MAX_RECORDING_WINDOW_DAYS: i64 = 30;

/// Zoom v2 REST connector.
///
/// Authenticates with Server-to-Server OAuth and exposes a typed subset
/// of the meetings / recordings / users surface.
pub struct ZoomConnector {
    client: reqwest::Client,
    auth: ZoomS2sAuth,
    base_url: String,
}

impl ZoomConnector {
    /// Build a connector from the three Server-to-Server credentials.
    pub fn new(account_id: String, client_id: String, client_secret: String) -> ArgoResult<Self> {
        let auth = ZoomS2sAuth::new(account_id, client_id, client_secret);
        Self::with_auth(auth)
    }

    /// Build from a pre-constructed [`ZoomS2sAuth`]. Useful in tests that
    /// need to override the OAuth token URL.
    pub fn with_auth(auth: ZoomS2sAuth) -> ArgoResult<Self> {
        let client = reqwest::Client::builder()
            .user_agent(concat!("argo-mcp-connector-zoom/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| ArgoError::Provider(format!("zoom http client: {e}")))?;
        Ok(Self {
            client,
            auth,
            base_url: ZOOM_API_BASE_URL.to_string(),
        })
    }

    /// Override the API base URL — used by tests to point at a wiremock
    /// server.
    #[must_use]
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into().trim_end_matches('/').to_string();
        self
    }

    /// Reference to the auth holder. Useful for tests asserting token state.
    #[must_use]
    pub fn auth(&self) -> &ZoomS2sAuth {
        &self.auth
    }

    /// POST `/users/{host_user_id}/meetings` with `type=2` (scheduled).
    pub async fn create_meeting(
        &self,
        host_user_id: &str,
        draft: ZoomMeetingDraft,
    ) -> ArgoResult<ZoomMeeting> {
        let path = format!("/users/{}/meetings", encode_path(host_user_id));
        let body = MeetingCreate {
            topic: &draft.topic,
            kind: 2,
            start_time: format_zoom_time(draft.start_time),
            duration: draft.duration_minutes,
            timezone: &draft.timezone,
            agenda: draft.agenda.as_deref(),
            settings: MeetingCreateSettings {
                waiting_room: draft.waiting_room,
                join_before_host: draft.join_before_host,
            },
        };
        let wire: MeetingWire = self.post_json(&path, &body).await?;
        Ok(wire.into())
    }

    /// GET `/meetings/{meeting_id}`.
    pub async fn get_meeting(&self, meeting_id: i64) -> ArgoResult<ZoomMeeting> {
        let path = format!("/meetings/{meeting_id}");
        let wire: MeetingWire = self.get_json(&path, &[]).await?;
        Ok(wire.into())
    }

    /// GET `/users/{host_user_id}/meetings?type=<kind>`.
    pub async fn list_meetings(
        &self,
        host_user_id: &str,
        kind: ZoomMeetingKind,
    ) -> ArgoResult<Vec<ZoomMeeting>> {
        let path = format!("/users/{}/meetings", encode_path(host_user_id));
        let wire: MeetingsListWire = self
            .get_json(&path, &[("type", kind.as_query())])
            .await?;
        Ok(wire.meetings.into_iter().map(Into::into).collect())
    }

    /// DELETE `/meetings/{meeting_id}?schedule_for_reminder=<bool>`.
    pub async fn delete_meeting(
        &self,
        meeting_id: i64,
        schedule_reminder: bool,
    ) -> ArgoResult<()> {
        let path = format!("/meetings/{meeting_id}");
        let flag = if schedule_reminder { "true" } else { "false" };
        self.delete(&path, &[("schedule_for_reminder", flag)]).await
    }

    /// GET `/users/{host_user_id}/recordings?from=<since>&to=<until>`.
    ///
    /// Zoom hard-caps the window at 30 days; wider windows return
    /// [`ArgoError::Provider`] without making the HTTP call.
    pub async fn list_recordings(
        &self,
        host_user_id: &str,
        since: NaiveDate,
        until: NaiveDate,
    ) -> ArgoResult<Vec<ZoomRecording>> {
        if until < since {
            return Err(ArgoError::Provider(format!(
                "zoom list_recordings: until ({until}) before since ({since})"
            )));
        }
        let span = (until - since).num_days();
        if span > MAX_RECORDING_WINDOW_DAYS {
            return Err(ArgoError::Provider(format!(
                "zoom list_recordings: window {span}d exceeds Zoom's {MAX_RECORDING_WINDOW_DAYS}d cap"
            )));
        }
        let path = format!("/users/{}/recordings", encode_path(host_user_id));
        let from = format_zoom_date(since);
        let to = format_zoom_date(until);
        let wire: RecordingsListWire = self
            .get_json(&path, &[("from", from.as_str()), ("to", to.as_str())])
            .await?;
        Ok(wire.meetings.into_iter().map(Into::into).collect())
    }

    /// GET `/users[?status=<status>]`.
    pub async fn list_users(&self, status: Option<ZoomUserStatus>) -> ArgoResult<Vec<ZoomUser>> {
        let query: Vec<(&str, &str)> = match status {
            Some(s) => vec![("status", s.as_query())],
            None => Vec::new(),
        };
        let wire: UsersListWire = self.get_json("/users", &query).await?;
        Ok(wire.users.into_iter().map(Into::into).collect())
    }

    async fn get_json<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, &str)],
    ) -> ArgoResult<T> {
        let url = format!("{}{}", self.base_url.trim_end_matches('/'), path);
        self.send_with_refresh(|token| {
            self.client
                .get(&url)
                .bearer_auth(token)
                .header(reqwest::header::ACCEPT, "application/json")
                .query(query)
        })
        .await
    }

    async fn post_json<B: serde::Serialize, T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> ArgoResult<T> {
        let url = format!("{}{}", self.base_url.trim_end_matches('/'), path);
        let payload = serde_json::to_vec(body)
            .map_err(|e| ArgoError::Provider(format!("zoom encode {path}: {e}")))?;
        self.send_with_refresh(|token| {
            self.client
                .post(&url)
                .bearer_auth(token)
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .header(reqwest::header::ACCEPT, "application/json")
                .body(payload.clone())
        })
        .await
    }

    async fn delete(&self, path: &str, query: &[(&str, &str)]) -> ArgoResult<()> {
        let url = format!("{}{}", self.base_url.trim_end_matches('/'), path);
        let token = self.bearer().await?;
        let resp = self
            .client
            .delete(&url)
            .bearer_auth(&token)
            .query(query)
            .send()
            .await
            .map_err(|e| ArgoError::Provider(format!("zoom DELETE {path}: {e}")))?;
        if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
            let refreshed = self
                .auth
                .refresh(&self.client)
                .await
                .map_err(ArgoError::Provider)?;
            let retry = self
                .client
                .delete(&url)
                .bearer_auth(&refreshed)
                .query(query)
                .send()
                .await
                .map_err(|e| ArgoError::Provider(format!("zoom DELETE retry {path}: {e}")))?;
            return drain_empty(retry, "DELETE", path).await;
        }
        drain_empty(resp, "DELETE", path).await
    }

    async fn bearer(&self) -> ArgoResult<String> {
        match self.auth.cached_token() {
            Some(t) => Ok(t),
            None => self
                .auth
                .refresh(&self.client)
                .await
                .map_err(ArgoError::Provider),
        }
    }

    async fn send_with_refresh<F, T>(&self, build_req: F) -> ArgoResult<T>
    where
        F: Fn(&str) -> reqwest::RequestBuilder,
        T: serde::de::DeserializeOwned,
    {
        let token = self.bearer().await?;
        let resp = build_req(&token)
            .send()
            .await
            .map_err(|e| ArgoError::Provider(format!("zoom request: {e}")))?;
        if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
            let new_token = self
                .auth
                .refresh(&self.client)
                .await
                .map_err(ArgoError::Provider)?;
            let retry = build_req(&new_token)
                .send()
                .await
                .map_err(|e| ArgoError::Provider(format!("zoom retry: {e}")))?;
            return decode_response(retry).await;
        }
        decode_response(resp).await
    }
}

async fn decode_response<T: serde::de::DeserializeOwned>(resp: reqwest::Response) -> ArgoResult<T> {
    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| ArgoError::Provider(format!("zoom read body: {e}")))?;
    if !status.is_success() {
        return Err(ArgoError::Provider(format!("zoom HTTP {status}: {text}")));
    }
    serde_json::from_str(&text)
        .map_err(|e| ArgoError::Provider(format!("zoom decode: {e}: {text}")))
}

async fn drain_empty(resp: reqwest::Response, method: &str, path: &str) -> ArgoResult<()> {
    let status = resp.status();
    if status.is_success() {
        // Drop the body — Zoom returns 204 No Content on delete.
        let _ = resp.bytes().await;
        return Ok(());
    }
    let text = resp
        .text()
        .await
        .map_err(|e| ArgoError::Provider(format!("zoom {method} {path}: read body: {e}")))?;
    Err(ArgoError::Provider(format!(
        "zoom {method} {path}: HTTP {status}: {text}"
    )))
}

fn encode_path(value: &str) -> String {
    let mut out = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(char::from(byte));
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod client_tests;
