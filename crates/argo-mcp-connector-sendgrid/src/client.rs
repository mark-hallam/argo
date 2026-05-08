//! Thin SendGrid REST API client.

use crate::auth::{SendgridConfig, SENDGRID_API_KEY_ENV};
use argo_mcp::{CallToolResult, McpContent};

/// SendGrid REST client authenticated with a bearer API key.
#[derive(Clone)]
pub struct SendgridClient {
    http: reqwest::Client,
    base_url: String,
    api_key: String,
}

impl SendgridClient {
    /// Build from `SENDGRID_API_KEY`.
    pub fn from_env() -> Result<Self, String> {
        let cfg = SendgridConfig::from_env()?;
        Self::new(cfg.api_key, cfg.base_url)
    }

    /// Build with explicit API key and REST API base URL.
    pub fn new(api_key: String, base_url: String) -> Result<Self, String> {
        if api_key.trim().is_empty() {
            return Err(format!("missing {SENDGRID_API_KEY_ENV}"));
        }
        let http = reqwest::Client::builder()
            .user_agent(concat!(
                "argo-mcp-connector-sendgrid/",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .map_err(|e| format!("sendgrid http client: {e}"))?;
        Ok(Self {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
        })
    }

    /// GET `/templates`.
    pub async fn list_templates(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let mut query = page_query(args);
        query.push((
            "generations".into(),
            optional_str(args, "generations").unwrap_or_else(|| "dynamic".into()),
        ));
        self.get("/templates", query).await
    }

    /// POST `/mail/send`.
    pub async fn send_email(&self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        self.post_json("/mail/send", send_email_body(args)?).await
    }

    /// GET `/marketing/contacts`.
    pub async fn list_contacts(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        self.get("/marketing/contacts", page_query(args)).await
    }

    /// PUT `/marketing/contacts`.
    pub async fn add_contact(&self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        self.put_json("/marketing/contacts", add_contact_body(args)?)
            .await
    }

    /// GET `/marketing/lists`.
    pub async fn list_lists(&self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        self.get("/marketing/lists", page_query(args)).await
    }

    async fn get(
        &self,
        path: &str,
        query: Vec<(String, String)>,
    ) -> Result<serde_json::Value, String> {
        let resp = self
            .http
            .get(format!("{}{}", self.base_url, path))
            .bearer_auth(&self.api_key)
            .query(&query)
            .send()
            .await
            .map_err(|e| format!("sendgrid GET {path}: {e}"))?;
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
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("sendgrid POST {path}: {e}"))?;
        decode_response("POST", path, resp).await
    }

    async fn put_json(
        &self,
        path: &str,
        body: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let resp = self
            .http
            .put(format!("{}{}", self.base_url, path))
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("sendgrid PUT {path}: {e}"))?;
        decode_response("PUT", path, resp).await
    }
}

/// Convert a SendGrid result into MCP text content.
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
        .map_err(|e| format!("sendgrid {method} {path}: read response: {e}"))?;
    let parsed = if text.trim().is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_str(&text)
            .map_err(|e| format!("sendgrid {method} {path}: decode response: {e}: {text}"))?
    };
    if status.is_success() {
        Ok(parsed)
    } else {
        Err(format!("sendgrid {method} {path}: HTTP {status}: {parsed}"))
    }
}

fn send_email_body(args: &serde_json::Value) -> Result<serde_json::Value, String> {
    let mut body = serde_json::Map::new();
    body.insert("from".into(), email_object(required_str(args, "from")?));
    body.insert(
        "personalizations".into(),
        serde_json::json!([{ "to": email_list(args, "to")? }]),
    );
    insert_optional(args, &mut body, "subject");
    let mut content = Vec::new();
    if let Some(text) = optional_str(args, "text") {
        content.push(serde_json::json!({"type": "text/plain", "value": text}));
    }
    if let Some(html) = optional_str(args, "html") {
        content.push(serde_json::json!({"type": "text/html", "value": html}));
    }
    if !content.is_empty() {
        body.insert("content".into(), serde_json::Value::Array(content));
    }
    if let Some(template_id) = optional_str(args, "template_id") {
        body.insert("template_id".into(), template_id.into());
    }
    if let Some(data) = args
        .get("dynamic_template_data")
        .filter(|value| value.is_object())
    {
        body["personalizations"][0]["dynamic_template_data"] = data.clone();
    }
    if let Some(categories) = string_array(args, "categories") {
        body.insert("categories".into(), categories);
    }
    Ok(serde_json::Value::Object(body))
}

fn add_contact_body(args: &serde_json::Value) -> Result<serde_json::Value, String> {
    let mut contact = serde_json::Map::new();
    contact.insert("email".into(), required_str(args, "email")?.into());
    insert_optional(args, &mut contact, "first_name");
    insert_optional(args, &mut contact, "last_name");
    if let Some(fields) = args.get("custom_fields").filter(|value| value.is_object()) {
        contact.insert("custom_fields".into(), fields.clone());
    }
    Ok(serde_json::json!({"contacts": [serde_json::Value::Object(contact)]}))
}

fn email_list(args: &serde_json::Value, field: &str) -> Result<Vec<serde_json::Value>, String> {
    let values = args
        .get(field)
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| format!("missing array field '{field}'"))?;
    let mut out = Vec::new();
    for value in values {
        let email = value
            .as_str()
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| format!("invalid email in '{field}'"))?;
        out.push(email_object(email.to_string()));
    }
    if out.is_empty() {
        Err(format!("missing array field '{field}'"))
    } else {
        Ok(out)
    }
}

fn email_object(email: String) -> serde_json::Value {
    serde_json::json!({ "email": email })
}

fn page_query(args: &serde_json::Value) -> Vec<(String, String)> {
    let mut query = Vec::new();
    if let Some(size) = args.get("page_size").and_then(serde_json::Value::as_u64) {
        query.push(("page_size".into(), size.clamp(1, 200).to_string()));
    }
    push_query_str(args, &mut query, "page_token");
    query
}

fn push_query_str(args: &serde_json::Value, query: &mut Vec<(String, String)>, field: &str) {
    if let Some(value) = optional_str(args, field) {
        query.push((field.into(), value));
    }
}

fn insert_optional(
    args: &serde_json::Value,
    body: &mut serde_json::Map<String, serde_json::Value>,
    field: &str,
) {
    if let Some(value) = optional_str(args, field) {
        body.insert(field.into(), value.into());
    }
}

fn required_str(args: &serde_json::Value, field: &str) -> Result<String, String> {
    optional_str(args, field).ok_or_else(|| format!("missing string field '{field}'"))
}

fn optional_str(args: &serde_json::Value, field: &str) -> Option<String> {
    args.get(field)
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
}

fn string_array(args: &serde_json::Value, field: &str) -> Option<serde_json::Value> {
    let values = args.get(field)?.as_array()?;
    let strings: Vec<serde_json::Value> = values
        .iter()
        .filter_map(|value| value.as_str())
        .filter(|value| !value.trim().is_empty())
        .map(|value| value.to_string().into())
        .collect();
    (!strings.is_empty()).then_some(serde_json::Value::Array(strings))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    async fn serve_once(
        status: &'static str,
        response: &'static str,
    ) -> (String, tokio::task::JoinHandle<String>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().expect("addr");
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept");
            let mut buf = vec![0_u8; 8192];
            let n = socket.read(&mut buf).await.expect("read");
            let req = String::from_utf8_lossy(&buf[..n]).to_string();
            let wire = format!(
                "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{}",
                response.len(),
                response
            );
            socket.write_all(wire.as_bytes()).await.expect("write");
            req
        });
        (format!("http://{addr}"), server)
    }

    fn request_body(req: &str) -> serde_json::Value {
        let body = req.split("\r\n\r\n").nth(1).expect("http body");
        serde_json::from_str(body).expect("json body")
    }

    #[tokio::test]
    async fn list_templates_uses_bearer_auth_and_generation_query() {
        let (base, server) = serve_once("200 OK", r#"{"templates":[]}"#).await;
        let client = SendgridClient::new("sg_key".into(), base).expect("client");
        let out = client
            .list_templates(&serde_json::json!({"page_size": 5}))
            .await
            .expect("templates");
        assert!(out["templates"].is_array());
        let req = server.await.expect("server");
        assert!(req
            .to_ascii_lowercase()
            .contains("authorization: bearer sg_key"));
        assert!(req.starts_with("GET /templates?page_size=5&generations=dynamic "));
    }

    #[tokio::test]
    async fn send_email_posts_mail_send_with_template_passthrough() {
        let (base, server) = serve_once("202 Accepted", "").await;
        let client = SendgridClient::new("sg_key".into(), base).expect("client");
        client
            .send_email(&serde_json::json!({
                "from": "from@example.com",
                "to": ["to@example.com"],
                "subject": "Hello",
                "template_id": "d-template",
                "dynamic_template_data": {"name": "Ada"}
            }))
            .await
            .expect("send");
        let req = server.await.expect("server");
        assert!(req.starts_with("POST /mail/send "));
        let body = request_body(&req);
        assert_eq!(body["template_id"], "d-template");
        assert_eq!(
            body["personalizations"][0]["dynamic_template_data"]["name"],
            "Ada"
        );
    }

    #[tokio::test]
    async fn add_contact_puts_contacts_array() {
        let (base, server) = serve_once("202 Accepted", r#"{"job_id":"job"}"#).await;
        let client = SendgridClient::new("sg_key".into(), base).expect("client");
        client
            .add_contact(&serde_json::json!({
                "email": "a@example.com",
                "first_name": "Ada",
                "custom_fields": {"role": "engineer"}
            }))
            .await
            .expect("add contact");
        let req = server.await.expect("server");
        assert!(req.starts_with("PUT /marketing/contacts "));
        let body = request_body(&req);
        assert_eq!(body["contacts"][0]["email"], "a@example.com");
        assert_eq!(body["contacts"][0]["first_name"], "Ada");
        assert_eq!(body["contacts"][0]["custom_fields"]["role"], "engineer");
    }
}
