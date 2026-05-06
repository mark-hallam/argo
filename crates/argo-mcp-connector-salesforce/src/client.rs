//! Thin Salesforce REST API client.
use crate::auth;
use argo_mcp::{CallToolResult, McpContent};

const API_VERSION: &str = "v60.0";

/// Salesforce REST client with bearer token auth.
#[derive(Clone)]
pub struct SalesforceClient {
    http: reqwest::Client,
    base_url: String,
    access_token: String,
}

impl SalesforceClient {
    /// Build from OAuth env vars and token cache.
    pub async fn from_env() -> Result<Self, String> {
        let token = auth::access_token_from_env().await?;
        Self::new(token.access_token, token.instance_url)
    }

    /// Build with explicit token and instance URL.
    pub fn new(access_token: String, instance_url: String) -> Result<Self, String> {
        if access_token.trim().is_empty() {
            return Err("missing Salesforce access token".into());
        }
        let http = reqwest::Client::builder()
            .user_agent(concat!(
                "argo-mcp-connector-salesforce/",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .map_err(|e| format!("salesforce http client: {e}"))?;
        Ok(Self {
            http,
            base_url: format!(
                "{}/services/data/{API_VERSION}",
                instance_url.trim_end_matches('/')
            ),
            access_token,
        })
    }

    /// Run a SOQL query passthrough.
    pub async fn query(&self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        let soql = required_str(args, "soql")?;
        self.get("/query", vec![("q".into(), soql)]).await
    }

    /// Fetch one object by API name and id.
    pub async fn get_object(&self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        let object = clean_segment(&required_str(args, "object")?)?;
        let id = clean_id(&required_str(args, "id")?)?;
        self.get(&format!("/sobjects/{object}/{id}"), Vec::new())
            .await
    }

    /// Create a Salesforce object.
    pub async fn create_object(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let object = clean_segment(&required_str(args, "object")?)?;
        let fields = required_object(args, "fields")?;
        self.post_json(&format!("/sobjects/{object}"), fields).await
    }

    /// Update a Salesforce object.
    pub async fn update_object(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let object = clean_segment(&required_str(args, "object")?)?;
        let id = clean_id(&required_str(args, "id")?)?;
        let fields = required_object(args, "fields")?;
        self.patch_json(&format!("/sobjects/{object}/{id}"), fields)
            .await
    }

    /// List Accounts using SOQL.
    pub async fn list_accounts(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let limit = bounded_limit(args, 20, 200);
        self.query(&serde_json::json!({
            "soql": format!("SELECT Id, Name, Industry, Website FROM Account LIMIT {limit}")
        }))
        .await
    }

    /// List Opportunities using SOQL.
    pub async fn list_opportunities(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let limit = bounded_limit(args, 20, 200);
        self.query(&serde_json::json!({
            "soql": format!(
                "SELECT Id, Name, StageName, CloseDate, Amount, AccountId FROM Opportunity LIMIT {limit}"
            )
        }))
        .await
    }

    /// Add a note to a parent record.
    pub async fn add_note(&self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        let parent_id = clean_id(&required_str(args, "parent_id")?)?;
        let title = args
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Argo note");
        let body = required_str(args, "body")?;
        self.post_json(
            "/sobjects/Note",
            serde_json::json!({
                "ParentId": parent_id,
                "Title": title,
                "Body": body
            }),
        )
        .await
    }

    async fn get(
        &self,
        path: &str,
        query: Vec<(String, String)>,
    ) -> Result<serde_json::Value, String> {
        let resp = self
            .http
            .get(format!("{}{}", self.base_url, path))
            .bearer_auth(&self.access_token)
            .query(&query)
            .send()
            .await
            .map_err(|e| format!("salesforce GET {path}: {e}"))?;
        decode_response("GET", path, resp).await
    }

    async fn post_json(
        &self,
        path: &str,
        body: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let resp = self
            .http
            .post(format!("{}{}", self.base_url, path))
            .bearer_auth(&self.access_token)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("salesforce POST {path}: {e}"))?;
        decode_response("POST", path, resp).await
    }

    async fn patch_json(
        &self,
        path: &str,
        body: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let resp = self
            .http
            .patch(format!("{}{}", self.base_url, path))
            .bearer_auth(&self.access_token)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("salesforce PATCH {path}: {e}"))?;
        decode_response("PATCH", path, resp).await
    }
}

/// Convert a Salesforce result into MCP text content.
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

async fn decode_response(
    method: &str,
    path: &str,
    resp: reqwest::Response,
) -> Result<serde_json::Value, String> {
    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| format!("salesforce {method} {path}: read response: {e}"))?;
    let parsed = if text.trim().is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_str(&text)
            .map_err(|e| format!("salesforce {method} {path}: decode response: {e}: {text}"))?
    };
    if status.is_success() {
        Ok(parsed)
    } else {
        Err(format!(
            "salesforce {method} {path}: HTTP {status}: {parsed}"
        ))
    }
}

fn required_str(args: &serde_json::Value, field: &str) -> Result<String, String> {
    args.get(field)
        .and_then(|v| v.as_str())
        .filter(|v| !v.trim().is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("missing string field '{field}'"))
}

fn required_object(args: &serde_json::Value, field: &str) -> Result<serde_json::Value, String> {
    args.get(field)
        .filter(|v| v.is_object())
        .cloned()
        .ok_or_else(|| format!("missing object field '{field}'"))
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
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_'))
    {
        Ok(raw.to_string())
    } else {
        Err(format!("unsafe Salesforce object segment: {raw}"))
    }
}

fn clean_id(raw: &str) -> Result<String, String> {
    if !raw.is_empty()
        && raw
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
    {
        Ok(raw.to_string())
    } else {
        Err(format!("unsafe Salesforce id: {raw}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    fn serve_once(body: &'static str, status_line: &'static str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = [0; 4096];
            let n = stream.read(&mut buf).unwrap();
            let req = String::from_utf8_lossy(&buf[..n]).to_ascii_lowercase();
            assert!(req.contains("authorization: bearer sf_token"));
            let response = format!(
                "{status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).unwrap();
        });
        format!("http://{addr}")
    }

    #[tokio::test]
    async fn query_passes_soql_through() {
        let base = serve_once(r#"{"totalSize":0,"records":[]}"#, "HTTP/1.1 200 OK");
        let client = SalesforceClient::new("sf_token".into(), base).unwrap();
        let out = client
            .query(&serde_json::json!({"soql": "SELECT Id FROM Account LIMIT 1"}))
            .await
            .unwrap();
        assert_eq!(out["totalSize"], 0);
    }

    #[tokio::test]
    async fn create_object_posts_fields() {
        let base = serve_once(
            r#"{"id":"001xx000003","success":true}"#,
            "HTTP/1.1 201 Created",
        );
        let client = SalesforceClient::new("sf_token".into(), base).unwrap();
        let out = client
            .create_object(&serde_json::json!({
                "object": "Account",
                "fields": {"Name": "Acme"}
            }))
            .await
            .unwrap();
        assert_eq!(out["success"], true);
    }

    #[test]
    fn ids_and_objects_are_path_safe() {
        assert_eq!(clean_segment("Opportunity").unwrap(), "Opportunity");
        assert!(clean_segment("../Account").is_err());
        assert!(clean_id("001xx000003").is_ok());
    }
}
