mod common;

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use sha1::{Digest, Sha1};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::process::{Command, Stdio};

#[test]
fn voice_webhook_valid_twilio_payload_keeps_legacy_response_shape() {
    let _lock = common::gateway_lock_guard();
    let port = common::pick_free_port();
    let home = tempfile::tempdir().unwrap().keep();
    let mut child = Command::new(common::argo_bin())
        .env("ARGO_HOME", home)
        .env("TWILIO_AUTH_TOKEN", "auth")
        .env("ARGO_CHANNEL_HMAC_SECRET", "secret")
        .args([
            "gateway",
            "start",
            "--port",
            &port.to_string(),
            "--max-requests",
            "1",
            "--skip-startup-check",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn argo gateway start");
    let stderr = child.stderr.take().expect("stderr piped");
    let _drain = common::listener_signal::await_listener_signal(stderr)
        .expect("gateway listener signal");

    let body = "Digits=1&From=%2B15551234567";
    let url = "http://localhost/twilio/voice/decision";
    let params = vec![
        ("Digits".to_string(), "1".to_string()),
        ("From".to_string(), "+15551234567".to_string()),
    ];
    let sig = twilio_signature(url, &params, "auth");
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    let req = format!(
        "POST /twilio/voice/decision HTTP/1.1\r\nHost: localhost\r\nX-Twilio-Signature: {sig}\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(req.as_bytes()).expect("write request");
    let mut resp = String::new();
    stream.read_to_string(&mut resp).expect("read response");

    assert!(resp.starts_with("HTTP/1.1 400 Bad Request"), "response: {resp}");
    assert_eq!(
        common::body_of(&resp),
        "{\"error\":\"missing voice callback token\"}"
    );
    assert!(child.wait().expect("wait gateway").success());
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
