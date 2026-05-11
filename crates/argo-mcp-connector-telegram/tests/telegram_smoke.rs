//! Real-Telegram smoke test, gated by environment variables.
//!
//! Set `ARGO_TELEGRAM_TEST_TOKEN` to a real bot token and
//! `ARGO_TELEGRAM_TEST_CHAT_ID` to a chat the bot can post into. When either
//! is unset, the test exits silently — the swarm's offline workers never
//! see network calls.
//!
//! What it asserts: an outbound `sendMessage` round-trips through the
//! Telegram API, the response decodes into `TelegramMessage`, and the
//! returned `chat_id` matches the requested chat.

use argo_mcp_connector_telegram::{TelegramConnector, TelegramMessage};

const TOKEN_ENV: &str = "ARGO_TELEGRAM_TEST_TOKEN";
const CHAT_ENV: &str = "ARGO_TELEGRAM_TEST_CHAT_ID";

#[tokio::test]
async fn sends_and_parses_real_message() {
    let Ok(token) = std::env::var(TOKEN_ENV) else {
        eprintln!("skipping: {TOKEN_ENV} not set");
        return;
    };
    let Ok(chat_raw) = std::env::var(CHAT_ENV) else {
        eprintln!("skipping: {CHAT_ENV} not set");
        return;
    };
    let chat_id: i64 = match chat_raw.parse() {
        Ok(v) => v,
        Err(e) => {
            panic!("{CHAT_ENV} must be an integer chat id, got '{chat_raw}': {e}");
        }
    };

    let bot = TelegramConnector::new(token);
    let text = format!("argo skycot telegram smoke @ {}", chrono_like_now());
    let msg: TelegramMessage = bot
        .send_message(chat_id, &text)
        .await
        .expect("send_message succeeded");
    assert_eq!(msg.chat_id, chat_id, "echoed chat_id matches request");
    assert!(msg.message_id > 0, "message_id is positive");
    assert!(msg.date > 0, "date is a real unix epoch");

    let serialized = serde_json::to_string(&msg).expect("re-serialize");
    let reparsed: TelegramMessage = serde_json::from_str(&serialized).expect("re-parse");
    assert_eq!(reparsed, msg, "byte-round-trip preserves all fields");
}

fn chrono_like_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
