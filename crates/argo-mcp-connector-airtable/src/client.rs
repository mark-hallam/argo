//! Thin Airtable API HTTP client.
use crate::auth;
use argo_mcp::{CallToolResult, McpContent};

/// Airtable API client authenticated with a personal access token.
#[derive(Clone)]
pub struct AirtableClient {
    http: reqwest::Client,
    base_url: String,
    pat: String,
}

impl AirtableClient {
    /// Build from `AIRTABLE_PAT`.
    pub fn from_env() -> Result<Self, String> {
        Self::new(auth::pat_from_env()?, auth::base_url_from_env())
    }

    /// Build with an explicit token and base URL.
    pub fn new(pat: String, base_url: String) -> Result<Self, String> {
        if pat.trim().is_empty() {
            return Err(format!("missing {}", auth::AIRTABLE_PAT_ENV));
        }
        let http = reqwest::Client::builder()
            .user_agent(concat!(
                "argo-mcp-connector-airtable/",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .map_err(|e| format!("airtable http client: {e}"))?;
        Ok(Self {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
            pat,
        })
    }

    /// GET `/meta/bases`.
    pub async fn list_bases(&self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        self.get("/meta/bases", common_query(args)).await
    }

    /// GET `/meta/bases/{base_id}/tables`.
    pub async fn list_tables(&self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        let base_id = path_segment(&required_str(args, "base_id")?)?;
        self.get(&format!("/meta/bases/{base_id}/tables"), Vec::new())
            .await
    }

    /// GET `/{base_id}/{table}` with optional `filterByFormula`.
    pub async fn list_records(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        self.get(&table_path(args)?, record_query(args)).await
    }

    /// GET `/{base_id}/{table}/{record_id}`.
    pub async fn get_record(&self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        let record_id = path_segment(&required_str(args, "record_id")?)?;
        self.get(&format!("{}/{record_id}", table_path(args)?), Vec::new())
            .await
    }

    /// POST `/{base_id}/{table}`.
    pub async fn create_record(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        self.post_json(&table_path(args)?, record_body(args)?).await
    }

    /// PATCH `/{base_id}/{table}/{record_id}`.
    pub async fn update_record(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let record_id = path_segment(&required_str(args, "record_id")?)?;
        self.patch_json(
            &format!("{}/{record_id}", table_path(args)?),
            record_body(args)?,
        )
        .await
    }

    /// DELETE `/{base_id}/{table}/{record_id}`.
    pub async fn delete_record(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let record_id = path_segment(&required_str(args, "record_id")?)?;
        self.delete(&format!("{}/{record_id}", table_path(args)?))
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
            .bearer_auth(&self.pat)
            .query(&query)
            .send()
            .await
            .map_err(|e| format!("airtable GET {path}: {e}"))?;
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
            .bearer_auth(&self.pat)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("airtable POST {path}: {e}"))?;
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
            .bearer_auth(&self.pat)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("airtable PATCH {path}: {e}"))?;
        decode_response("PATCH", path, resp).await
    }

    async fn delete(&self, path: &str) -> Result<serde_json::Value, String> {
        let resp = self
            .http
            .delete(format!("{}{}", self.base_url, path))
            .bearer_auth(&self.pat)
            .send()
            .await
            .map_err(|e| format!("airtable DELETE {path}: {e}"))?;
        decode_response("DELETE", path, resp).await
    }
}

/// Convert an Airtable tool result into MCP text content.
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
        .map_err(|e| format!("airtable {method} {path}: read response: {e}"))?;
    let parsed = if text.trim().is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_str(&text)
            .map_err(|e| format!("airtable {method} {path}: decode response: {e}: {text}"))?
    };
    if status.is_success() {
        Ok(parsed)
    } else {
        Err(format!("airtable {method} {path}: HTTP {status}: {parsed}"))
    }
}

fn table_path(args: &serde_json::Value) -> Result<String, String> {
    let base_id = path_segment(&required_str(args, "base_id")?)?;
    let table = path_segment(&required_any_str(args, &["table", "table_id"])?)?;
    Ok(format!("/{base_id}/{table}"))
}

fn record_body(args: &serde_json::Value) -> Result<serde_json::Value, String> {
    let mut body = serde_json::Map::new();
    body.insert("fields".into(), required_object(args, "fields")?);
    copy_bool(args, &mut body, "typecast");
    Ok(serde_json::Value::Object(body))
}

fn common_query(args: &serde_json::Value) -> Vec<(String, String)> {
    let mut query = Vec::new();
    push_u64(args, &mut query, "pageSize", "page_size", 100);
    push_u64(args, &mut query, "maxRecords", "max_records", 1000);
    push_str(args, &mut query, "offset", "offset");
    query
}

fn record_query(args: &serde_json::Value) -> Vec<(String, String)> {
    let mut query = common_query(args);
    push_str(args, &mut query, "view", "view");
    push_str(args, &mut query, "filterByFormula", "filterByFormula");
    if let Some(fields) = args.get("fields").and_then(|v| v.as_array()) {
        for field in fields.iter().filter_map(|v| v.as_str()) {
            query.push(("fields[]".into(), field.to_string()));
        }
    }
    query
}

fn required_str(args: &serde_json::Value, field: &str) -> Result<String, String> {
    args.get(field)
        .and_then(|v| v.as_str())
        .filter(|v| !v.trim().is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("missing string field '{field}'"))
}

fn required_any_str(args: &serde_json::Value, fields: &[&str]) -> Result<String, String> {
    fields
        .iter()
        .find_map(|field| required_str(args, field).ok())
        .ok_or_else(|| format!("missing string field '{}', or '{}'", fields[0], fields[1]))
}

fn required_object(args: &serde_json::Value, field: &str) -> Result<serde_json::Value, String> {
    args.get(field)
        .filter(|v| v.is_object())
        .cloned()
        .ok_or_else(|| format!("missing object field '{field}'"))
}

fn copy_bool(
    args: &serde_json::Value,
    out: &mut serde_json::Map<String, serde_json::Value>,
    field: &str,
) {
    if let Some(value) = args.get(field).and_then(|v| v.as_bool()) {
        out.insert(field.into(), value.into());
    }
}

fn push_str(
    args: &serde_json::Value,
    out: &mut Vec<(String, String)>,
    wire_name: &str,
    arg_name: &str,
) {
    if let Some(value) = args.get(arg_name).and_then(|v| v.as_str()) {
        out.push((wire_name.into(), value.to_string()));
    }
}

fn push_u64(
    args: &serde_json::Value,
    out: &mut Vec<(String, String)>,
    wire_name: &str,
    arg_name: &str,
    max: u64,
) {
    if let Some(value) = args.get(arg_name).and_then(|v| v.as_u64()) {
        out.push((wire_name.into(), value.clamp(1, max).to_string()));
    }
}

fn path_segment(raw: &str) -> Result<String, String> {
    if raw.trim().is_empty() || raw.contains('/') || raw.contains('\\') {
        return Err(format!("unsafe Airtable path segment: {raw}"));
    }
    Ok(percent_encode(raw))
}

fn percent_encode(raw: &str) -> String {
    let mut out = String::new();
    for byte in raw.as_bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            out.push(char::from(*byte));
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    fn serve_once<F>(body: &'static str, status_line: &'static str, assert_req: F) -> String
    where
        F: FnOnce(&str) + Send + 'static,
    {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = [0; 4096];
            let n = stream.read(&mut buf).unwrap();
            let req = String::from_utf8_lossy(&buf[..n]).to_string();
            assert!(req
                .to_ascii_lowercase()
                .contains("authorization: bearer pat_test"));
            assert_req(&req);
            let response = format!(
                "{status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).unwrap();
        });
        format!("http://{addr}")
    }

    #[tokio::test]
    async fn list_records_passes_filter_formula_through() {
        let base = serve_once(r#"{"records":[]}"#, "HTTP/1.1 200 OK", |req| {
            assert!(req.contains("GET /app123/Table%201"));
            assert!(req.to_ascii_lowercase().contains("filterbyformula="));
        });
        let client = AirtableClient::new("pat_test".into(), base).unwrap();
        let out = client
            .list_records(&serde_json::json!({
                "base_id": "app123",
                "table": "Table 1",
                "filterByFormula": "{Status}='Open'"
            }))
            .await
            .unwrap();
        assert_eq!(out["records"].as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn create_record_posts_fields() {
        let base = serve_once(
            r#"{"id":"rec123","fields":{"Name":"Acme"}}"#,
            "HTTP/1.1 200 OK",
            |req| {
                assert!(req.starts_with("POST /app123/Contacts"));
                assert!(req.contains(r#""fields":{"Name":"Acme"}"#));
            },
        );
        let client = AirtableClient::new("pat_test".into(), base).unwrap();
        let out = client
            .create_record(&serde_json::json!({
                "base_id": "app123",
                "table": "Contacts",
                "fields": {"Name": "Acme"}
            }))
            .await
            .unwrap();
        assert_eq!(out["id"], "rec123");
    }

    #[test]
    fn segments_are_path_safe_and_encoded() {
        assert_eq!(path_segment("Table 1").unwrap(), "Table%201");
        assert!(path_segment("../Contacts").is_err());
        assert!(path_segment("Contacts/2026").is_err());
    }
}
