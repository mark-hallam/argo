use crate::handlers::worker_actions::ActionRecord;
use crate::handlers::worker_actions_transitions::open_workspace_memory_with_store;
use argo_memory::MemoryFile;
use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use serde_json::Value;
use sha1::{Digest, Sha1};
use std::path::PathBuf;
use tokio::sync::{Mutex, MutexGuard};

use super::*;

const HMAC_SECRET: &str = "voice-test-hmac";
const TWILIO_AUTH: &str = "auth";
const VOICE_URL: &str = "https://gw.example.com/twilio/voice/decision";

async fn env_lock() -> MutexGuard<'static, ()> {
    static M: Mutex<()> = Mutex::const_new(());
    M.lock().await
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf()
}

fn set_test_env(tmp: &std::path::Path) {
    std::env::set_var("ARGO_HOME", tmp.join("home"));
    std::fs::create_dir_all(tmp.join("home")).expect("home dir");
    std::env::set_var("ARGO_VERTICALS_DIR", workspace_root().join("verticals"));
    std::env::set_var("ARGO_CHANNEL_HMAC_SECRET", HMAC_SECRET);
    std::env::set_var("TWILIO_AUTH_TOKEN", TWILIO_AUTH);
    std::env::set_var("ARGO_TWILIO_VOICE_DECISION_URL", VOICE_URL);
}

fn clear_test_env() {
    std::env::remove_var("ARGO_HOME");
    std::env::remove_var("ARGO_VERTICALS_DIR");
    std::env::remove_var("ARGO_CHANNEL_HMAC_SECRET");
    std::env::remove_var("TWILIO_AUTH_TOKEN");
    std::env::remove_var("ARGO_TWILIO_VOICE_DECISION_URL");
}

#[test]
fn predicate_recognises_twilio_payload() {
    assert!(is_voice_webhook(
        "POST /twilio/voice/decision HTTP/1.1\r\n\r\nDigits=1"
    ));
    assert!(is_voice_webhook(
        "POST /twilio/voice/decision?vt=abc HTTP/1.1\r\n\r\nDigits=1"
    ));
}

#[test]
fn predicate_rejects_slack_payload() {
    assert!(!is_voice_webhook(
        "POST /slack/actions HTTP/1.1\r\n\r\npayload={}"
    ));
}

#[test]
fn signature_valid_passes() {
    let url = "https://gw.example.com/twilio/voice/decision?vt=abc";
    let params = vec![("Digits".to_string(), "1".to_string())];
    let sig = twilio_signature(url, &params, "auth");
    assert!(super::super::routes_voice_auth::verify_twilio_signature(
        url, &params, &sig, "auth"
    ));
}

#[test]
fn signature_invalid_rejects() {
    let url = "https://gw.example.com/twilio/voice/decision?vt=abc";
    let params = vec![("Digits".to_string(), "1".to_string())];
    let forged = vec![("Digits".to_string(), "2".to_string())];
    let sig = twilio_signature(url, &params, "auth");
    assert!(!super::super::routes_voice_auth::verify_twilio_signature(
        url, &forged, &sig, "auth"
    ));
}

#[tokio::test]
async fn payload_routes_to_existing_voice_handler() {
    let _g = env_lock().await;
    std::env::set_var("TWILIO_AUTH_TOKEN", TWILIO_AUTH);
    let req = "POST /twilio/voice/decision HTTP/1.1\r\nHost: localhost\r\n\r\nDigits=1";
    let (status, body) = super::super::routes_channels::email_webhook_response(req).await;
    assert_eq!(status, "401 Unauthorized");
    assert_eq!(body, "{\"error\":\"missing X-Twilio-Signature\"}");
    clear_test_env();
}

#[tokio::test]
async fn voice_approve_callback_emits_approval_replied_audit_row() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    seed_pending("voice-approve").await;
    let token = voice_callback_token("voice-approve");
    let req = voice_request(&token, "1", "+15550100");
    let (status, body) = voice_webhook_response(&req).await;
    assert_eq!(status, "200 OK", "body={body}");
    let v = parse(&body);
    assert_eq!(v["decision"], "approve");
    assert_eq!(v["state"], "approved");
    let payloads = approval_replied_payloads().await;
    assert_eq!(payloads.len(), 1);
    assert_voice_payload(&payloads[0], "voice-approve", "approve", "1");
    clear_test_env();
}

#[tokio::test]
async fn voice_deny_callback_emits_approval_replied_audit_row() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    seed_pending("voice-deny").await;
    let token = voice_callback_token("voice-deny");
    let req = voice_request(&token, "2", "+15550100");
    let (status, body) = voice_webhook_response(&req).await;
    assert_eq!(status, "200 OK", "body={body}");
    let v = parse(&body);
    assert_eq!(v["decision"], "deny");
    assert_eq!(v["state"], "blocked");
    let payloads = approval_replied_payloads().await;
    assert_eq!(payloads.len(), 1);
    assert_voice_payload(&payloads[0], "voice-deny", "deny", "2");
    clear_test_env();
}

#[tokio::test]
async fn duplicate_voice_callback_does_not_emit_second_audit_row() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    seed_pending("voice-dupe").await;
    let token = voice_callback_token("voice-dupe");
    let req = voice_request(&token, "1", "+15550100");
    let (status, body) = voice_webhook_response(&req).await;
    assert_eq!(status, "200 OK", "body={body}");
    let (status, body) = voice_webhook_response(&req).await;
    assert_eq!(status, "200 OK", "body={body}");
    let v = parse(&body);
    assert_eq!(v["already_resolved"], true);
    let payloads = approval_replied_payloads().await;
    assert_eq!(payloads.len(), 1, "late DTMF must not duplicate audit");
    clear_test_env();
}

fn pending_record(id: &str) -> ActionRecord {
    ActionRecord {
        schema_key: ActionRecord::SCHEMA_KEY.into(),
        action_id: id.into(),
        job_id: "job-voice".into(),
        worker_id: argo_identity::WorkerId::nil(),
        item_id: None,
        action_type: "call_operator".into(),
        risk_level: "high".into(),
        requires_approval: true,
        state: "pending".into(),
        payload: serde_json::json!({"summary": format!("voice fixture {id}")}),
        approved_by: None,
        approved_at: None,
        executed_at: None,
        platform: None,
        result: None,
        evidence: Vec::new(),
        blocked_reason: None,
        created_at: chrono::Utc::now().to_rfc3339(),
    }
}

async fn seed_pending(action_id: &str) {
    let (memory, ws, _store) = open_workspace_memory_with_store()
        .await
        .expect("open memory");
    memory
        .append_for_vertical(
            ws,
            MemoryFile::Memory,
            serde_json::to_string(&pending_record(action_id)).expect("serialise"),
            None,
            "gtm-engineer",
        )
        .await
        .expect("append memory row");
}

async fn approval_replied_payloads() -> Vec<Value> {
    let (_memory, _ws, store) = open_workspace_memory_with_store()
        .await
        .expect("open store");
    let rows: Vec<String> = sqlx::query_scalar(
        "SELECT payload_json FROM audit_record WHERE kind = 'approval.replied' ORDER BY seq ASC",
    )
    .fetch_all(store.pool())
    .await
    .expect("select audit rows");
    rows.into_iter()
        .map(|s| serde_json::from_str(&s).expect("payload json"))
        .collect()
}

fn assert_voice_payload(payload: &Value, action_id: &str, kind: &str, digit: &str) {
    assert_eq!(payload["action_id"], action_id);
    assert_eq!(payload["channel"], "voice");
    assert_eq!(payload["replier"], "+15550100");
    assert_eq!(payload["reply_kind"], kind);
    assert_eq!(payload["reply_body"], digit);
    assert_eq!(payload["reply_digit"], digit);
    assert!(payload["replied_at"].as_str().is_some());
}

fn parse(body: &str) -> Value {
    serde_json::from_str(body).expect("response json")
}

fn voice_request(token: &str, digit: &str, caller: &str) -> String {
    let url = format!("{VOICE_URL}?vt={token}");
    let params = vec![
        ("Digits".to_string(), digit.to_string()),
        ("From".to_string(), caller.to_string()),
    ];
    let sig = twilio_signature(&url, &params, TWILIO_AUTH);
    let encoded_caller = caller.replace('+', "%2B");
    let body = format!("Digits={digit}&From={encoded_caller}");
    format!(
        "POST /twilio/voice/decision?vt={token} HTTP/1.1\r\nHost: gw.example.com\r\nX-Twilio-Signature: {sig}\r\nContent-Length: {len}\r\n\r\n{body}",
        len = body.len()
    )
}

fn voice_callback_token(action_id: &str) -> String {
    let exp = chrono::Utc::now().timestamp() + 300;
    let payload = format!("{action_id}|{action_id}|operator-1|{exp}|nonce-1");
    let sig = super::super::routes_channels::hmac_hex(payload.as_bytes(), HMAC_SECRET.as_bytes());
    format!("{}.{}", hex::encode(payload.as_bytes()), sig)
}

fn twilio_signature(url: &str, params: &[(String, String)], auth_token: &str) -> String {
    let mut sorted = params.to_vec();
    sorted.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    let mut data = url.to_string();
    for (k, v) in sorted {
        data.push_str(&k);
        data.push_str(&v);
    }
    STANDARD.encode(hmac_sha1(data.as_bytes(), auth_token.as_bytes()))
}

fn hmac_sha1(data: &[u8], secret: &[u8]) -> [u8; 20] {
    const BLOCK: usize = 64;
    let mut key = [0_u8; BLOCK];
    if secret.len() > BLOCK {
        key[..20].copy_from_slice(&Sha1::digest(secret));
    } else {
        key[..secret.len()].copy_from_slice(secret);
    }
    let mut ipad = [0x36_u8; BLOCK];
    let mut opad = [0x5c_u8; BLOCK];
    for i in 0..BLOCK {
        ipad[i] ^= key[i];
        opad[i] ^= key[i];
    }
    let mut inner = Sha1::new();
    inner.update(ipad);
    inner.update(data);
    let inner = inner.finalize();
    let mut outer = Sha1::new();
    outer.update(opad);
    outer.update(inner);
    outer.finalize().into()
}
