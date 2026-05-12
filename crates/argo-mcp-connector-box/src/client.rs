//! Box.com REST client.
//!
//! Issues requests against two Box host bases:
//!
//! - `https://api.box.com/2.0/...` for metadata operations
//!   (`/folders/{id}/items`, `/search`, `PUT /files/{id}` for shared link).
//! - `https://upload.box.com/api/2.0/files/content` for multipart file uploads.
//!
//! Auth is `Authorization: Bearer <oauth_token>`. Box returns 401 for expired
//! or revoked tokens; the connector surfaces this as
//! [`argo_types::ArgoError::Provider`] with a hint that the caller's OAuth
//! wrapper should refresh the token (token refresh is out of scope for v1 —
//! see crate docs).
//!
//! Transient transport failures (429 / 5xx / network error) are retried with
//! exponential backoff via [`argo_mcp_connector_shared::RetryPolicy`].

use crate::types::{
    BoxFile, BoxItem, EntryWire, ItemsResp, SharedLinkSettings, SharedLinkUpdateBody,
    SharedLinkUpdateResp, UploadAttributes, UploadParent,
};
use crate::SharedLinkAccess;
use argo_mcp_connector_shared::{default_http_policy, RetryPolicy};
use argo_types::{ArgoError, ArgoResult};
use std::time::Duration;

const API_BASE: &str = "https://api.box.com";
const UPLOAD_BASE: &str = "https://upload.box.com";

/// Default `list_folder` page limit. Box caps per-page at 1000; the connector
/// pulls a single page (no auto-pagination in v1).
const LIST_LIMIT: u32 = 1000;

/// Fields requested on list/search responses. Requesting an explicit field set
/// keeps the wire payload stable across Box's silent schema additions and
/// guarantees the columns the wire DTOs deserialize.
const ITEM_FIELDS: &str = "id,name,type,size,modified_at,etag";

/// Typed Box.com REST client.
pub struct BoxConnector {
    client: reqwest::Client,
    oauth_token: String,
    api_base: String,
    upload_base: String,
    retry: RetryPolicy,
}

impl BoxConnector {
    /// Build a connector with the given OAuth 2.0 bearer access token.
    ///
    /// Token refresh is the caller's responsibility — this connector assumes
    /// the token is valid. On 401 responses, methods return
    /// [`ArgoError::Provider`] with a refresh hint so the caller's wrapper can
    /// refresh and retry.
    #[must_use]
    pub fn new(oauth_token: String) -> Self {
        let client = reqwest::Client::builder()
            .user_agent(concat!(
                "argo-mcp-connector-box/",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            client,
            oauth_token,
            api_base: API_BASE.into(),
            upload_base: UPLOAD_BASE.into(),
            retry: default_http_policy(),
        }
    }

    /// Override the metadata-host base URL. Tests use this to point at a
    /// `wiremock` server.
    #[must_use]
    pub fn with_api_base(mut self, base: impl Into<String>) -> Self {
        self.api_base = base.into().trim_end_matches('/').to_string();
        self
    }

    /// Override the upload-host base URL. Tests use this to point at a
    /// `wiremock` server.
    #[must_use]
    pub fn with_upload_base(mut self, base: impl Into<String>) -> Self {
        self.upload_base = base.into().trim_end_matches('/').to_string();
        self
    }

    /// Override the retry policy. Tests use a tightened single-shot policy so
    /// the suite does not stall on the default 3-attempt backoff.
    #[must_use]
    pub fn with_retry_policy(mut self, policy: RetryPolicy) -> Self {
        self.retry = policy;
        self
    }

    /// `GET /2.0/folders/{folder_id}/items` — list the immediate children of
    /// `folder_id`. Pass `"0"` for the user's root folder.
    pub async fn list_folder(&self, folder_id: &str) -> ArgoResult<Vec<BoxItem>> {
        validate_id(folder_id, "folder_id")?;
        let url = format!(
            "{}/2.0/folders/{folder_id}/items?fields={ITEM_FIELDS}&limit={LIST_LIMIT}",
            self.api_base
        );
        let resp = self
            .send_with_retry(|| self.client.get(&url).bearer_auth(&self.oauth_token))
            .await?;
        let items: ItemsResp = decode_json_response(resp, "list_folder").await?;
        Ok(items
            .entries
            .into_iter()
            .filter_map(EntryWire::into_public)
            .collect())
    }

    /// `POST /api/2.0/files/content` against the upload host — multipart
    /// upload of `content` under `parent_folder_id` with leaf name `name`.
    ///
    /// Box wraps successful uploads in `{ "entries": [...] }`; the connector
    /// extracts the first (and only) entry.
    pub async fn upload_file(
        &self,
        parent_folder_id: &str,
        name: &str,
        content: &[u8],
    ) -> ArgoResult<BoxFile> {
        validate_id(parent_folder_id, "parent_folder_id")?;
        if name.is_empty() {
            return Err(ArgoError::Schema("box upload: name must not be empty".into()));
        }
        let attrs = UploadAttributes {
            name,
            parent: UploadParent {
                id: parent_folder_id,
            },
        };
        let attrs_json = serde_json::to_string(&attrs)
            .map_err(|e| ArgoError::Provider(format!("box encode upload attrs: {e}")))?;
        let url = format!("{}/api/2.0/files/content", self.upload_base);
        let bytes_owned: Vec<u8> = content.to_vec();
        let name_owned: String = name.to_string();
        let resp = self
            .send_with_retry(|| {
                // Rebuild the multipart on every attempt — reqwest's
                // `Form` is not `Clone`, so we cannot reuse it across retries.
                let form = reqwest::multipart::Form::new()
                    .text("attributes", attrs_json.clone())
                    .part(
                        "file",
                        reqwest::multipart::Part::bytes(bytes_owned.clone())
                            .file_name(name_owned.clone())
                            .mime_str("application/octet-stream")
                            .unwrap_or_else(|_| {
                                reqwest::multipart::Part::bytes(bytes_owned.clone())
                                    .file_name(name_owned.clone())
                            }),
                    );
                self.client
                    .post(&url)
                    .bearer_auth(&self.oauth_token)
                    .multipart(form)
            })
            .await?;
        let wrap: ItemsResp = decode_json_response(resp, "upload_file").await?;
        let first = wrap
            .entries
            .into_iter()
            .find_map(|e| match e {
                EntryWire::File(f) => Some(f),
                _ => None,
            })
            .ok_or_else(|| {
                ArgoError::Provider("box upload_file: response had no file entry".into())
            })?;
        Ok(first.into())
    }

    /// `GET /2.0/files/{file_id}/content` — download the raw bytes.
    ///
    /// Box may answer with a 302 redirect to its CDN. reqwest follows
    /// redirects by default (up to 10), so callers get the final body
    /// transparently.
    pub async fn download_file(&self, file_id: &str) -> ArgoResult<Vec<u8>> {
        validate_id(file_id, "file_id")?;
        let url = format!("{}/2.0/files/{file_id}/content", self.api_base);
        let resp = self
            .send_with_retry(|| self.client.get(&url).bearer_auth(&self.oauth_token))
            .await?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(map_http_error(status, "download_file", &text));
        }
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| ArgoError::Provider(format!("box download_file body: {e}")))?;
        Ok(bytes.to_vec())
    }

    /// `PUT /2.0/files/{file_id}?fields=shared_link` — enable a shared link
    /// with the given access level and return its URL.
    ///
    /// Box's `access` levels are preserved verbatim: `open`, `company`,
    /// `collaborators`. See [`SharedLinkAccess`] for semantics.
    pub async fn create_shared_link(
        &self,
        file_id: &str,
        access: SharedLinkAccess,
    ) -> ArgoResult<String> {
        validate_id(file_id, "file_id")?;
        let body = SharedLinkUpdateBody {
            shared_link: SharedLinkSettings {
                access: access.as_wire(),
            },
        };
        let payload = serde_json::to_vec(&body)
            .map_err(|e| ArgoError::Provider(format!("box encode shared_link: {e}")))?;
        let url = format!(
            "{}/2.0/files/{file_id}?fields=shared_link",
            self.api_base
        );
        let resp = self
            .send_with_retry(|| {
                self.client
                    .put(&url)
                    .bearer_auth(&self.oauth_token)
                    .header(reqwest::header::CONTENT_TYPE, "application/json")
                    .body(payload.clone())
            })
            .await?;
        let parsed: SharedLinkUpdateResp = decode_json_response(resp, "create_shared_link").await?;
        Ok(parsed.shared_link.url)
    }

    /// `GET /2.0/search?query=...&limit=...` — search files/folders accessible
    /// to the token's user.
    pub async fn search(&self, query: &str, limit: u32) -> ArgoResult<Vec<BoxItem>> {
        if query.trim().is_empty() {
            return Err(ArgoError::Schema(
                "box search: query must not be empty".into(),
            ));
        }
        let safe_limit = limit.clamp(1, 200);
        let encoded = urlencode(query);
        let url = format!(
            "{}/2.0/search?query={encoded}&limit={safe_limit}&fields={ITEM_FIELDS}",
            self.api_base
        );
        let resp = self
            .send_with_retry(|| self.client.get(&url).bearer_auth(&self.oauth_token))
            .await?;
        let items: ItemsResp = decode_json_response(resp, "search").await?;
        Ok(items
            .entries
            .into_iter()
            .filter_map(EntryWire::into_public)
            .collect())
    }

    async fn send_with_retry<B>(&self, build: B) -> ArgoResult<reqwest::Response>
    where
        B: Fn() -> reqwest::RequestBuilder,
    {
        let max = self.retry.max_attempts.max(1);
        let mut attempt: u32 = 0;
        loop {
            attempt += 1;
            let result = build().send().await;
            let retriable = match &result {
                Ok(r) => {
                    let s = r.status().as_u16();
                    s == 429 || (500..=599).contains(&s)
                }
                Err(_) => true,
            };
            if !retriable || attempt >= max {
                return result.map_err(|e| ArgoError::Provider(format!("box request: {e}")));
            }
            let jitter = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| u64::from(d.subsec_nanos()))
                .unwrap_or(0);
            let delay =
                argo_mcp_connector_shared::retry::backoff_delay_ms(&self.retry, attempt, jitter);
            tokio::time::sleep(Duration::from_millis(delay)).await;
        }
    }
}

fn validate_id(raw: &str, field: &str) -> ArgoResult<()> {
    if raw.is_empty() {
        return Err(ArgoError::Schema(format!("box {field}: must not be empty")));
    }
    if !raw.chars().all(|c| c.is_ascii_digit()) {
        return Err(ArgoError::Schema(format!(
            "box {field}: must be a numeric string, got {raw:?}"
        )));
    }
    Ok(())
}

async fn decode_json_response<T: serde::de::DeserializeOwned>(
    resp: reqwest::Response,
    route: &str,
) -> ArgoResult<T> {
    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| ArgoError::Provider(format!("box read body {route}: {e}")))?;
    if !status.is_success() {
        return Err(map_http_error(status, route, &text));
    }
    serde_json::from_str(&text)
        .map_err(|e| ArgoError::Provider(format!("box decode {route}: {e}: {text}")))
}

fn map_http_error(status: reqwest::StatusCode, route: &str, body: &str) -> ArgoError {
    if status.as_u16() == 401 {
        return ArgoError::Provider(format!(
            "box {route}: HTTP 401 unauthorized — OAuth token is expired or invalid; \
             caller should refresh the token via the OAuth wrapper and retry. \
             body: {body}"
        ));
    }
    ArgoError::Provider(format!("box {route} HTTP {status}: {body}"))
}

/// Minimal URL-form encoder for query strings — escapes the characters that
/// matter for Box search queries: space, `&`, `=`, `?`, `#`, `+`, `%`.
fn urlencode(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for b in raw.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => {
                out.push_str(&format!("%{b:02X}"));
            }
        }
    }
    out
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod client_tests;
