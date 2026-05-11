//! Dropbox v2 REST client.
//!
//! Issues `POST` requests against the two Dropbox host bases:
//!
//! - `https://api.dropboxapi.com/2/...` for metadata operations
//!   (`files/list_folder`, `files/delete_v2`, `files/search_v2`,
//!   `sharing/create_shared_link_with_settings`). JSON in, JSON out.
//! - `https://content.dropboxapi.com/2/...` for `files/upload` and
//!   `files/download`. The argument JSON travels in the `Dropbox-API-Arg`
//!   header; bodies (upload payload, download response) are raw bytes.
//!
//! Transient transport failures (429 / 5xx / network error) are retried with
//! exponential backoff via [`argo_mcp_connector_shared::default_http_policy`].

use crate::types::{
    DbxEntry, DbxFile, DbxSharedLink, DbxWriteMode, EntryWire, FileWire, ListFolderArg,
    ListFolderResp, PathArg, SearchArg, SearchOpts, SearchResp, SharedLinkResp,
};
use argo_mcp_connector_shared::{default_http_policy, RetryPolicy};
use argo_types::{ArgoError, ArgoResult};
use std::time::Duration;

const API_BASE: &str = "https://api.dropboxapi.com";
const CONTENT_BASE: &str = "https://content.dropboxapi.com";

/// Hard cap on single-request upload size: 150 MiB.
///
/// Larger payloads must be sent via `/files/upload_session/*` (not
/// implemented in v1). The connector rejects oversized uploads with
/// [`ArgoError::Provider`] before issuing the network call.
pub const MAX_UPLOAD_BYTES: usize = 150 * 1024 * 1024;

/// Typed Dropbox v2 client.
pub struct DropboxConnector {
    client: reqwest::Client,
    access_token: String,
    api_base: String,
    content_base: String,
    retry: RetryPolicy,
}

impl DropboxConnector {
    /// Build a connector with the given long-lived OAuth2 access token.
    #[must_use]
    pub fn new(access_token: String) -> Self {
        let client = reqwest::Client::builder()
            .user_agent(concat!(
                "argo-mcp-connector-dropbox/",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            client,
            access_token,
            api_base: API_BASE.into(),
            content_base: CONTENT_BASE.into(),
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

    /// Override the content-host base URL. Tests use this to point at a
    /// `wiremock` server.
    #[must_use]
    pub fn with_content_base(mut self, base: impl Into<String>) -> Self {
        self.content_base = base.into().trim_end_matches('/').to_string();
        self
    }

    /// Override the retry policy. Tests use a tightened single-shot policy
    /// so the suite does not stall on the default 3-attempt backoff.
    #[must_use]
    pub fn with_retry_policy(mut self, policy: RetryPolicy) -> Self {
        self.retry = policy;
        self
    }

    /// `POST /2/files/list_folder` — list the immediate (or recursive)
    /// contents of `path`. Pass `""` for the user's root.
    ///
    /// Tombstoned (`deleted`) entries are dropped.
    pub async fn list_folder(
        &self,
        path: &str,
        recursive: bool,
    ) -> ArgoResult<Vec<DbxEntry>> {
        validate_path(path)?;
        let arg = ListFolderArg { path, recursive };
        let resp: ListFolderResp = self.post_api_json("/2/files/list_folder", &arg).await?;
        Ok(resp.entries.into_iter().filter_map(EntryWire::into_public).collect())
    }

    /// `POST /2/files/upload` against the content host. Enforces the 150 MiB
    /// single-request cap before issuing the call.
    pub async fn upload(
        &self,
        path: &str,
        bytes: &[u8],
        mode: DbxWriteMode,
    ) -> ArgoResult<DbxFile> {
        validate_path(path)?;
        if bytes.len() > MAX_UPLOAD_BYTES {
            return Err(ArgoError::Provider(format!(
                "dropbox upload: payload {} bytes exceeds 150 MiB single-request cap; \
                 chunked upload (upload_session) is not implemented in v1 — \
                 fall back to the document-executor MS Graph UploadOnly path",
                bytes.len()
            )));
        }
        let arg = serde_json::json!({
            "path": path,
            "mode": mode.to_json(),
        });
        let arg_header = serde_json::to_string(&arg)
            .map_err(|e| ArgoError::Provider(format!("dropbox encode upload arg: {e}")))?;
        let url = format!("{}/2/files/upload", self.content_base);
        let resp = self
            .send_with_retry(|| {
                self.client
                    .post(&url)
                    .bearer_auth(&self.access_token)
                    .header("Dropbox-API-Arg", arg_header.clone())
                    .header(reqwest::header::CONTENT_TYPE, "application/octet-stream")
                    .body(bytes.to_vec())
            })
            .await?;
        let wire: FileWire = decode_json_response(resp, "upload").await?;
        Ok(wire.into())
    }

    /// `POST /2/files/download` against the content host. Returns the raw
    /// bytes; the metadata that Dropbox emits in the `Dropbox-API-Result`
    /// response header is intentionally ignored (callers that need it should
    /// re-`list_folder` against the parent).
    pub async fn download(&self, path: &str) -> ArgoResult<Vec<u8>> {
        validate_path(path)?;
        let arg = PathArg { path };
        let arg_header = serde_json::to_string(&arg)
            .map_err(|e| ArgoError::Provider(format!("dropbox encode download arg: {e}")))?;
        let url = format!("{}/2/files/download", self.content_base);
        let resp = self
            .send_with_retry(|| {
                self.client
                    .post(&url)
                    .bearer_auth(&self.access_token)
                    .header("Dropbox-API-Arg", arg_header.clone())
            })
            .await?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(ArgoError::Provider(format!(
                "dropbox download HTTP {status}: {text}"
            )));
        }
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| ArgoError::Provider(format!("dropbox download body: {e}")))?;
        Ok(bytes.to_vec())
    }

    /// `POST /2/files/delete_v2` against the metadata host.
    pub async fn delete(&self, path: &str) -> ArgoResult<()> {
        validate_path(path)?;
        let arg = PathArg { path };
        let _: serde_json::Value = self.post_api_json("/2/files/delete_v2", &arg).await?;
        Ok(())
    }

    /// `POST /2/sharing/create_shared_link_with_settings`. When
    /// `requires_password` is true the link is created with
    /// `requested_visibility = "password"`; the actual password value is
    /// out-of-band (callers must set it through Dropbox's UI or a follow-up
    /// settings call — v1 does not surface that).
    pub async fn create_shared_link(
        &self,
        path: &str,
        requires_password: bool,
    ) -> ArgoResult<DbxSharedLink> {
        validate_path(path)?;
        let visibility = if requires_password { "password" } else { "public" };
        let arg = serde_json::json!({
            "path": path,
            "settings": { "requested_visibility": visibility },
        });
        let resp: SharedLinkResp = self
            .post_api_json("/2/sharing/create_shared_link_with_settings", &arg)
            .await?;
        Ok(resp.into())
    }

    /// `POST /2/files/search_v2`. Returns at most `max_results` live entries.
    pub async fn search(&self, query: &str, max_results: u32) -> ArgoResult<Vec<DbxEntry>> {
        let arg = SearchArg {
            query,
            options: SearchOpts { max_results },
        };
        let resp: SearchResp = self.post_api_json("/2/files/search_v2", &arg).await?;
        Ok(resp
            .matches
            .into_iter()
            .filter_map(|m| m.metadata.into_entry())
            .collect())
    }

    async fn post_api_json<B, T>(&self, route: &str, body: &B) -> ArgoResult<T>
    where
        B: serde::Serialize,
        T: serde::de::DeserializeOwned,
    {
        let payload = serde_json::to_vec(body)
            .map_err(|e| ArgoError::Provider(format!("dropbox encode {route}: {e}")))?;
        let url = format!("{}{route}", self.api_base);
        let resp = self
            .send_with_retry(|| {
                self.client
                    .post(&url)
                    .bearer_auth(&self.access_token)
                    .header(reqwest::header::CONTENT_TYPE, "application/json")
                    .body(payload.clone())
            })
            .await?;
        decode_json_response(resp, route).await
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
                return result.map_err(|e| ArgoError::Provider(format!("dropbox request: {e}")));
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

fn validate_path(path: &str) -> ArgoResult<()> {
    if path.is_empty() || path.starts_with('/') {
        Ok(())
    } else {
        Err(ArgoError::Schema(format!(
            "dropbox path must start with '/' (or be empty for root), got {path:?}"
        )))
    }
}

async fn decode_json_response<T: serde::de::DeserializeOwned>(
    resp: reqwest::Response,
    route: &str,
) -> ArgoResult<T> {
    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| ArgoError::Provider(format!("dropbox read body {route}: {e}")))?;
    if !status.is_success() {
        return Err(ArgoError::Provider(format!(
            "dropbox {route} HTTP {status}: {text}"
        )));
    }
    serde_json::from_str(&text)
        .map_err(|e| ArgoError::Provider(format!("dropbox decode {route}: {e}: {text}")))
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod client_tests;
