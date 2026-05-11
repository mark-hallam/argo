//! Zoom v2 API DTOs.
//!
//! Zoom meetings are addressed by an `i64` numeric id; users by a string
//! id (Zoom UUID or the magic `"me"` token). All wire timestamps use
//! ISO-8601 with a trailing `Z` (UTC); the meeting `timezone` field is a
//! separate IANA tz string used purely for the user-visible time
//! display — it does NOT shift the underlying instant.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

/// Filter for `list_meetings`. Maps to Zoom's `type` query parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoomMeetingKind {
    /// Scheduled meetings that have not yet started.
    Scheduled,
    /// Currently live meetings.
    Live,
    /// Scheduled + recurring future occurrences.
    Upcoming,
    /// Past meetings.
    Previous,
}

impl ZoomMeetingKind {
    /// The wire string Zoom expects for the `type` query parameter.
    #[must_use]
    pub fn as_query(self) -> &'static str {
        match self {
            Self::Scheduled => "scheduled",
            Self::Live => "live",
            Self::Upcoming => "upcoming",
            Self::Previous => "previous_meetings",
        }
    }
}

/// User-status filter for `list_users`. Maps to Zoom's `status` query
/// parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoomUserStatus {
    /// Active users (Zoom's default).
    Active,
    /// Deactivated users.
    Inactive,
    /// Invited users who have not onboarded.
    Pending,
}

impl ZoomUserStatus {
    /// The wire string Zoom expects for the `status` query parameter.
    #[must_use]
    pub fn as_query(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Inactive => "inactive",
            Self::Pending => "pending",
        }
    }

    pub(crate) fn from_wire(raw: &str) -> Self {
        match raw {
            "inactive" => Self::Inactive,
            "pending" => Self::Pending,
            _ => Self::Active,
        }
    }
}

/// Draft payload for `create_meeting`. `start_time` is sent as ISO-8601
/// with `Z`; `timezone` is a separate IANA tz string that Zoom uses for
/// the user-visible time display only.
#[derive(Debug, Clone)]
#[allow(missing_docs)]
pub struct ZoomMeetingDraft {
    pub topic: String,
    pub start_time: DateTime<Utc>,
    pub duration_minutes: u32,
    pub timezone: String,
    pub agenda: Option<String>,
    pub waiting_room: bool,
    pub join_before_host: bool,
}

/// Persisted meeting returned by Zoom.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(missing_docs)]
pub struct ZoomMeeting {
    pub id: i64,
    pub uuid: String,
    pub host_email: String,
    pub topic: String,
    pub start_time: DateTime<Utc>,
    pub duration_minutes: u32,
    pub join_url: String,
    pub password: Option<String>,
}

/// One file in a `ZoomRecording` (Zoom returns one entry per output
/// format: MP4 video, M4A audio, VTT captions, ...).
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(missing_docs)]
pub struct ZoomRecordingFile {
    pub id: String,
    pub file_type: String,
    pub download_url: String,
    pub file_size_bytes: u64,
}

/// Persisted recording group for a single meeting instance.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(missing_docs)]
pub struct ZoomRecording {
    pub meeting_id: i64,
    pub uuid: String,
    pub recording_start: DateTime<Utc>,
    pub recording_files: Vec<ZoomRecordingFile>,
}

/// Zoom account user.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(missing_docs)]
pub struct ZoomUser {
    pub id: String,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub status: ZoomUserStatus,
    pub last_login_time: Option<DateTime<Utc>>,
}

// ---------- Wire types (inbound) ----------

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub(crate) struct MeetingWire {
    pub id: i64,
    pub uuid: String,
    pub host_email: String,
    pub topic: String,
    pub start_time: Option<DateTime<Utc>>,
    pub duration: u32,
    pub join_url: String,
    pub password: Option<String>,
}

impl From<MeetingWire> for ZoomMeeting {
    fn from(w: MeetingWire) -> Self {
        Self {
            id: w.id,
            uuid: w.uuid,
            host_email: w.host_email,
            topic: w.topic,
            start_time: w.start_time.unwrap_or_else(default_epoch),
            duration_minutes: w.duration,
            join_url: w.join_url,
            password: w.password.filter(|p| !p.is_empty()),
        }
    }
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub(crate) struct MeetingsListWire {
    pub meetings: Vec<MeetingWire>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub(crate) struct RecordingFileWire {
    pub id: String,
    pub file_type: String,
    pub download_url: String,
    pub file_size: u64,
}

impl From<RecordingFileWire> for ZoomRecordingFile {
    fn from(w: RecordingFileWire) -> Self {
        Self {
            id: w.id,
            file_type: w.file_type,
            download_url: w.download_url,
            file_size_bytes: w.file_size,
        }
    }
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub(crate) struct RecordingWire {
    pub id: i64,
    pub uuid: String,
    pub recording_start: Option<DateTime<Utc>>,
    pub start_time: Option<DateTime<Utc>>,
    pub recording_files: Vec<RecordingFileWire>,
}

impl From<RecordingWire> for ZoomRecording {
    fn from(w: RecordingWire) -> Self {
        Self {
            meeting_id: w.id,
            uuid: w.uuid,
            recording_start: w
                .recording_start
                .or(w.start_time)
                .unwrap_or_else(default_epoch),
            recording_files: w.recording_files.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub(crate) struct RecordingsListWire {
    pub meetings: Vec<RecordingWire>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub(crate) struct UserWire {
    pub id: String,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub status: String,
    pub last_login_time: Option<DateTime<Utc>>,
}

impl From<UserWire> for ZoomUser {
    fn from(w: UserWire) -> Self {
        Self {
            id: w.id,
            email: w.email,
            first_name: w.first_name,
            last_name: w.last_name,
            status: ZoomUserStatus::from_wire(&w.status),
            last_login_time: w.last_login_time,
        }
    }
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub(crate) struct UsersListWire {
    pub users: Vec<UserWire>,
}

// ---------- Outbound payloads ----------

/// `type=2` ⇒ scheduled meeting in Zoom's type taxonomy.
#[derive(Debug, Serialize)]
pub(crate) struct MeetingCreate<'a> {
    pub topic: &'a str,
    #[serde(rename = "type")]
    pub kind: u8,
    pub start_time: String,
    pub duration: u32,
    pub timezone: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agenda: Option<&'a str>,
    pub settings: MeetingCreateSettings,
}

#[derive(Debug, Serialize)]
pub(crate) struct MeetingCreateSettings {
    pub waiting_room: bool,
    pub join_before_host: bool,
}

/// Format a UTC instant as ISO-8601 with `Z`.
#[must_use]
pub fn format_zoom_time(ts: DateTime<Utc>) -> String {
    ts.format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// Format a date as `YYYY-MM-DD`.
#[must_use]
pub fn format_zoom_date(date: NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

fn default_epoch() -> DateTime<Utc> {
    DateTime::<Utc>::from_timestamp(0, 0).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meeting_kind_maps_to_zoom_query() {
        assert_eq!(ZoomMeetingKind::Scheduled.as_query(), "scheduled");
        assert_eq!(ZoomMeetingKind::Live.as_query(), "live");
        assert_eq!(ZoomMeetingKind::Upcoming.as_query(), "upcoming");
        assert_eq!(ZoomMeetingKind::Previous.as_query(), "previous_meetings");
    }

    #[test]
    fn user_status_round_trips() {
        assert_eq!(ZoomUserStatus::from_wire("active"), ZoomUserStatus::Active);
        assert_eq!(ZoomUserStatus::from_wire("inactive"), ZoomUserStatus::Inactive);
        assert_eq!(ZoomUserStatus::from_wire("pending"), ZoomUserStatus::Pending);
        // Unknown wire values fall back to Active — keeps the connector
        // forward-compatible with new Zoom user statuses.
        assert_eq!(ZoomUserStatus::from_wire("???"), ZoomUserStatus::Active);
    }

    #[test]
    fn format_zoom_time_emits_z_suffix() {
        let ts = DateTime::<Utc>::from_timestamp(1_700_000_000, 0).expect("ts");
        let s = format_zoom_time(ts);
        assert!(s.ends_with('Z'));
        assert!(!s.contains('+'));
    }
}
