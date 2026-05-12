//! Microsoft Graph DTOs for the Teams connector.
//!
//! Graph identifies teams and channels by string ids (Azure AD object ids
//! for teams; channel-id strings of the form `19:xxxx@thread.tacv2` for
//! channels). All wire timestamps are ISO-8601 in UTC. The connector
//! exposes only the subset of fields the swarm needs; everything else is
//! ignored at decode time via `#[serde(default)]` so Graph can add new
//! fields without breaking us.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A Microsoft Team (a top-level group in Teams).
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(missing_docs)]
pub struct Team {
    pub id: String,
    pub display_name: String,
    pub description: Option<String>,
}

/// A channel inside a team.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(missing_docs)]
pub struct Channel {
    pub id: String,
    pub display_name: String,
    pub description: Option<String>,
    /// Channel membership type — `standard`, `private`, or `shared`.
    pub membership_type: String,
}

/// A channel/chat message returned by Graph.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(missing_docs)]
pub struct Message {
    pub id: String,
    /// `text` for plain text, `html` for rendered HTML (Graph's
    /// `body.contentType`). The connector preserves what Graph returns.
    pub content_type: MessageContentType,
    pub body: String,
    pub created: Option<DateTime<Utc>>,
    /// Sender display name when Graph reports it; absent for system
    /// messages.
    pub from_display_name: Option<String>,
    /// Web/link URL to the message in the Teams client.
    pub web_url: Option<String>,
}

/// Plain text vs HTML body. Graph accepts both on send; messages
/// returned from search may be either.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageContentType {
    /// `text` — Graph treats the body as a plain string.
    Text,
    /// `html` — Graph renders inline HTML (bold, links, mentions).
    Html,
}

impl MessageContentType {
    /// Wire string Graph expects (`text` or `html`).
    #[must_use]
    pub fn as_wire(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Html => "html",
        }
    }

    pub(crate) fn from_wire(raw: &str) -> Self {
        match raw {
            "html" | "Html" | "HTML" => Self::Html,
            _ => Self::Text,
        }
    }
}

/// A member of a team (Graph's `aadUserConversationMember`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(missing_docs)]
pub struct TeamMember {
    pub id: String,
    pub display_name: String,
    pub email: Option<String>,
    /// Member roles. Empty for ordinary members; `["owner"]` for owners.
    pub roles: Vec<String>,
}

/// A Teams Online Meeting (NOT a calendar event — calendar events live
/// behind `/me/events` on the Graph Calendar API). The meeting object
/// carries a join URL and is shareable directly.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(missing_docs)]
pub struct OnlineMeeting {
    pub id: String,
    pub subject: String,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub join_web_url: String,
}

// ---------- Wire types (inbound) ----------

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub(crate) struct TeamWire {
    pub id: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    pub description: Option<String>,
}

impl From<TeamWire> for Team {
    fn from(w: TeamWire) -> Self {
        Self {
            id: w.id,
            display_name: w.display_name,
            description: w.description.filter(|s| !s.is_empty()),
        }
    }
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub(crate) struct TeamsListWire {
    pub value: Vec<TeamWire>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub(crate) struct ChannelWire {
    pub id: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    pub description: Option<String>,
    #[serde(rename = "membershipType")]
    pub membership_type: Option<String>,
}

impl From<ChannelWire> for Channel {
    fn from(w: ChannelWire) -> Self {
        Self {
            id: w.id,
            display_name: w.display_name,
            description: w.description.filter(|s| !s.is_empty()),
            membership_type: w.membership_type.unwrap_or_else(|| "standard".into()),
        }
    }
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub(crate) struct ChannelsListWire {
    pub value: Vec<ChannelWire>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub(crate) struct MessageBodyWire {
    #[serde(rename = "contentType")]
    pub content_type: Option<String>,
    pub content: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub(crate) struct MessageFromUserWire {
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub(crate) struct MessageFromWire {
    pub user: Option<MessageFromUserWire>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub(crate) struct MessageWire {
    pub id: String,
    pub body: Option<MessageBodyWire>,
    #[serde(rename = "createdDateTime")]
    pub created_date_time: Option<DateTime<Utc>>,
    pub from: Option<MessageFromWire>,
    #[serde(rename = "webUrl")]
    pub web_url: Option<String>,
}

impl From<MessageWire> for Message {
    fn from(w: MessageWire) -> Self {
        let (content_type, body) = match w.body {
            Some(b) => (
                MessageContentType::from_wire(b.content_type.as_deref().unwrap_or("text")),
                b.content.unwrap_or_default(),
            ),
            None => (MessageContentType::Text, String::new()),
        };
        let from_display_name = w
            .from
            .and_then(|f| f.user)
            .and_then(|u| u.display_name)
            .filter(|s| !s.is_empty());
        Self {
            id: w.id,
            content_type,
            body,
            created: w.created_date_time,
            from_display_name,
            web_url: w.web_url.filter(|s| !s.is_empty()),
        }
    }
}

/// Single search hit envelope returned by `/search/query`.
#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub(crate) struct SearchHitResourceWire {
    pub id: Option<String>,
    pub body: Option<MessageBodyWire>,
    #[serde(rename = "createdDateTime")]
    pub created_date_time: Option<DateTime<Utc>>,
    pub from: Option<MessageFromWire>,
    #[serde(rename = "webUrl")]
    pub web_url: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub(crate) struct SearchHitWire {
    pub resource: Option<SearchHitResourceWire>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub(crate) struct SearchHitsContainerWire {
    pub hits: Vec<SearchHitWire>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub(crate) struct SearchHitsContainersOuterWire {
    #[serde(rename = "hitsContainers")]
    pub hits_containers: Vec<SearchHitsContainerWire>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub(crate) struct SearchQueryResponseWire {
    pub value: Vec<SearchHitsContainersOuterWire>,
}

impl SearchQueryResponseWire {
    pub(crate) fn into_messages(self) -> Vec<Message> {
        let mut out = Vec::new();
        for outer in self.value {
            for container in outer.hits_containers {
                for hit in container.hits {
                    if let Some(resource) = hit.resource {
                        if let Some(id) = resource.id {
                            let (content_type, body) = match resource.body {
                                Some(b) => (
                                    MessageContentType::from_wire(
                                        b.content_type.as_deref().unwrap_or("text"),
                                    ),
                                    b.content.unwrap_or_default(),
                                ),
                                None => (MessageContentType::Text, String::new()),
                            };
                            let from_display_name = resource
                                .from
                                .and_then(|f| f.user)
                                .and_then(|u| u.display_name)
                                .filter(|s| !s.is_empty());
                            out.push(Message {
                                id,
                                content_type,
                                body,
                                created: resource.created_date_time,
                                from_display_name,
                                web_url: resource.web_url.filter(|s| !s.is_empty()),
                            });
                        }
                    }
                }
            }
        }
        out
    }
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub(crate) struct TeamMemberWire {
    pub id: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    pub email: Option<String>,
    pub roles: Vec<String>,
}

impl From<TeamMemberWire> for TeamMember {
    fn from(w: TeamMemberWire) -> Self {
        Self {
            id: w.id,
            display_name: w.display_name,
            email: w.email.filter(|s| !s.is_empty()),
            roles: w.roles,
        }
    }
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub(crate) struct TeamMembersListWire {
    pub value: Vec<TeamMemberWire>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub(crate) struct OnlineMeetingTimeWire {
    #[serde(rename = "dateTime")]
    pub date_time: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub(crate) struct OnlineMeetingWire {
    pub id: String,
    pub subject: Option<String>,
    #[serde(rename = "startDateTime")]
    pub start_date_time: Option<DateTime<Utc>>,
    #[serde(rename = "endDateTime")]
    pub end_date_time: Option<DateTime<Utc>>,
    /// Some Graph payloads return `start`/`end` envelopes instead of the
    /// flat `startDateTime`/`endDateTime` fields.
    pub start: Option<OnlineMeetingTimeWire>,
    pub end: Option<OnlineMeetingTimeWire>,
    #[serde(rename = "joinWebUrl")]
    pub join_web_url: Option<String>,
    #[serde(rename = "joinUrl")]
    pub join_url: Option<String>,
}

impl From<OnlineMeetingWire> for OnlineMeeting {
    fn from(w: OnlineMeetingWire) -> Self {
        let start = w
            .start_date_time
            .or_else(|| w.start.and_then(|s| s.date_time))
            .unwrap_or_else(default_epoch);
        let end = w
            .end_date_time
            .or_else(|| w.end.and_then(|s| s.date_time))
            .unwrap_or(start);
        let join = w.join_web_url.or(w.join_url).unwrap_or_default();
        Self {
            id: w.id,
            subject: w.subject.unwrap_or_default(),
            start,
            end,
            join_web_url: join,
        }
    }
}

// ---------- Outbound payloads ----------

#[derive(Debug, Serialize)]
pub(crate) struct OutboundBody<'a> {
    #[serde(rename = "contentType")]
    pub content_type: &'a str,
    pub content: &'a str,
}

#[derive(Debug, Serialize)]
pub(crate) struct OutboundMessage<'a> {
    pub body: OutboundBody<'a>,
}

#[derive(Debug, Serialize)]
pub(crate) struct OnlineMeetingDraft<'a> {
    #[serde(rename = "startDateTime")]
    pub start_date_time: String,
    #[serde(rename = "endDateTime")]
    pub end_date_time: String,
    pub subject: &'a str,
    pub participants: OnlineMeetingParticipants<'a>,
}

#[derive(Debug, Serialize)]
pub(crate) struct OnlineMeetingParticipants<'a> {
    pub attendees: Vec<OnlineMeetingAttendee<'a>>,
}

#[derive(Debug, Serialize)]
pub(crate) struct OnlineMeetingAttendee<'a> {
    #[serde(rename = "upn")]
    pub upn: &'a str,
    pub role: &'a str,
}

#[derive(Debug, Serialize)]
pub(crate) struct SearchQueryEnvelope<'a> {
    pub requests: Vec<SearchRequest<'a>>,
}

#[derive(Debug, Serialize)]
pub(crate) struct SearchRequest<'a> {
    #[serde(rename = "entityTypes")]
    pub entity_types: Vec<&'a str>,
    pub query: SearchQueryString<'a>,
}

#[derive(Debug, Serialize)]
pub(crate) struct SearchQueryString<'a> {
    #[serde(rename = "queryString")]
    pub query_string: &'a str,
}

/// Format a UTC instant as ISO-8601 with a `Z` suffix.
#[must_use]
pub fn format_graph_time(ts: DateTime<Utc>) -> String {
    ts.format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

fn default_epoch() -> DateTime<Utc> {
    DateTime::<Utc>::from_timestamp(0, 0).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_type_round_trips() {
        assert_eq!(MessageContentType::Text.as_wire(), "text");
        assert_eq!(MessageContentType::Html.as_wire(), "html");
        assert_eq!(MessageContentType::from_wire("text"), MessageContentType::Text);
        assert_eq!(MessageContentType::from_wire("html"), MessageContentType::Html);
        assert_eq!(MessageContentType::from_wire("Html"), MessageContentType::Html);
        // Unknown wire content types default to Text — forward-compat.
        assert_eq!(MessageContentType::from_wire("rtf"), MessageContentType::Text);
    }

    #[test]
    fn format_graph_time_emits_z_suffix() {
        let ts = DateTime::<Utc>::from_timestamp(1_700_000_000, 0).expect("ts");
        let s = format_graph_time(ts);
        assert!(s.ends_with('Z'));
        assert!(!s.contains('+'));
    }

    #[test]
    fn channel_membership_defaults_to_standard() {
        let wire = ChannelWire {
            id: "19:abc@thread.tacv2".into(),
            display_name: "General".into(),
            description: None,
            membership_type: None,
        };
        let channel: Channel = wire.into();
        assert_eq!(channel.membership_type, "standard");
    }

    #[test]
    fn online_meeting_falls_back_to_join_url() {
        let wire = OnlineMeetingWire {
            id: "om-1".into(),
            subject: Some("Sync".into()),
            start_date_time: None,
            end_date_time: None,
            start: None,
            end: None,
            join_web_url: None,
            join_url: Some("https://teams.microsoft.com/l/meetup-join/abc".into()),
        };
        let m: OnlineMeeting = wire.into();
        assert_eq!(m.join_web_url, "https://teams.microsoft.com/l/meetup-join/abc");
    }
}
