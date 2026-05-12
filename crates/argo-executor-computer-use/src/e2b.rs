//! E2B sandbox HTTP client. Vendors the few REST calls the
//! computer-use executor needs:
//!
//! - `POST /sandboxes` → open a sandbox of the requested
//!   template; returns a sandbox id + websocket-ish handle.
//! - `POST /sandboxes/<id>/screenshot` → request a fresh PNG.
//! - `POST /sandboxes/<id>/click` / `type` / `key` / `scroll` /
//!   `move` → execute one CU primitive.
//! - `DELETE /sandboxes/<id>` → close + reclaim.
//!
//! The E2B managed service exposes the real REST surface (see
//! https://e2b.dev/docs); this client is intentionally minimal —
//! no streaming, no logs, no file mount API yet (mounts land in
//! a follow-up PR alongside the photo_paths feature).

use crate::e2b_reap::{filter_idle, sandbox_handle, E2bReapReport, E2bSandboxInfo};
use crate::CuError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// The E2B sandbox calls the executor makes. Production
/// implemented by [`E2bClient`] over real HTTP;
/// [`crate::testing::MockE2bClient`] is the scripted offline
/// counterpart used by `tests/cu_harness_offline.rs`.
///
/// The trait mirrors exactly what the executor needs today; new
/// E2B primitives land here in lockstep with the call site.
#[async_trait]
pub trait E2bCall: Send + Sync {
    /// Open a fresh sandbox of the requested template.
    async fn open_sandbox(&self, template: &str) -> Result<E2bSandbox, CuError>;
    /// Capture a fresh screenshot.
    async fn screenshot(&self, sandbox: &E2bSandbox) -> Result<Vec<u8>, CuError>;
    /// Execute one CU primitive in the sandbox.
    async fn execute_action(
        &self,
        sandbox: &E2bSandbox,
        action: &CuAction,
    ) -> Result<ActionResult, CuError>;
    /// Close + reclaim the sandbox.
    async fn close_sandbox(&self, sandbox: &E2bSandbox) -> Result<(), CuError>;
    /// List sandboxes the configured API-key owns. Default impl
    /// returns empty — production [`E2bClient`] overrides to hit
    /// E2B's `GET /sandboxes`; test mocks override to return
    /// scripted rows.
    async fn list_sandboxes(&self) -> Result<Vec<E2bSandboxInfo>, CuError> {
        Ok(Vec::new())
    }
    /// Close sandboxes idle longer than `max_idle_secs`. Default
    /// impl drives [`list_sandboxes`] + [`close_sandbox`] so any
    /// implementor that overrides those two automatically gets a
    /// working reaper. Reaping continues past per-sandbox close
    /// errors; the report carries `(id, message)` pairs for any
    /// failed close so operators see partial progress.
    async fn reap_idle(&self, max_idle_secs: u64) -> Result<E2bReapReport, CuError> {
        let sandboxes = self.list_sandboxes().await?;
        let scanned = sandboxes.len();
        let now = chrono::Utc::now();
        let idle = filter_idle(&sandboxes, max_idle_secs, now);
        let mut reaped: Vec<String> = Vec::new();
        let mut errors: Vec<(String, String)> = Vec::new();
        for info in idle {
            let handle = sandbox_handle(&info.id);
            match self.close_sandbox(&handle).await {
                Ok(()) => reaped.push(info.id),
                Err(e) => errors.push((info.id, e.to_string())),
            }
        }
        Ok(E2bReapReport {
            scanned,
            reaped,
            errors,
        })
    }
}

#[async_trait]
impl E2bCall for E2bClient {
    async fn open_sandbox(&self, template: &str) -> Result<E2bSandbox, CuError> {
        E2bClient::open_sandbox(self, template).await
    }
    async fn screenshot(&self, sandbox: &E2bSandbox) -> Result<Vec<u8>, CuError> {
        E2bClient::screenshot(self, sandbox).await
    }
    async fn execute_action(
        &self,
        sandbox: &E2bSandbox,
        action: &CuAction,
    ) -> Result<ActionResult, CuError> {
        E2bClient::execute_action(self, sandbox, action).await
    }
    async fn close_sandbox(&self, sandbox: &E2bSandbox) -> Result<(), CuError> {
        E2bClient::close_sandbox(self, sandbox).await
    }
    async fn list_sandboxes(&self) -> Result<Vec<E2bSandboxInfo>, CuError> {
        E2bClient::list_sandboxes(self).await
    }
}

/// Thin HTTP client. Holds a `reqwest::Client` so connection
/// reuse + tls config are shared across calls. Cloneable so the
/// executor can stamp one client and pass clones into per-step
/// helpers.
#[derive(Debug, Clone)]
pub struct E2bClient {
    pub(crate) base_url: String,
    pub(crate) api_key: String,
    pub(crate) http: reqwest::Client,
}

impl E2bClient {
    /// Build a client. Caller passes the base URL +
    /// per-request timeout (seconds) so tests can wire a mock
    /// server.
    pub fn new(
        base_url: String,
        api_key: String,
        per_request_timeout_seconds: u32,
    ) -> Result<Self, CuError> {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(u64::from(
                per_request_timeout_seconds,
            )))
            .build()
            .map_err(|e| CuError::Http {
                stage: "e2b/client.build",
                source: e,
            })?;
        Ok(Self {
            base_url,
            api_key,
            http,
        })
    }

    /// Open a sandbox of the requested template. Returns the
    /// sandbox handle the rest of the API uses.
    pub async fn open_sandbox(&self, template: &str) -> Result<E2bSandbox, CuError> {
        let url = format!("{}/sandboxes", self.base_url);
        let body = serde_json::json!({
            "templateId": template,
        });
        let resp = self
            .http
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| CuError::Http {
                stage: "e2b/sandbox.open",
                source: e,
            })?;
        let status = resp.status();
        if !status.is_success() {
            return Err(CuError::HttpStatus {
                stage: "e2b/sandbox.open",
                status: status.as_u16(),
                body: read_body_truncated(resp).await,
            });
        }
        let parsed: SandboxOpenResponse = resp.json().await.map_err(|e| CuError::Http {
            stage: "e2b/sandbox.open.parse",
            source: e,
        })?;
        Ok(E2bSandbox {
            id: parsed.sandbox_id,
        })
    }

    /// Capture a fresh PNG screenshot. Returns raw bytes; the
    /// caller persists to the audit dir.
    pub async fn screenshot(&self, sandbox: &E2bSandbox) -> Result<Vec<u8>, CuError> {
        let url = format!("{}/sandboxes/{}/screenshot", self.base_url, sandbox.id);
        let resp = self
            .http
            .post(&url)
            .bearer_auth(&self.api_key)
            .send()
            .await
            .map_err(|e| CuError::Http {
                stage: "e2b/screenshot",
                source: e,
            })?;
        let status = resp.status();
        if !status.is_success() {
            return Err(CuError::HttpStatus {
                stage: "e2b/screenshot",
                status: status.as_u16(),
                body: read_body_truncated(resp).await,
            });
        }
        let bytes = resp.bytes().await.map_err(|e| CuError::Http {
            stage: "e2b/screenshot.body",
            source: e,
        })?;
        Ok(bytes.to_vec())
    }

    /// Execute one CU primitive in the sandbox. The sandbox
    /// applies the action and returns the outcome (text response
    /// from the synthesised tool, or success/failure marker).
    pub async fn execute_action(
        &self,
        sandbox: &E2bSandbox,
        action: &CuAction,
    ) -> Result<ActionResult, CuError> {
        let path = match action {
            CuAction::Click { .. } => "click",
            CuAction::DoubleClick { .. } => "double-click",
            CuAction::Type { .. } => "type",
            CuAction::Key { .. } => "key",
            CuAction::Scroll { .. } => "scroll",
            CuAction::MouseMove { .. } => "move",
            CuAction::Navigate { .. } => "navigate",
        };
        let url = format!("{}/sandboxes/{}/{path}", self.base_url, sandbox.id);
        let resp = self
            .http
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(action)
            .send()
            .await
            .map_err(|e| CuError::Http {
                stage: "e2b/action",
                source: e,
            })?;
        let status = resp.status();
        if !status.is_success() {
            return Err(CuError::HttpStatus {
                stage: "e2b/action",
                status: status.as_u16(),
                body: read_body_truncated(resp).await,
            });
        }
        let parsed: ActionResult = resp.json().await.map_err(|e| CuError::Http {
            stage: "e2b/action.parse",
            source: e,
        })?;
        Ok(parsed)
    }

    /// Close the sandbox + reclaim resources. The pool holds
    /// onto sandboxes for 10 min idle TTL; explicit close is
    /// only used at executor shutdown / TTL eviction.
    pub async fn close_sandbox(&self, sandbox: &E2bSandbox) -> Result<(), CuError> {
        let url = format!("{}/sandboxes/{}", self.base_url, sandbox.id);
        let resp = self
            .http
            .delete(&url)
            .bearer_auth(&self.api_key)
            .send()
            .await
            .map_err(|e| CuError::Http {
                stage: "e2b/sandbox.close",
                source: e,
            })?;
        let status = resp.status();
        if !status.is_success() {
            return Err(CuError::HttpStatus {
                stage: "e2b/sandbox.close",
                status: status.as_u16(),
                body: read_body_truncated(resp).await,
            });
        }
        Ok(())
    }
}

/// Opaque sandbox handle. The id is a UUID-like string the E2B
/// service stamps at open time.
///
/// Named `E2bSandbox` (not plain `Sandbox`) to disambiguate from the
/// other `Sandbox` types in the workspace: `argo_sandbox::Sandbox` is
/// the canonical command-execution seam, and
/// `argo_executor_filesystem::Sandbox` is the filesystem isolation
/// primitive. Keeping the E2B-specific prefix keeps the type's role
/// obvious in the code body without per-site `as ...` aliases.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct E2bSandbox {
    pub id: String,
}

#[derive(Debug, Deserialize)]
struct SandboxOpenResponse {
    #[serde(rename = "sandboxId")]
    sandbox_id: String,
}

/// One CU primitive. Tagged enum — the field set differs per
/// action and serialises to the body the E2B endpoint expects.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CuAction {
    Click { x: u32, y: u32 },
    DoubleClick { x: u32, y: u32 },
    Type { text: String },
    Key { keys: Vec<String> },
    Scroll { x: u32, y: u32, dy: i32 },
    MouseMove { x: u32, y: u32 },
    Navigate { url: String },
}

impl CuAction {
    /// Audit-log-friendly summary. Used as the `cu://action/<n>`
    /// row body in the per-action `evidence` array.
    #[must_use]
    pub fn summary(&self) -> String {
        match self {
            Self::Click { x, y } => format!("click ({x},{y})"),
            Self::DoubleClick { x, y } => format!("double_click ({x},{y})"),
            Self::Type { text } => {
                let preview: String = text.chars().take(40).collect();
                format!("type \"{preview}\"")
            }
            Self::Key { keys } => format!("key {}", keys.join("+")),
            Self::Scroll { x, y, dy } => format!("scroll ({x},{y}) dy={dy}"),
            Self::MouseMove { x, y } => format!("move ({x},{y})"),
            Self::Navigate { url } => format!("navigate {url}"),
        }
    }
}

/// What the sandbox returns after executing one action. The
/// synthesised tool may attach text (e.g. URL after a navigate)
/// that becomes part of the result_json.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActionResult {
    /// Whether the sandbox could carry the action out.
    pub ok: bool,
    /// Optional text payload. Populated by `navigate` (with the
    /// final URL after redirects), `type` (echoed value), etc.
    #[serde(default)]
    pub text: Option<String>,
    /// Optional new screenshot bytes. Some E2B endpoints return
    /// the post-action screenshot inline to save a round trip;
    /// when absent the loop calls `screenshot` separately.
    #[serde(default)]
    pub screenshot_b64: Option<String>,
}

/// Decode an inline `screenshot_b64` payload from an `ActionResult`
/// into raw PNG bytes. Used by the CU loop to reuse the
/// post-action screenshot the sandbox already shipped inline,
/// avoiding a second HTTP round-trip to `screenshot`. Malformed
/// base64 surfaces as `CuError::Protocol` so callers don't
/// silently fall back to an HTTP fetch — that would mask an
/// upstream bug in the E2B response shape.
pub(crate) fn decode_inline_screenshot(b64: &str) -> Result<Vec<u8>, CuError> {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    STANDARD.decode(b64).map_err(|e| {
        CuError::Protocol(format!(
            "e2b returned malformed inline screenshot: {e}"
        ))
    })
}

async fn read_body_truncated(resp: reqwest::Response) -> String {
    match resp.text().await {
        Ok(t) => {
            let mut s = t;
            if s.len() > 512 {
                s.truncate(512);
                s.push_str("…(truncated)");
            }
            s
        }
        Err(_) => "(body read failed)".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cu_action_summary_redacts_long_type_text() {
        let a = CuAction::Type {
            text: "a".repeat(200),
        };
        let s = a.summary();
        assert!(s.starts_with("type \""));
        assert!(s.len() < 60, "should truncate; got {s:?}");
    }

    #[test]
    fn cu_action_summary_formats_each_kind() {
        assert_eq!(
            CuAction::Click { x: 10, y: 20 }.summary(),
            "click (10,20)"
        );
        assert_eq!(
            CuAction::DoubleClick { x: 1, y: 2 }.summary(),
            "double_click (1,2)"
        );
        assert_eq!(
            CuAction::Key {
                keys: vec!["ctrl".into(), "c".into()]
            }
            .summary(),
            "key ctrl+c"
        );
        assert_eq!(
            CuAction::Scroll {
                x: 0,
                y: 0,
                dy: -30
            }
            .summary(),
            "scroll (0,0) dy=-30"
        );
        assert_eq!(
            CuAction::Navigate {
                url: "https://example.com".into()
            }
            .summary(),
            "navigate https://example.com"
        );
    }

    #[test]
    fn cu_action_serialises_with_kind_tag() {
        let a = CuAction::Click { x: 5, y: 7 };
        let v = serde_json::to_value(&a).unwrap();
        assert_eq!(v["kind"], "click");
        assert_eq!(v["x"], 5);
        assert_eq!(v["y"], 7);
    }

    #[test]
    fn action_result_default_optional_fields() {
        let v = serde_json::from_str::<ActionResult>(r#"{"ok": true}"#).unwrap();
        assert!(v.ok);
        assert!(v.text.is_none());
        assert!(v.screenshot_b64.is_none());
    }

    #[test]
    fn e2b_client_builds_with_timeout() {
        // Sanity: client construction doesn't make a network
        // call; it just stamps the reqwest::Client.
        let c = E2bClient::new("http://localhost:1".into(), "k".into(), 5);
        assert!(c.is_ok());
    }

    #[test]
    fn decode_inline_screenshot_returns_raw_bytes_for_valid_b64() {
        // Standard-alphabet base64 of the canonical PNG signature
        // header bytes — exercises the success path the CU loop
        // takes when E2B ships `screenshot_b64` inline.
        let png_header: &[u8] = b"\x89PNG\r\n\x1a\n";
        use base64::engine::general_purpose::STANDARD;
        use base64::Engine;
        let encoded = STANDARD.encode(png_header);
        let bytes = decode_inline_screenshot(&encoded).expect("valid b64 decodes");
        assert_eq!(bytes, png_header);
    }

    #[test]
    fn decode_inline_screenshot_surfaces_protocol_error_for_malformed_b64() {
        // Trailing `!` is not a valid base64 character; decode
        // must surface `CuError::Protocol` rather than silently
        // falling back to a fresh HTTP fetch (which would mask
        // the upstream bug).
        let err = match decode_inline_screenshot("not_valid_base64!!!") {
            Err(e) => e,
            Ok(bytes) => panic!("malformed b64 must error; got {} bytes", bytes.len()),
        };
        match err {
            CuError::Protocol(msg) => {
                assert!(
                    msg.contains("malformed inline screenshot"),
                    "expected protocol message, got: {msg}",
                );
            }
            other => panic!("expected CuError::Protocol; got: {other:?}"),
        }
    }
}
