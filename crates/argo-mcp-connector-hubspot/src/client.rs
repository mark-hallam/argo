//! Thin HubSpot CRM HTTP client.
use argo_mcp::{CallToolResult, McpContent};

const DEFAULT_BASE_URL: &str = "https://api.hubapi.com";

/// HubSpot CRM client authenticated with a private-app token.
#[derive(Clone)]
pub struct HubspotClient {
    http: reqwest::Client,
    base_url: String,
    token: String,
}

impl HubspotClient {
    /// Build from `HUBSPOT_PRIVATE_APP_TOKEN`.
    pub fn from_env() -> Result<Self, String> {
        let token = std::env::var("HUBSPOT_PRIVATE_APP_TOKEN")
            .map_err(|_| "missing HUBSPOT_PRIVATE_APP_TOKEN".to_string())?;
        if token.trim().is_empty() {
            return Err("missing HUBSPOT_PRIVATE_APP_TOKEN".into());
        }
        let base_url =
            std::env::var("HUBSPOT_BASE_URL").unwrap_or_else(|_| DEFAULT_BASE_URL.into());
        Self::new(token, base_url)
    }

    /// Build with an explicit token and base URL.
    pub fn new(token: String, base_url: String) -> Result<Self, String> {
        let http = reqwest::Client::builder()
            .user_agent(concat!(
                "argo-mcp-connector-hubspot/",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .map_err(|e| format!("hubspot http client: {e}"))?;
        Ok(Self {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
            token,
        })
    }

    /// POST `/crm/v3/objects/contacts/search`.
    pub async fn search_contacts(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let mut body = serde_json::Map::new();
        copy_arg(args, &mut body, "query");
        copy_arg(args, &mut body, "properties");
        copy_arg(args, &mut body, "after");
        body.insert("limit".into(), bounded_limit(args, 10, 100).into());
        self.post(
            "/crm/v3/objects/contacts/search",
            serde_json::Value::Object(body),
        )
        .await
    }

    /// POST `/crm/v3/objects/contacts`.
    pub async fn create_contact(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let properties = required_object(args, "properties")?;
        self.post(
            "/crm/v3/objects/contacts",
            serde_json::json!({"properties": properties}),
        )
        .await
    }

    /// POST `/crm/v3/objects/deals`.
    pub async fn create_deal(&self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        let mut body = serde_json::json!({"properties": required_object(args, "properties")?});
        if let Some(associations) = args.get("associations") {
            body["associations"] = associations.clone();
        }
        self.post("/crm/v3/objects/deals", body).await
    }

    /// POST `/crm/v3/objects/notes`.
    pub async fn log_activity(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let note_body = required_str(args, "note_body")?;
        let mut properties = args
            .get("properties")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();
        properties.insert("hs_note_body".into(), note_body.into());
        let ts = args
            .get("timestamp")
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .unwrap_or_else(now_millis_string);
        properties.insert("hs_timestamp".into(), ts.into());
        let mut body = serde_json::json!({"properties": properties});
        if let Some(associations) = args.get("associations") {
            body["associations"] = associations.clone();
        }
        self.post("/crm/v3/objects/notes", body).await
    }

    /// GET `/crm/v3/pipelines/{object_type}` or one pipeline.
    pub async fn get_pipeline(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let object_type = args
            .get("object_type")
            .and_then(|v| v.as_str())
            .unwrap_or("deals");
        let mut path = format!("/crm/v3/pipelines/{}", clean_segment(object_type)?);
        if let Some(id) = args.get("pipeline_id").and_then(|v| v.as_str()) {
            path.push('/');
            path.push_str(&clean_segment(id)?);
        }
        self.get(&path).await
    }

    async fn get(&self, path: &str) -> Result<serde_json::Value, String> {
        let resp = self
            .http
            .get(format!("{}{}", self.base_url, path))
            .bearer_auth(&self.token)
            .send()
            .await
            .map_err(|e| format!("hubspot GET {path}: {e}"))?;
        decode_response(path, resp).await
    }

    async fn post(&self, path: &str, body: serde_json::Value) -> Result<serde_json::Value, String> {
        let resp = self
            .http
            .post(format!("{}{}", self.base_url, path))
            .bearer_auth(&self.token)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("hubspot POST {path}: {e}"))?;
        decode_response(path, resp).await
    }
}

/// Convert a tool result into MCP text content.
pub fn to_call_result(result: Result<serde_json::Value, String>) -> CallToolResult {
    match result {
        Ok(value) => CallToolResult {
            content: vec![McpContent::Text {
                text: value.to_string(),
            }],
            is_error: false,
        },
        Err(text) => CallToolResult {
            content: vec![McpContent::Text { text }],
            is_error: true,
        },
    }
}

async fn decode_response(path: &str, resp: reqwest::Response) -> Result<serde_json::Value, String> {
    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| format!("hubspot {path}: read response: {e}"))?;
    let parsed = if text.trim().is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_str(&text)
            .map_err(|e| format!("hubspot {path}: decode response: {e}: {text}"))?
    };
    if status.is_success() {
        return Ok(parsed);
    }
    Err(format!("hubspot {path}: HTTP {status}: {parsed}"))
}

fn required_object(args: &serde_json::Value, field: &str) -> Result<serde_json::Value, String> {
    args.get(field)
        .filter(|v| v.is_object())
        .cloned()
        .ok_or_else(|| format!("missing object field '{field}'"))
}

fn required_str(args: &serde_json::Value, field: &str) -> Result<String, String> {
    args.get(field)
        .and_then(|v| v.as_str())
        .filter(|v| !v.trim().is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("missing string field '{field}'"))
}

fn copy_arg(
    args: &serde_json::Value,
    body: &mut serde_json::Map<String, serde_json::Value>,
    field: &str,
) {
    if let Some(value) = args.get(field) {
        body.insert(field.into(), value.clone());
    }
}

fn bounded_limit(args: &serde_json::Value, default: u64, max: u64) -> u64 {
    args.get("limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(default)
        .clamp(1, max)
}

fn clean_segment(raw: &str) -> Result<String, String> {
    if !raw.is_empty()
        && raw
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
    {
        Ok(raw.to_string())
    } else {
        Err(format!("unsafe HubSpot path segment: {raw}"))
    }
}

fn now_millis_string() -> String {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    millis.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_token_is_clear() {
        std::env::remove_var("HUBSPOT_PRIVATE_APP_TOKEN");
        let err = match HubspotClient::from_env() {
            Ok(_) => panic!("expected missing token error"),
            Err(err) => err,
        };
        assert!(err.contains("HUBSPOT_PRIVATE_APP_TOKEN"));
    }

    #[test]
    fn path_segments_are_strict() {
        assert_eq!(clean_segment("deals").unwrap(), "deals");
        assert!(clean_segment("../deals").is_err());
    }
}
