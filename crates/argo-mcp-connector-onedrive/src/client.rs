//! Microsoft Graph (OneDrive) REST client — `graph.microsoft.com/v1.0`.
//! Bearer auth (refresh out of scope). Upload splits at 4 MiB: simple `PUT
//! /content` below the cap, resumable `createUploadSession` + chunked PUT
//! above. Shared-drive ids `"<driveId>!<itemId>"` route through `/drives/`.

use crate::types::{
    ChildrenResp, CreateLinkArg, CreateLinkResp, DriveItem, DriveItemWire, LinkScope, LinkType,
    SearchResp, UploadSessionResp, UploadStatusResp,
};
use argo_mcp_connector_shared::{default_http_policy, RetryPolicy};
use argo_types::{ArgoError, ArgoResult};
use std::time::Duration;

const GRAPH_BASE: &str = "https://graph.microsoft.com/v1.0";

/// Hard boundary between simple and resumable upload, per the Microsoft Graph
/// docs. Payloads strictly less than this go through `PUT /content`; equal-or
/// -greater payloads must use a resumable session.
pub const SIMPLE_UPLOAD_CAP: usize = 4 * 1024 * 1024;

/// Default chunk size for resumable uploads. Graph requires chunks to be a
/// multiple of 320 KiB; 5 MiB (16 × 320 KiB) is the documented sweet spot.
pub const DEFAULT_CHUNK_BYTES: usize = 5 * 1024 * 1024;

/// Typed Microsoft Graph OneDrive client.
pub struct OneDriveConnector {
    client: reqwest::Client,
    oauth_token: String,
    base: String,
    retry: RetryPolicy,
    chunk_bytes: usize,
}

impl OneDriveConnector {
    /// Build a connector with the given OAuth2 access token. Refresh is
    /// out-of-scope; callers must hand fresh tokens in on rotation.
    #[must_use]
    pub fn new(oauth_token: String) -> Self {
        let client = reqwest::Client::builder()
            .user_agent(concat!(
                "argo-mcp-connector-onedrive/",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            client,
            oauth_token,
            base: GRAPH_BASE.into(),
            retry: default_http_policy(),
            chunk_bytes: DEFAULT_CHUNK_BYTES,
        }
    }

    /// Override the Graph base URL. Tests use this to point at `wiremock`.
    #[must_use]
    pub fn with_base(mut self, base: impl Into<String>) -> Self {
        self.base = base.into().trim_end_matches('/').to_string();
        self
    }

    /// Override the retry policy. Tests use a tightened single-shot policy so
    /// the suite does not stall on the default backoff.
    #[must_use]
    pub fn with_retry_policy(mut self, policy: RetryPolicy) -> Self {
        self.retry = policy;
        self
    }

    /// Override the resumable-upload chunk size. Tests shrink this so the
    /// chunked path can be exercised on small fixtures. Graph requires a
    /// multiple of 320 KiB in production but the connector does not enforce
    /// that — Graph returns `invalidRange` on misalignment.
    #[must_use]
    pub fn with_chunk_bytes(mut self, bytes: usize) -> Self {
        self.chunk_bytes = bytes.max(1);
        self
    }

    /// `GET /me/drive/{root,items/<id>}/children` — list the immediate
    /// children of `item_id`, or of `/me/drive/root` when `item_id` is
    /// `None`. Items missing both `file` and `folder` facets are dropped.
    pub async fn list_folder(&self, item_id: Option<&str>) -> ArgoResult<Vec<DriveItem>> {
        let path = match item_id {
            None => "/me/drive/root/children".to_string(),
            Some(id) => format!("{}/children", self.encode_item_id(id)),
        };
        let url = format!("{}{path}", self.base);
        let resp = self
            .send_with_retry(|| self.client.get(&url).bearer_auth(&self.oauth_token))
            .await?;
        let body: ChildrenResp = decode_json(resp, "list_folder").await?;
        Ok(body
            .value
            .into_iter()
            .filter_map(DriveItemWire::into_public)
            .collect())
    }

    /// `PUT /me/drive/items/<parent>:/<name>:/content` — single-request
    /// upload. Payload size must be strictly less than [`SIMPLE_UPLOAD_CAP`];
    /// otherwise the call returns `ArgoError::Provider` *without* a network
    /// round-trip.
    pub async fn upload_small_file(
        &self,
        parent: &str,
        name: &str,
        content: &[u8],
    ) -> ArgoResult<DriveItem> {
        if content.len() >= SIMPLE_UPLOAD_CAP {
            return Err(ArgoError::Provider(format!(
                "onedrive upload_small_file: payload {} bytes >= 4 MiB simple-upload cap; \
                 use upload_large_file (resumable session) for this size",
                content.len()
            )));
        }
        let url = format!(
            "{}{}:/{}:/content",
            self.base,
            self.encode_item_id(parent),
            urlencode_segment(name)
        );
        let resp = self
            .send_with_retry(|| {
                self.client
                    .put(&url)
                    .bearer_auth(&self.oauth_token)
                    .header(reqwest::header::CONTENT_TYPE, "application/octet-stream")
                    .body(content.to_vec())
            })
            .await?;
        let wire: DriveItemWire = decode_json(resp, "upload_small_file").await?;
        wire.into_public().ok_or_else(|| {
            ArgoError::Provider("onedrive upload_small_file: response missing file facet".into())
        })
    }

    /// `POST createUploadSession` + chunked PUT — resumable upload. Required
    /// for payloads at-or-above [`SIMPLE_UPLOAD_CAP`]; smaller payloads also
    /// work but incur the session-creation round-trip, so prefer
    /// `upload_small_file` for those.
    pub async fn upload_large_file(
        &self,
        parent: &str,
        name: &str,
        content: &[u8],
    ) -> ArgoResult<DriveItem> {
        let session_url = self.create_upload_session(parent, name).await?;
        self.put_chunks(&session_url, content).await
    }

    async fn create_upload_session(&self, parent: &str, name: &str) -> ArgoResult<String> {
        let url = format!(
            "{}{}:/{}:/createUploadSession",
            self.base,
            self.encode_item_id(parent),
            urlencode_segment(name)
        );
        let body = serde_json::json!({
            "item": { "@microsoft.graph.conflictBehavior": "replace" }
        });
        let payload = serde_json::to_vec(&body).map_err(|e| {
            ArgoError::Provider(format!("onedrive encode createUploadSession: {e}"))
        })?;
        let resp = self
            .send_with_retry(|| {
                self.client
                    .post(&url)
                    .bearer_auth(&self.oauth_token)
                    .header(reqwest::header::CONTENT_TYPE, "application/json")
                    .body(payload.clone())
            })
            .await?;
        let session: UploadSessionResp = decode_json(resp, "createUploadSession").await?;
        Ok(session.upload_url)
    }

    async fn put_chunks(&self, session_url: &str, content: &[u8]) -> ArgoResult<DriveItem> {
        let total = content.len();
        let mut offset = 0usize;
        let chunk_count = total.div_ceil(self.chunk_bytes.max(1));
        let attempt_cap = chunk_count.saturating_mul(3).max(8);
        for _ in 0..attempt_cap {
            if offset >= total {
                return Err(ArgoError::Provider(
                    "onedrive upload_large_file: payload exhausted before final DriveItem".into(),
                ));
            }
            let end = (offset + self.chunk_bytes).min(total);
            let chunk = &content[offset..end];
            let range = format!("bytes {}-{}/{}", offset, end - 1, total);
            let resp_result = self
                .client
                .put(session_url)
                .bearer_auth(&self.oauth_token)
                .header(reqwest::header::CONTENT_LENGTH, chunk.len() as u64)
                .header("Content-Range", range)
                .body(chunk.to_vec())
                .send()
                .await;
            let resp = match resp_result {
                Ok(r) => r,
                Err(_) => {
                    offset = self.query_resume_offset(session_url, total).await?;
                    continue;
                }
            };
            let status = resp.status();
            if status.as_u16() == 202 {
                offset = end;
                continue;
            }
            if status.is_success() {
                let wire: DriveItemWire = decode_json(resp, "upload chunk").await?;
                return wire.into_public().ok_or_else(|| {
                    ArgoError::Provider(
                        "onedrive upload_large_file: final response missing file facet".into(),
                    )
                });
            }
            let code = status.as_u16();
            if code == 429 || code == 416 || (500..=599).contains(&code) {
                offset = self.query_resume_offset(session_url, total).await?;
                continue;
            }
            let text = resp.text().await.unwrap_or_default();
            return Err(ArgoError::Provider(format!(
                "onedrive upload chunk HTTP {status}: {text}"
            )));
        }
        Err(ArgoError::Provider(
            "onedrive upload_large_file: exceeded chunk attempt budget".into(),
        ))
    }

    async fn query_resume_offset(&self, session_url: &str, total: usize) -> ArgoResult<usize> {
        // `nextExpectedRanges` looks like ["524288-", ...]. The leading
        // number is the next byte Graph wants. Empty list = Graph thinks the
        // upload is complete; we fall through to total so the loop exits.
        let resp = self
            .send_with_retry(|| self.client.get(session_url).bearer_auth(&self.oauth_token))
            .await?;
        let status: UploadStatusResp = decode_json(resp, "upload status").await?;
        if let Some(range) = status.next_expected_ranges.first() {
            let start = range.split('-').next().unwrap_or("0");
            let parsed: usize = start.parse().map_err(|e| {
                ArgoError::Provider(format!("onedrive parse nextExpectedRanges {range:?}: {e}"))
            })?;
            Ok(parsed.min(total))
        } else {
            Ok(total)
        }
    }

    /// `GET /me/drive/items/<id>/content` — download. Graph 302s to a
    /// pre-signed CDN URL; `reqwest` follows redirects by default so the
    /// caller receives the file bytes verbatim.
    pub async fn download_file(&self, item_id: &str) -> ArgoResult<Vec<u8>> {
        let url = format!("{}{}/content", self.base, self.encode_item_id(item_id));
        let resp = self
            .send_with_retry(|| self.client.get(&url).bearer_auth(&self.oauth_token))
            .await?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(map_status(status, &format!("download_file: {text}")));
        }
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| ArgoError::Provider(format!("onedrive download body: {e}")))?;
        Ok(bytes.to_vec())
    }

    /// `POST /me/drive/items/<id>/createLink` — generate a shareable URL.
    /// Returns the link's `webUrl` field; the caller is expected to surface
    /// that to the operator UI or downstream tool.
    pub async fn create_share_link(
        &self,
        item_id: &str,
        link_type: LinkType,
        scope: LinkScope,
    ) -> ArgoResult<String> {
        let url = format!("{}{}/createLink", self.base, self.encode_item_id(item_id));
        let arg = CreateLinkArg {
            link_type: link_type.as_str(),
            scope: scope.as_str(),
        };
        let payload = serde_json::to_vec(&arg)
            .map_err(|e| ArgoError::Provider(format!("onedrive encode createLink: {e}")))?;
        let resp = self
            .send_with_retry(|| {
                self.client
                    .post(&url)
                    .bearer_auth(&self.oauth_token)
                    .header(reqwest::header::CONTENT_TYPE, "application/json")
                    .body(payload.clone())
            })
            .await?;
        let body: CreateLinkResp = decode_json(resp, "createLink").await?;
        Ok(body.link.web_url)
    }

    /// `GET /me/drive/root/search(q='...')` — query Graph's drive-scoped
    /// search index. Items missing both file and folder facets are dropped.
    pub async fn search(&self, query: &str) -> ArgoResult<Vec<DriveItem>> {
        let encoded = urlencode_segment(query);
        let url = format!("{}/me/drive/root/search(q='{}')", self.base, encoded);
        let resp = self
            .send_with_retry(|| self.client.get(&url).bearer_auth(&self.oauth_token))
            .await?;
        let body: SearchResp = decode_json(resp, "search").await?;
        Ok(body
            .value
            .into_iter()
            .filter_map(DriveItemWire::into_public)
            .collect())
    }

    /// Returns the path prefix for an item id, honoring Graph's shared-drive
    /// composite (`"<driveId>!<itemId>"`). Public for rustdoc cross-refs but
    /// callers should not need to construct paths manually.
    pub(crate) fn encode_item_id(&self, id: &str) -> String {
        match id.split_once('!') {
            Some((drive_id, item_id)) if !drive_id.is_empty() && !item_id.is_empty() => {
                format!(
                    "/drives/{}/items/{}",
                    urlencode_segment(drive_id),
                    urlencode_segment(id)
                )
            }
            _ => format!("/me/drive/items/{}", urlencode_segment(id)),
        }
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
                return result
                    .map_err(|e| ArgoError::Provider(format!("onedrive request: {e}")));
            }
            let jitter = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| u64::from(d.subsec_nanos()))
                .unwrap_or(0);
            let delay = argo_mcp_connector_shared::retry::backoff_delay_ms(
                &self.retry, attempt, jitter,
            );
            tokio::time::sleep(Duration::from_millis(delay)).await;
        }
    }
}

// `!` stays raw — Graph uses it as the shared-drive separator inside the
// composite id, which is split before this function is reached.
fn urlencode_segment(s: &str) -> String {
    const RESERVED: &[u8] = b"/?#[]@!$&'()*+,;= \"<>\\^`{|}";
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b == b'!' {
            out.push('!');
        } else if b > 0x20 && b < 0x7f && !RESERVED.contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn map_status(status: reqwest::StatusCode, ctx: &str) -> ArgoError {
    if status.as_u16() == 401 {
        ArgoError::Provider(format!(
            "onedrive 401 unauthorized — token expired or insufficient scope (refresh required): {ctx}"
        ))
    } else {
        ArgoError::Provider(format!("onedrive HTTP {status}: {ctx}"))
    }
}

async fn decode_json<T: serde::de::DeserializeOwned>(
    resp: reqwest::Response,
    route: &str,
) -> ArgoResult<T> {
    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| ArgoError::Provider(format!("onedrive read body {route}: {e}")))?;
    if !status.is_success() {
        return Err(map_status(status, &format!("{route}: {text}")));
    }
    serde_json::from_str(&text)
        .map_err(|e| ArgoError::Provider(format!("onedrive decode {route}: {e}: {text}")))
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod client_tests;
