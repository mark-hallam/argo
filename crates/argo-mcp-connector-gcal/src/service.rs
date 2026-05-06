//! MCP service implementation for Google Calendar.
use crate::client::{optional_i64, required_str, GcalClient, GcalConfig};
use argo_mcp::{
    CallToolParams, CallToolResult, McpContent, McpError, McpResult, McpServerService,
    McpToolSchema, ServerInfo,
};
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use serde_json::{json, Value};

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// MCP service exposing Google Calendar tools.
#[derive(Debug, Clone)]
pub struct GcalMcpService {
    client: GcalClient,
}

impl GcalMcpService {
    /// Build from an existing API client.
    #[must_use]
    pub fn new(client: GcalClient) -> Self {
        Self { client }
    }

    /// Build from `GCAL_ACCESS_TOKEN` or `GCAL_TEST_TOKEN`.
    pub async fn from_env() -> McpResult<Self> {
        Ok(Self::new(GcalClient::new(GcalConfig::from_env().await?)))
    }
}

#[async_trait]
impl McpServerService for GcalMcpService {
    fn server_info(&self) -> ServerInfo {
        ServerInfo {
            name: "gcal".into(),
            version: VERSION.into(),
        }
    }

    fn list_tools(&self) -> Vec<McpToolSchema> {
        vec![
            tool(
                "gcal.list_events",
                "List Google Calendar events in a time range.",
                json!({
                    "type": "object",
                    "properties": {
                        "calendar_id": {"type": "string"},
                        "time_min": {"type": "string"},
                        "time_max": {"type": "string"},
                        "max_results": {"type": "integer"}
                    }
                }),
            ),
            tool(
                "gcal.create_event",
                "Create a Google Calendar event.",
                event_schema(true),
            ),
            tool(
                "gcal.update_event",
                "Patch a Google Calendar event.",
                event_schema(false),
            ),
            tool(
                "gcal.suggest_time",
                "Find the first free slot in a time range.",
                json!({
                    "type": "object",
                    "required": ["time_min", "time_max", "duration_minutes"],
                    "properties": {
                        "calendar_id": {"type": "string"},
                        "time_min": {"type": "string"},
                        "time_max": {"type": "string"},
                        "duration_minutes": {"type": "integer"}
                    }
                }),
            ),
            tool(
                "gcal.respond_to_invite",
                "Respond to a Google Calendar invite.",
                json!({
                    "type": "object",
                    "required": ["event_id", "attendee_email", "response_status"],
                    "properties": {
                        "calendar_id": {"type": "string"},
                        "event_id": {"type": "string"},
                        "attendee_email": {"type": "string"},
                        "response_status": {
                            "type": "string",
                            "enum": ["accepted", "declined", "tentative", "needsAction"]
                        }
                    }
                }),
            ),
        ]
    }

    async fn call_tool(&self, params: CallToolParams) -> McpResult<CallToolResult> {
        let value = match params.name.as_str() {
            "gcal.list_events" => self.client.list_events(&params.arguments).await?,
            "gcal.create_event" => self.client.create_event(&params.arguments).await?,
            "gcal.update_event" => self.client.update_event(&params.arguments).await?,
            "gcal.suggest_time" => self.suggest_time(&params.arguments).await?,
            "gcal.respond_to_invite" => self.client.respond_to_invite(&params.arguments).await?,
            other => {
                return Err(McpError::Rpc {
                    code: -32602,
                    message: format!("unknown gcal tool: {other}"),
                    data: None,
                })
            }
        };
        Ok(json_result(value))
    }
}

impl GcalMcpService {
    async fn suggest_time(&self, args: &Value) -> McpResult<Value> {
        let freebusy = self.client.freebusy(args).await?;
        let start = parse_time(required_str(args, "time_min")?)?;
        let end = parse_time(required_str(args, "time_max")?)?;
        let duration = Duration::minutes(optional_i64(args, "duration_minutes").unwrap_or(30));
        let mut cursor = start;
        let mut busy = busy_ranges(&freebusy);
        busy.sort_by_key(|(s, _)| *s);
        for (busy_start, busy_end) in busy {
            if cursor + duration <= busy_start {
                return Ok(slot(cursor, cursor + duration));
            }
            if busy_end > cursor {
                cursor = busy_end;
            }
        }
        if cursor + duration <= end {
            Ok(slot(cursor, cursor + duration))
        } else {
            Ok(json!({"available": false}))
        }
    }
}

fn tool(name: &str, description: &str, input_schema: Value) -> McpToolSchema {
    McpToolSchema {
        name: name.into(),
        description: description.into(),
        input_schema,
    }
}

fn event_schema(create: bool) -> Value {
    let mut required = vec!["summary", "start", "end"];
    if !create {
        required = vec!["event_id"];
    }
    json!({
        "type": "object",
        "required": required,
        "properties": {
            "calendar_id": {"type": "string"},
            "event_id": {"type": "string"},
            "summary": {"type": "string"},
            "description": {"type": "string"},
            "location": {"type": "string"},
            "start": {"type": "string"},
            "end": {"type": "string"},
            "time_zone": {"type": "string"},
            "attendees": {"type": "array"}
        }
    })
}

fn json_result(value: Value) -> CallToolResult {
    CallToolResult {
        content: vec![McpContent::Text {
            text: serde_json::to_string(&value).unwrap_or_else(|_| "{}".into()),
        }],
        is_error: false,
    }
}

fn parse_time(value: &str) -> McpResult<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| McpError::Initialize(format!("invalid RFC3339 time {value}: {e}")))
}

fn busy_ranges(value: &Value) -> Vec<(DateTime<Utc>, DateTime<Utc>)> {
    let calendars = value.get("calendars").and_then(Value::as_object);
    let Some(calendars) = calendars else {
        return Vec::new();
    };
    calendars
        .values()
        .flat_map(|cal| {
            cal.get("busy")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
        })
        .filter_map(|busy| {
            let start = busy.get("start").and_then(Value::as_str)?;
            let end = busy.get("end").and_then(Value::as_str)?;
            Some((parse_time(start).ok()?, parse_time(end).ok()?))
        })
        .collect()
}

fn slot(start: DateTime<Utc>, end: DateTime<Utc>) -> Value {
    json!({
        "available": true,
        "start": start.to_rfc3339(),
        "end": end.to_rfc3339()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn service() -> GcalMcpService {
        GcalMcpService::new(GcalClient::new(GcalConfig {
            access_token: "test".into(),
            calendar_id: "primary".into(),
            base_url: "http://127.0.0.1:1".into(),
        }))
    }

    #[test]
    fn advertises_all_five_tools() {
        let names: Vec<String> = service().list_tools().into_iter().map(|t| t.name).collect();
        assert_eq!(names.len(), 5);
        assert!(names.contains(&"gcal.list_events".into()));
        assert!(names.contains(&"gcal.create_event".into()));
        assert!(names.contains(&"gcal.update_event".into()));
        assert!(names.contains(&"gcal.suggest_time".into()));
        assert!(names.contains(&"gcal.respond_to_invite".into()));
    }

    #[test]
    fn finds_gap_after_busy_block() {
        let freebusy = json!({
            "calendars": {
                "primary": {"busy": [{
                    "start": "2026-05-06T09:00:00Z",
                    "end": "2026-05-06T10:00:00Z"
                }]}
            }
        });
        let busy = busy_ranges(&freebusy);
        assert_eq!(busy.len(), 1);
        let next = slot(busy[0].1, busy[0].1 + Duration::minutes(30));
        assert_eq!(next["start"], "2026-05-06T10:00:00+00:00");
    }
}
