use argo_channels::voice::{
    verify_voice_callback_token, VoiceApprovalChannel, VoiceApprovalChannelConfig,
};
use argo_channels::{
    ChannelId, Notification, NotifyChannel, OperatorTarget, RateLimitConfig, RiskLevel,
};
use chrono::{Duration as ChronoDuration, Utc};
use std::{collections::BTreeMap, time::Duration};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

const SECRET: &str = "voice-roundtrip-secret";
const DECISION_URL: &str = "https://gw.example.com/twilio/voice/decision";

#[tokio::test]
async fn voice_outbound_callback_token_verifies_for_inbound() {
    let (base, request) = twilio_calls_mock().await;
    let mut phones = BTreeMap::new();
    phones.insert("operator".into(), "+15551234567".into());
    let channel = VoiceApprovalChannel::new(VoiceApprovalChannelConfig {
        channel_id: ChannelId::new("voice"),
        account_sid: "AC_roundtrip".into(),
        auth_token: "auth-roundtrip".into(),
        from_number: "+15550000000".into(),
        endpoint: format!("{base}/2010-04-01/Accounts/AC_roundtrip/Calls.json"),
        decision_callback_base_url: DECISION_URL.into(),
        hmac_secret: SECRET.into(),
        operator_phone_numbers: phones,
        rate_limit: RateLimitConfig::unlimited(),
        quiet_hours: None,
    });

    channel.send(notification()).await.unwrap();
    let req = request.await.unwrap();
    assert!(req.starts_with("POST /2010-04-01/Accounts/AC_roundtrip/Calls.json"));
    assert!(req.contains("To=%2B15551234567"));
    let twiml = form_value(&req, "Twiml").expect("Twiml field");
    let token = twiml
        .split_once("vt=")
        .and_then(|(_, tail)| tail.split('"').next())
        .expect("voice callback token");

    let claims = verify_voice_callback_token(token, SECRET).unwrap();
    assert_eq!(claims.operator_id, "operator");
    assert_eq!(claims.action_id, "act-roundtrip-1");
    assert_eq!(claims.approval_id, "appr-roundtrip-1");
    let err = verify_voice_callback_token(token, "wrong-secret").unwrap_err();
    assert!(err.to_string().contains("signature mismatch"));
}

async fn twilio_calls_mock() -> (String, tokio::task::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut buf = vec![0_u8; 8192];
        let n = tokio::time::timeout(Duration::from_secs(5), stream.read(&mut buf))
            .await
            .unwrap()
            .unwrap();
        let req = String::from_utf8_lossy(&buf[..n]).to_string();
        let body = r#"{"sid":"CA789","status":"queued"}"#;
        let resp = format!(
            "HTTP/1.1 201 Created\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        stream.write_all(resp.as_bytes()).await.unwrap();
        req
    });
    (format!("http://{addr}"), handle)
}

fn notification() -> Notification {
    Notification {
        target: OperatorTarget {
            operator_id: "operator".into(),
            email: "operator@example.com".into(),
        },
        action_id: "act-roundtrip-1".into(),
        approval_id: "appr-roundtrip-1".into(),
        risk_level: RiskLevel::High,
        prompt_body: "Approve roundtrip test?".into(),
        approve_url: "https://example.com/approve".into(),
        reject_url: "https://example.com/reject".into(),
        expiry: Utc::now() + ChronoDuration::minutes(5),
    }
}

fn form_value(req: &str, name: &str) -> Option<String> {
    let body = req.split_once("\r\n\r\n").map_or(req, |(_, b)| b);
    body.split('&').find_map(|pair| {
        let (key, val) = pair.split_once('=')?;
        (key == name).then(|| percent_decode(val))
    })
}

fn percent_decode(s: &str) -> String {
    let mut out = Vec::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                if let Ok(hex) = std::str::from_utf8(&bytes[i + 1..i + 3]) {
                    if let Ok(byte) = u8::from_str_radix(hex, 16) {
                        out.push(byte);
                        i += 3;
                        continue;
                    }
                }
                out.push(bytes[i]);
            }
            b'+' => out.push(b' '),
            byte => out.push(byte),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).replace("&amp;", "&")
}
