//! Telegram Bot API client.
//!
//! `TelegramConnector` wraps `reqwest::Client` for the three primitives the
//! approvals path needs: outbound `sendMessage`, `sendMessage` with a
//! `reply_markup` keyboard, and `answerCallbackQuery` for inline-button
//! taps. The request URL is always
//! `{base_url}/bot{TOKEN}/{method}` (Telegram's path-embedded bot-token
//! scheme); base URL defaults to `https://api.telegram.org` and is
//! overridable through [`TelegramConnector::with_base_url`] for tests.

use crate::types::TelegramMessage;
use argo_types::{ArgoError, ArgoResult};
use serde::Deserialize;

/// Production Telegram Bot API base URL.
pub const TELEGRAM_API_BASE_URL: &str = "https://api.telegram.org";

/// Telegram Bot-API client. Cheap to clone — wraps an `Arc`-backed
/// `reqwest::Client`.
#[derive(Clone)]
pub struct TelegramConnector {
    client: reqwest::Client,
    bot_token: String,
    base_url: String,
}

impl TelegramConnector {
    /// Build a connector with the given bot token. Uses
    /// [`TELEGRAM_API_BASE_URL`] as the base URL.
    #[must_use]
    pub fn new(bot_token: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            bot_token,
            base_url: TELEGRAM_API_BASE_URL.to_string(),
        }
    }

    /// Override the base URL. Test seam — production should leave the
    /// default `https://api.telegram.org`.
    #[must_use]
    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_url = base_url.trim_end_matches('/').to_string();
        self
    }

    /// Send a plain text message via `sendMessage`.
    pub async fn send_message(&self, chat_id: i64, text: &str) -> ArgoResult<TelegramMessage> {
        let body = serde_json::json!({ "chat_id": chat_id, "text": text });
        self.call_with_result("sendMessage", body).await
    }

    /// Send a message with a custom reply keyboard. `buttons` is a row-major
    /// grid of labels — each inner `Vec<String>` becomes one keyboard row.
    pub async fn send_reply_keyboard(
        &self,
        chat_id: i64,
        text: &str,
        buttons: Vec<Vec<String>>,
    ) -> ArgoResult<TelegramMessage> {
        let body = serde_json::json!({
            "chat_id": chat_id,
            "text": text,
            "reply_markup": {
                "keyboard": buttons,
                "resize_keyboard": true,
                "one_time_keyboard": true,
            }
        });
        self.call_with_result("sendMessage", body).await
    }

    /// Acknowledge an inline-keyboard callback. `text` is an optional toast
    /// shown to the user (≤200 chars per Telegram docs).
    pub async fn answer_callback_query(
        &self,
        query_id: &str,
        text: Option<&str>,
    ) -> ArgoResult<()> {
        let mut body = serde_json::json!({ "callback_query_id": query_id });
        if let Some(t) = text {
            body["text"] = serde_json::Value::String(t.to_string());
        }
        let _: TelegramResponse<serde_json::Value> = self.call("answerCallbackQuery", body).await?;
        Ok(())
    }

    async fn call_with_result<T>(
        &self,
        method: &str,
        body: serde_json::Value,
    ) -> ArgoResult<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let resp: TelegramResponse<T> = self.call(method, body).await?;
        resp.into_result(method)
    }

    async fn call<T>(&self, method: &str, body: serde_json::Value) -> ArgoResult<TelegramResponse<T>>
    where
        T: for<'de> Deserialize<'de>,
    {
        let url = format!("{}/bot{}/{}", self.base_url, self.bot_token, method);
        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| ArgoError::Provider(format!("telegram {method}: send: {e}")))?;
        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| ArgoError::Provider(format!("telegram {method}: read: {e}")))?;
        if let Ok(parsed) = serde_json::from_str::<TelegramResponse<T>>(&text) {
            if status.is_success() {
                Ok(parsed)
            } else {
                Err(parsed.into_error(method, status))
            }
        } else {
            Err(ArgoError::Provider(format!(
                "telegram {method}: HTTP {status}: {text}"
            )))
        }
    }
}

#[derive(Deserialize)]
struct TelegramResponse<T> {
    ok: bool,
    result: Option<T>,
    description: Option<String>,
    error_code: Option<i64>,
}

impl<T> TelegramResponse<T> {
    fn into_result(self, method: &str) -> ArgoResult<T> {
        if self.ok {
            self.result.ok_or_else(|| {
                ArgoError::Provider(format!("telegram {method}: ok=true but result missing"))
            })
        } else {
            Err(ArgoError::Provider(format!(
                "telegram {method}: API error code={} desc={}",
                self.error_code.unwrap_or(0),
                self.description.as_deref().unwrap_or("(none)")
            )))
        }
    }

    fn into_error(self, method: &str, status: reqwest::StatusCode) -> ArgoError {
        ArgoError::Provider(format!(
            "telegram {method}: HTTP {status}: code={} desc={}",
            self.error_code.unwrap_or(0),
            self.description.as_deref().unwrap_or("(none)")
        ))
    }
}
