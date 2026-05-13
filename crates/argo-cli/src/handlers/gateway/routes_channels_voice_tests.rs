use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use sha1::{Digest, Sha1};

use super::*;

#[test]
fn predicate_recognises_twilio_payload() {
    assert!(is_voice_webhook("POST /twilio/voice/decision HTTP/1.1\r\n\r\nDigits=1"));
    assert!(is_voice_webhook("POST /twilio/voice/decision?vt=abc HTTP/1.1\r\n\r\nDigits=1"));
}

#[test]
fn predicate_rejects_slack_payload() {
    assert!(!is_voice_webhook("POST /slack/actions HTTP/1.1\r\n\r\npayload={}"));
}

#[test]
fn signature_valid_passes() {
    let url = "https://gw.example.com/twilio/voice/decision?vt=abc";
    let params = vec![("Digits".to_string(), "1".to_string())];
    let sig = twilio_signature(url, &params, "auth");
    assert!(super::super::routes_voice_auth::verify_twilio_signature(url, &params, &sig, "auth"));
}

#[test]
fn signature_invalid_rejects() {
    let url = "https://gw.example.com/twilio/voice/decision?vt=abc";
    let params = vec![("Digits".to_string(), "1".to_string())];
    let forged = vec![("Digits".to_string(), "2".to_string())];
    let sig = twilio_signature(url, &params, "auth");
    assert!(!super::super::routes_voice_auth::verify_twilio_signature(url, &forged, &sig, "auth"));
}

#[tokio::test]
async fn payload_routes_to_existing_voice_handler() {
    std::env::set_var("TWILIO_AUTH_TOKEN", "auth");
    let req = "POST /twilio/voice/decision HTTP/1.1\r\nHost: localhost\r\n\r\nDigits=1";
    let (status, body) = super::super::routes_channels::email_webhook_response(req).await;
    assert_eq!(status, "401 Unauthorized");
    assert_eq!(body, "{\"error\":\"missing X-Twilio-Signature\"}");
    std::env::remove_var("TWILIO_AUTH_TOKEN");
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
