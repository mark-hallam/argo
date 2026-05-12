//! Microsoft Graph REST client for Teams.
//!
//! Holds a [`reqwest::Client`] and a bearer OAuth token. The caller owns
//! token acquisition + refresh (an Azure AD app flow lives outside this
//! crate); the connector only asserts the token in `Authorization:
//! Bearer ...`. A 401 surfaces as `ArgoError::Provider("teams … 401 …
//! refresh access token …")` — the caller is expected to refresh and
//! rebuild the connector, rather than have the connector do silent
//! refresh that would require it to know the Azure AD app secrets.
//!
//! Channel creation is deliberately NOT exposed — the safety posture is
//! "workers post to existing channels", never "workers spawn channels".

use crate::types::{
    format_graph_time, Channel, ChannelsListWire, Message, MessageContentType, MessageWire,
    OnlineMeeting, OnlineMeetingAttendee, OnlineMeetingDraft, OnlineMeetingParticipants,
    OnlineMeetingWire, OutboundBody, OutboundMessage, SearchQueryEnvelope, SearchQueryResponseWire,
    SearchQueryString, SearchRequest, Team, TeamMember, TeamMembersListWire, TeamsListWire,
};
use argo_types::{ArgoError, ArgoResult};
use chrono::{DateTime, Duration, Utc};

/// Default base URL for the Microsoft Graph v1.0 surface.
pub const GRAPH_API_BASE_URL: &str = "https://graph.microsoft.com/v1.0";

/// Teams MCP connector backed by Microsoft Graph.
///
/// Construct with an Azure AD bearer access token; the caller is
/// responsible for acquiring + refreshing the token through whichever
/// Azure AD flow fits their deployment (client credentials, on-behalf-of,
/// device code, etc.). The connector preserves the brief's signature
/// (`new(oauth_token: String) -> Self`); use [`Self::with_base_url`] in
/// tests to point it at a wiremock server.
pub struct TeamsConnector {
    client: reqwest::Client,
    oauth_token: String,
    base_url: String,
}

impl TeamsConnector {
    /// Build a connector from a Graph access token.
    ///
    /// Panics only if reqwest cannot construct a default client, which is
    /// not expected at runtime — the brief asks for an infallible
    /// constructor (`-> Self`), so a failure to build the inner client is
    /// surfaced via [`Self::try_new`] for callers that need it.
    #[must_use]
    pub fn new(oauth_token: String) -> Self {
        Self::try_new(oauth_token).unwrap_or_else(|_| Self {
            client: reqwest::Client::new(),
            oauth_token: String::new(),
            base_url: GRAPH_API_BASE_URL.to_string(),
        })
    }

    /// Fallible constructor — fails only if reqwest cannot construct a
    /// client (e.g. TLS init failure on a misconfigured host).
    pub fn try_new(oauth_token: String) -> ArgoResult<Self> {
        let client = reqwest::Client::builder()
            .user_agent(concat!(
                "argo-mcp-connector-microsoft-teams/",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .map_err(|e| ArgoError::Provider(format!("teams http client: {e}")))?;
        Ok(Self {
            client,
            oauth_token,
            base_url: GRAPH_API_BASE_URL.to_string(),
        })
    }

    /// Override the Graph base URL (tests).
    #[must_use]
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into().trim_end_matches('/').to_string();
        self
    }

    /// GET `/me/joinedTeams`.
    pub async fn list_teams(&self) -> ArgoResult<Vec<Team>> {
        let wire: TeamsListWire = self.get_json("/me/joinedTeams", &[]).await?;
        Ok(wire.value.into_iter().map(Into::into).collect())
    }

    /// GET `/teams/{team_id}/channels`.
    pub async fn list_channels(&self, team_id: &str) -> ArgoResult<Vec<Channel>> {
        let path = format!("/teams/{}/channels", encode_path(team_id));
        let wire: ChannelsListWire = self.get_json(&path, &[]).await?;
        Ok(wire.value.into_iter().map(Into::into).collect())
    }

    /// POST `/teams/{team_id}/channels/{channel_id}/messages`.
    ///
    /// Sends the body as plain text (`contentType=text`). Use
    /// [`Self::post_message_html`] to send HTML.
    pub async fn post_message(
        &self,
        team_id: &str,
        channel_id: &str,
        body: &str,
    ) -> ArgoResult<Message> {
        self.post_message_with(team_id, channel_id, body, MessageContentType::Text)
            .await
    }

    /// HTML variant of [`Self::post_message`].
    pub async fn post_message_html(
        &self,
        team_id: &str,
        channel_id: &str,
        html: &str,
    ) -> ArgoResult<Message> {
        self.post_message_with(team_id, channel_id, html, MessageContentType::Html)
            .await
    }

    async fn post_message_with(
        &self,
        team_id: &str,
        channel_id: &str,
        body: &str,
        content_type: MessageContentType,
    ) -> ArgoResult<Message> {
        let path = format!(
            "/teams/{}/channels/{}/messages",
            encode_path(team_id),
            encode_path(channel_id),
        );
        let payload = OutboundMessage {
            body: OutboundBody {
                content_type: content_type.as_wire(),
                content: body,
            },
        };
        let wire: MessageWire = self.post_json(&path, &payload).await?;
        Ok(wire.into())
    }

    /// POST `/search/query` filtered to `chatMessage` entities.
    ///
    /// Graph's `/search/query` is unified across Teams chat, channel
    /// messages, mail, files, etc. Filtering to `chatMessage` returns
    /// Teams + Outlook DMs; channel messages have a separate
    /// `chatMessageReply`-style entity. The caller's required scopes for
    /// this endpoint are documented in the crate-root rustdoc.
    pub async fn search_messages(&self, query: &str) -> ArgoResult<Vec<Message>> {
        let payload = SearchQueryEnvelope {
            requests: vec![SearchRequest {
                entity_types: vec!["chatMessage"],
                query: SearchQueryString {
                    query_string: query,
                },
            }],
        };
        let wire: SearchQueryResponseWire = self.post_json("/search/query", &payload).await?;
        Ok(wire.into_messages())
    }

    /// POST `/me/onlineMeetings` — creates a **Teams Online Meeting**, not
    /// a calendar event.
    ///
    /// The returned [`OnlineMeeting`] carries a `join_web_url` that any
    /// attendee can click. Putting the meeting on the host's calendar is
    /// the Calendar API's job; this connector deliberately stays out of
    /// `/me/events`.
    pub async fn schedule_meeting(
        &self,
        subject: &str,
        attendees: Vec<String>,
        start: DateTime<Utc>,
        duration_min: u32,
    ) -> ArgoResult<OnlineMeeting> {
        let end = start + Duration::minutes(i64::from(duration_min));
        let attendees_wire: Vec<OnlineMeetingAttendee<'_>> = attendees
            .iter()
            .map(|upn| OnlineMeetingAttendee {
                upn,
                role: "attendee",
            })
            .collect();
        let payload = OnlineMeetingDraft {
            start_date_time: format_graph_time(start),
            end_date_time: format_graph_time(end),
            subject,
            participants: OnlineMeetingParticipants {
                attendees: attendees_wire,
            },
        };
        let wire: OnlineMeetingWire = self.post_json("/me/onlineMeetings", &payload).await?;
        Ok(wire.into())
    }

    /// GET `/teams/{team_id}/members`.
    pub async fn list_team_members(&self, team_id: &str) -> ArgoResult<Vec<TeamMember>> {
        let path = format!("/teams/{}/members", encode_path(team_id));
        let wire: TeamMembersListWire = self.get_json(&path, &[]).await?;
        Ok(wire.value.into_iter().map(Into::into).collect())
    }

    async fn get_json<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, &str)],
    ) -> ArgoResult<T> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&self.oauth_token)
            .header(reqwest::header::ACCEPT, "application/json")
            .query(query)
            .send()
            .await
            .map_err(|e| ArgoError::Provider(format!("teams GET {path}: {e}")))?;
        decode_response(resp, "GET", path).await
    }

    async fn post_json<B: serde::Serialize, T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> ArgoResult<T> {
        let url = format!("{}{}", self.base_url, path);
        let payload = serde_json::to_vec(body)
            .map_err(|e| ArgoError::Provider(format!("teams encode {path}: {e}")))?;
        let resp = self
            .client
            .post(&url)
            .bearer_auth(&self.oauth_token)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .header(reqwest::header::ACCEPT, "application/json")
            .body(payload)
            .send()
            .await
            .map_err(|e| ArgoError::Provider(format!("teams POST {path}: {e}")))?;
        decode_response(resp, "POST", path).await
    }
}

async fn decode_response<T: serde::de::DeserializeOwned>(
    resp: reqwest::Response,
    method: &str,
    path: &str,
) -> ArgoResult<T> {
    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| ArgoError::Provider(format!("teams {method} {path}: read body: {e}")))?;
    if status == reqwest::StatusCode::UNAUTHORIZED {
        // Typed 401 with a refresh hint — the caller owns refresh.
        return Err(ArgoError::Provider(format!(
            "teams {method} {path}: HTTP 401 unauthorized — refresh access token and retry: {text}"
        )));
    }
    if !status.is_success() {
        return Err(ArgoError::Provider(format!(
            "teams {method} {path}: HTTP {status}: {text}"
        )));
    }
    serde_json::from_str(&text)
        .map_err(|e| ArgoError::Provider(format!("teams {method} {path}: decode {e}: {text}")))
}

fn encode_path(value: &str) -> String {
    let mut out = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b':' | b'@' => {
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
