//! Wiremock-backed unit tests for `WhatsAppCloudClient`.
//!
//! Eight scenarios per the brief: send-text-happy, send-template-w-3-vars,
//! send-media-image-by-url, send-media-document-w-caption,
//! 429-back-off-w-retry-after-honored, 401-bubbles-as-AuthFailed,
//! template-not-approved-error, outside-24h-window-error.

use super::*;
use crate::types::{WhatsAppCloudError, WhatsAppMediaKind, WhatsAppMediaSource};
use wiremock::matchers::{body_partial_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const PHONE_NUMBER_ID: &str = "100200300";
const TOKEN: &str = "test_token";
const RECIPIENT: &str = "+15557654321";
const ENDPOINT: &str = "/v18.0/100200300/messages";
const EXPECTED_AUTH: &str = "Bearer test_token";

fn client(server: &MockServer) -> WhatsAppCloudClient {
    WhatsAppCloudClient::new(PHONE_NUMBER_ID.to_string())
        .with_base_url(server.uri())
        .with_token_override(TOKEN.to_string())
        .with_max_retry_delay(Duration::from_secs(5))
}

fn ok_response(wamid: &str) -> serde_json::Value {
    serde_json::json!({
        "messaging_product": "whatsapp",
        "contacts": [{"input": RECIPIENT, "wa_id": "15557654321"}],
        "messages": [{"id": wamid}]
    })
}

fn err_response(code: i64, message: &str) -> serde_json::Value {
    serde_json::json!({ "error": {"message": message, "code": code} })
}

#[tokio::test]
async fn send_text_happy_path() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(ENDPOINT))
        .and(header("authorization", EXPECTED_AUTH))
        .and(body_partial_json(serde_json::json!({
            "messaging_product": "whatsapp",
            "recipient_type": "individual",
            "to": RECIPIENT,
            "type": "text",
            "text": {"body": "hello world"}
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(ok_response("wamid.text123")))
        .expect(1)
        .mount(&server)
        .await;
    let id = client(&server).send_text(RECIPIENT, "hello world").await.unwrap();
    assert_eq!(id.as_str(), "wamid.text123");
}

#[tokio::test]
async fn send_template_with_three_variables() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(ENDPOINT))
        .and(body_partial_json(serde_json::json!({
            "to": RECIPIENT,
            "type": "template",
            "template": {
                "name": "appointment_reminder",
                "language": {"code": "en_US"},
                "components": [{"type": "body", "parameters": [
                    {"type": "text", "text": "Mark"},
                    {"type": "text", "text": "Tuesday 9am"},
                    {"type": "text", "text": "+15551234567"}
                ]}]
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(ok_response("wamid.tmpl987")))
        .expect(1)
        .mount(&server)
        .await;
    let vars: Vec<WhatsAppTemplateVariable> = ["Mark", "Tuesday 9am", "+15551234567"]
        .iter().map(|s| WhatsAppTemplateVariable::new(*s)).collect();
    let id = client(&server)
        .send_template(RECIPIENT, "appointment_reminder", "en_US", &vars)
        .await.unwrap();
    assert_eq!(id.as_str(), "wamid.tmpl987");
}

#[tokio::test]
async fn send_media_image_by_url() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(ENDPOINT))
        .and(body_partial_json(serde_json::json!({
            "to": RECIPIENT, "type": "image",
            "image": {"link": "https://cdn.example.test/cat.jpg"}
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(ok_response("wamid.img555")))
        .expect(1).mount(&server).await;
    let src = WhatsAppMediaSource::Url("https://cdn.example.test/cat.jpg".into());
    let id = client(&server)
        .send_media(RECIPIENT, WhatsAppMediaKind::Image, &src, None)
        .await.unwrap();
    assert_eq!(id.as_str(), "wamid.img555");
}

#[tokio::test]
async fn send_media_document_with_caption() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(ENDPOINT))
        .and(body_partial_json(serde_json::json!({
            "to": RECIPIENT, "type": "document",
            "document": {"id": "media_pre_uploaded_42", "caption": "see attached invoice"}
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(ok_response("wamid.doc777")))
        .expect(1).mount(&server).await;
    let src = WhatsAppMediaSource::Id("media_pre_uploaded_42".into());
    let id = client(&server)
        .send_media(RECIPIENT, WhatsAppMediaKind::Document, &src, Some("see attached invoice"))
        .await.unwrap();
    assert_eq!(id.as_str(), "wamid.doc777");
}

#[tokio::test]
async fn rate_limited_429_retries_with_retry_after_honored() {
    let server = MockServer::start().await;
    Mock::given(method("POST")).and(path(ENDPOINT))
        .respond_with(ResponseTemplate::new(429)
            .insert_header("retry-after", "0").set_body_string("rate-limited"))
        .up_to_n_times(1).expect(1).mount(&server).await;
    Mock::given(method("POST")).and(path(ENDPOINT))
        .respond_with(ResponseTemplate::new(200).set_body_json(ok_response("wamid.retried")))
        .expect(1).mount(&server).await;
    let id = client(&server).send_text(RECIPIENT, "after backoff").await.unwrap();
    assert_eq!(id.as_str(), "wamid.retried");
}

#[tokio::test]
async fn unauthorized_401_bubbles_as_auth_failed() {
    let server = MockServer::start().await;
    Mock::given(method("POST")).and(path(ENDPOINT))
        .respond_with(ResponseTemplate::new(401)
            .set_body_json(err_response(190, "Invalid OAuth access token.")))
        .expect(1).mount(&server).await;
    let err = client(&server).send_text(RECIPIENT, "noauth").await.unwrap_err();
    match err {
        WhatsAppCloudError::AuthFailed(msg) => assert!(msg.contains("OAuth"), "got {msg}"),
        other => panic!("expected AuthFailed, got {other:?}"),
    }
}

#[tokio::test]
async fn template_not_approved_surfaces_typed_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST")).and(path(ENDPOINT))
        .respond_with(ResponseTemplate::new(400)
            .set_body_json(err_response(132001, "Template not approved")))
        .expect(1).mount(&server).await;
    let err = client(&server)
        .send_template(RECIPIENT, "missing_tmpl", "en_US", &[])
        .await.unwrap_err();
    assert!(matches!(err, WhatsAppCloudError::TemplateNotApproved(_)));
}

#[tokio::test]
async fn outside_24h_window_surfaces_typed_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST")).and(path(ENDPOINT))
        .respond_with(ResponseTemplate::new(400)
            .set_body_json(err_response(131047, "Outside 24h messaging window")))
        .expect(1).mount(&server).await;
    let err = client(&server).send_text(RECIPIENT, "stale").await.unwrap_err();
    assert!(matches!(err, WhatsAppCloudError::OutsideMessagingWindow(_)));
}
