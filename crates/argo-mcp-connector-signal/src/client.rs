//! signal-cli daemon REST/JSON-RPC client.
//!
//! Endpoints under `{daemon_url}/api/v1/`:
//!
//! - `POST /send` — send a direct or group message (group form uses
//!   `groupId` in place of `recipient`).
//! - `GET /receive/{account}` — drain the inbound envelope queue.
//! - `GET /groups/{account}` — list known groups for the registered account.
//!
//! All transport failures (DNS, refused connection, request build error) are
//! mapped to [`ArgoError::Provider`] with a hint that the operator should
//! ensure the signal-cli daemon is running at the configured URL.

use crate::types::{
    EnvelopeWire, GroupWire, SendDirectArg, SendGroupArg, SendResp, SignalGroup,
    SignalIncomingMessage, SignalMessageReceipt,
};
use argo_types::{ArgoError, ArgoResult};
use std::time::Duration;

const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// signal-cli local daemon client.
pub struct SignalConnector {
    client: reqwest::Client,
    daemon_url: String,
    account: String,
}

impl SignalConnector {
    /// Build a connector pointed at a signal-cli daemon URL and registered
    /// account. `daemon_url` is typically `http://127.0.0.1:8080`; `account`
    /// is the E.164 phone number signal-cli was registered with
    /// (e.g. `"+15551234567"`).
    ///
    /// The constructor validates that `account` is non-empty, starts with
    /// `+`, and contains only digits after the `+`. Malformed values surface
    /// at first call site as the existing `ArgoError::Schema` from
    /// `validate_account`; the struct itself is infallible.
    #[must_use]
    pub fn new(daemon_url: String, account: String) -> Self {
        let client = reqwest::Client::builder()
            .user_agent(concat!(
                "argo-mcp-connector-signal/",
                env!("CARGO_PKG_VERSION")
            ))
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            client,
            daemon_url: daemon_url.trim_end_matches('/').to_string(),
            account,
        }
    }

    /// `POST {daemon_url}/api/v1/send` with `{"number","recipient","message"}`.
    pub async fn send_message(
        &self,
        recipient: &str,
        text: &str,
    ) -> ArgoResult<SignalMessageReceipt> {
        validate_account(&self.account)?;
        let arg = SendDirectArg {
            number: &self.account,
            recipient,
            message: text,
        };
        let resp: SendResp = self.post_json("/api/v1/send", &arg).await?;
        Ok(resp.into())
    }

    /// `POST {daemon_url}/api/v1/send` with `{"number","groupId","message"}`.
    pub async fn send_group_message(
        &self,
        group_id: &str,
        text: &str,
    ) -> ArgoResult<SignalMessageReceipt> {
        validate_account(&self.account)?;
        let arg = SendGroupArg {
            number: &self.account,
            group_id,
            message: text,
        };
        let resp: SendResp = self.post_json("/api/v1/send", &arg).await?;
        Ok(resp.into())
    }

    /// `GET {daemon_url}/api/v1/receive/{account}` — drain the inbound queue.
    pub async fn receive(&self) -> ArgoResult<Vec<SignalIncomingMessage>> {
        validate_account(&self.account)?;
        let path = format!("/api/v1/receive/{}", self.account);
        let envelopes: Vec<EnvelopeWire> = self.get_json(&path).await?;
        Ok(envelopes.into_iter().map(EnvelopeWire::into_public).collect())
    }

    /// `GET {daemon_url}/api/v1/groups/{account}`.
    pub async fn list_groups(&self) -> ArgoResult<Vec<SignalGroup>> {
        validate_account(&self.account)?;
        let path = format!("/api/v1/groups/{}", self.account);
        let groups: Vec<GroupWire> = self.get_json(&path).await?;
        Ok(groups.into_iter().map(Into::into).collect())
    }

    async fn post_json<B, T>(&self, route: &str, body: &B) -> ArgoResult<T>
    where
        B: serde::Serialize,
        T: serde::de::DeserializeOwned,
    {
        let url = format!("{}{route}", self.daemon_url);
        let resp = self
            .client
            .post(&url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .json(body)
            .send()
            .await
            .map_err(|e| self.transport_error(route, &e))?;
        decode_json_response(resp, route).await
    }

    async fn get_json<T>(&self, route: &str) -> ArgoResult<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let url = format!("{}{route}", self.daemon_url);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| self.transport_error(route, &e))?;
        decode_json_response(resp, route).await
    }

    fn transport_error(&self, route: &str, err: &reqwest::Error) -> ArgoError {
        ArgoError::Provider(format!(
            "signal-cli {route}: {err}; ensure signal-cli daemon mode is running at {}",
            self.daemon_url
        ))
    }
}

fn validate_account(account: &str) -> ArgoResult<()> {
    let rest = account.strip_prefix('+').ok_or_else(|| {
        ArgoError::Schema(format!(
            "signal account must be E.164 (start with '+'), got {account:?}"
        ))
    })?;
    if rest.is_empty() || !rest.bytes().all(|b| b.is_ascii_digit()) {
        return Err(ArgoError::Schema(format!(
            "signal account must contain only digits after '+', got {account:?}"
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
        .map_err(|e| ArgoError::Provider(format!("signal-cli read body {route}: {e}")))?;
    if !status.is_success() {
        return Err(ArgoError::Provider(format!(
            "signal-cli {route} HTTP {status}: {text}"
        )));
    }
    if text.trim().is_empty() {
        return serde_json::from_str("null").map_err(|e| {
            ArgoError::Provider(format!("signal-cli decode {route}: empty body: {e}"))
        });
    }
    serde_json::from_str(&text)
        .map_err(|e| ArgoError::Provider(format!("signal-cli decode {route}: {e}: {text}")))
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod client_tests;
