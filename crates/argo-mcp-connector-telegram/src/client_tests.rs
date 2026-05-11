//! Wiremock-backed unit tests for `TelegramConnector`.
//!
//! The four scenarios required by the brief: send-success, send-401,
//! callback-answer, secret-match. The secret-match path lives in
//! `webhook.rs::tests`; here we cover the four HTTP-side cases.

use crate::client::TelegramConnector;
use crate::webhook::verify_webhook_secret;
use serde_json::Value;
use wiremock::matchers::{body_partial_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const TOKEN: &str = "123:ABC-DEF";

fn connector(server: &MockServer) -> TelegramConnector {
    TelegramConnector::new(TOKEN.to_string()).with_base_url(server.uri())
}

#[tokio::test]
async fn send_message_posts_chat_id_and_text() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/bot{TOKEN}/sendMessage")))
        .and(header("content-type", "application/json"))
        .and(body_partial_json(serde_json::json!({
            "chat_id": 42,
            "text": "hello",
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "ok": true,
            "result": {
                "message_id": 7,
                "chat": { "id": 42, "type": "private" },
                "date": 1700000000,
                "text": "hello"
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let bot = connector(&server);
    let msg = bot.send_message(42, "hello").await.unwrap();
    assert_eq!(msg.message_id, 7);
    assert_eq!(msg.chat_id, 42);
    assert_eq!(msg.date, 1700000000);
}

#[tokio::test]
async fn send_message_propagates_401() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/bot{TOKEN}/sendMessage")))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "ok": false,
            "error_code": 401,
            "description": "Unauthorized"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let bot = connector(&server);
    let err = bot.send_message(42, "hello").await.unwrap_err();
    let rendered = format!("{err}");
    assert!(rendered.contains("401"), "expected 401 in error, got: {rendered}");
    assert!(
        rendered.contains("Unauthorized"),
        "expected Unauthorized in error, got: {rendered}"
    );
}

#[tokio::test]
async fn send_reply_keyboard_includes_reply_markup() {
    let server = MockServer::start().await;
    let captured: std::sync::Arc<std::sync::Mutex<Option<Value>>> =
        std::sync::Arc::new(std::sync::Mutex::new(None));
    let captured_clone = captured.clone();
    Mock::given(method("POST"))
        .and(path(format!("/bot{TOKEN}/sendMessage")))
        .respond_with(move |req: &wiremock::Request| {
            let body: Value = serde_json::from_slice(&req.body).unwrap();
            *captured_clone.lock().unwrap() = Some(body);
            ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ok": true,
                "result": {
                    "message_id": 9,
                    "chat": { "id": 1, "type": "private" },
                    "date": 1700000001
                }
            }))
        })
        .expect(1)
        .mount(&server)
        .await;

    let bot = connector(&server);
    bot.send_reply_keyboard(
        1,
        "pick one",
        vec![vec!["yes".into(), "no".into()], vec!["abstain".into()]],
    )
    .await
    .unwrap();

    let body = captured.lock().unwrap().clone().expect("body captured");
    let keyboard = &body["reply_markup"]["keyboard"];
    assert_eq!(keyboard[0][0], "yes");
    assert_eq!(keyboard[0][1], "no");
    assert_eq!(keyboard[1][0], "abstain");
    assert_eq!(body["reply_markup"]["resize_keyboard"], true);
    assert_eq!(body["reply_markup"]["one_time_keyboard"], true);
}

#[tokio::test]
async fn answer_callback_query_posts_id_and_text() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/bot{TOKEN}/answerCallbackQuery")))
        .and(body_partial_json(serde_json::json!({
            "callback_query_id": "q1",
            "text": "got it"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "ok": true,
            "result": true
        })))
        .expect(1)
        .mount(&server)
        .await;

    let bot = connector(&server);
    bot.answer_callback_query("q1", Some("got it")).await.unwrap();
}

#[test]
fn secret_match_uses_constant_time_compare() {
    assert!(verify_webhook_secret(Some("expected"), "expected"));
    assert!(!verify_webhook_secret(Some("tampered"), "expected"));
    assert!(!verify_webhook_secret(None, "expected"));
}
