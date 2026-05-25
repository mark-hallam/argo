//! Discord approval interaction decision route.
//!
//! Sibling of [`super::routes_channels`]. Owns Discord interaction
//! payload parsing and delegates signed-token verification plus settle
//! behavior to the shared channel helpers.

use super::routes_channels::{
    bad_request, header_value, hmac_secret, percent_decode, server_error, settle, unauthorized,
    verify_token,
};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde_json::Value;

pub(super) fn is_discord_decision(req: &str) -> bool {
    req.starts_with("POST /channels/discord/decision ")
        || req.starts_with("POST /channels/discord/decision?")
}

pub(super) async fn discord_decision_response(req: &str) -> (&'static str, String) {
    let body = super::body_from_request(req);
    let Some((sig, ts)) = discord_signature_headers(req) else {
        return unauthorized("missing Discord signature headers");
    };
    let public_key = match discord_public_key() {
        Ok(k) => k,
        Err(e) => return server_error(&e),
    };
    if let Err(e) = verify_discord_signature(body, ts, sig, &public_key) {
        return unauthorized(&e);
    }
    let secret = match hmac_secret() {
        Ok(s) => s,
        Err(e) => return server_error(&e),
    };
    let parsed = match discord_payload(body) {
        Ok(v) => v,
        Err(e) => return bad_request(&e),
    };
    let Some(token) = discord_action_token(&parsed).or_else(|| query_token(req)) else {
        return bad_request("missing Discord approval token");
    };
    let claims = match verify_token(&token, &secret) {
        Ok(c) => c,
        Err(e) => return bad_request(&e),
    };
    match settle(
        &claims,
        &discord_actor(&parsed),
        discord_note(&parsed).as_deref(),
        "discord",
    )
    .await
    {
        Ok(v) => ("200 OK", v),
        Err(e) => server_error(&e.to_string()),
    }
}

fn discord_signature_headers(req: &str) -> Option<(&str, &str)> {
    let sig = header_value(req, "X-Signature-Ed25519")?;
    let ts = header_value(req, "X-Signature-Timestamp")?;
    Some((sig, ts))
}

fn discord_public_key() -> Result<VerifyingKey, String> {
    // tenant-unscoped: Discord interaction public key is a gateway process-level verifier.
    let key = argo_config::cred::get_unscoped("ARGO_DISCORD_PUBLIC_KEY")
        .map_err(|_| "missing ARGO_DISCORD_PUBLIC_KEY".to_string())?;
    let bytes = hex::decode(key.trim()).map_err(|_| "invalid ARGO_DISCORD_PUBLIC_KEY")?;
    let arr: [u8; 32] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| "invalid ARGO_DISCORD_PUBLIC_KEY")?;
    VerifyingKey::from_bytes(&arr).map_err(|_| "invalid ARGO_DISCORD_PUBLIC_KEY".to_string())
}

fn verify_discord_signature(
    body: &str,
    ts: &str,
    sig: &str,
    public_key: &VerifyingKey,
) -> Result<(), String> {
    let bytes = hex::decode(sig.trim()).map_err(|_| "invalid X-Signature-Ed25519")?;
    let arr: [u8; 64] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| "invalid X-Signature-Ed25519")?;
    let signature = Signature::from_bytes(&arr);
    let mut signed = Vec::with_capacity(ts.len() + body.len());
    signed.extend_from_slice(ts.as_bytes());
    signed.extend_from_slice(body.as_bytes());
    public_key
        .verify(&signed, &signature)
        .map_err(|_| "invalid X-Signature-Ed25519".to_string())
}

fn discord_payload(body: &str) -> Result<Value, String> {
    if body.trim().is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_str(body).map_err(|e| format!("parse Discord payload: {e}"))
}

fn discord_action_token(v: &Value) -> Option<String> {
    let raw = v
        .pointer("/data/custom_id")
        .or_else(|| v.get("custom_id"))
        .or_else(|| v.get("token"))?
        .as_str()?;
    Some(strip_custom_id_prefix(raw).to_string())
}

fn query_token(req: &str) -> Option<String> {
    let target = req.lines().next()?.split_whitespace().nth(1)?;
    let query = target.split_once('?')?.1;
    query.split('&').find_map(|kv| {
        let (k, v) = kv.split_once('=')?;
        (percent_decode(k).as_deref() == Some("token"))
            .then(|| percent_decode(v))
            .flatten()
    })
}

fn strip_custom_id_prefix(raw: &str) -> &str {
    raw.strip_prefix("argo_approval:").unwrap_or(raw)
}

fn discord_actor(v: &Value) -> String {
    v.pointer("/member/user/id")
        .or_else(|| v.pointer("/user/id"))
        .or_else(|| v.pointer("/member/nick"))
        .and_then(Value::as_str)
        .unwrap_or("discord-operator")
        .to_string()
}

fn discord_note(v: &Value) -> Option<String> {
    v.pointer("/data/custom_id")
        .and_then(Value::as_str)
        .map(|_| "discord_component".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use std::sync::OnceLock;

    #[test]
    fn discord_payload_extracts_actor_and_token() {
        let parsed = discord_payload(
            r#"{"member":{"user":{"id":"U1"}},"data":{"custom_id":"argo_approval:act.approve.999.sig"}}"#,
        )
        .unwrap();
        assert_eq!(discord_actor(&parsed), "U1");
        assert_eq!(
            discord_action_token(&parsed),
            Some("act.approve.999.sig".into())
        );
    }

    #[test]
    fn query_token_extracts_percent_decoded_token() {
        let req = "POST /channels/discord/decision?token=act.approve.999.%73ig HTTP/1.1\r\n\r\n";
        assert_eq!(query_token(req), Some("act.approve.999.sig".into()));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn tampered_token_returns_bad_request() {
        let _guard = env_lock().await;
        std::env::set_var("ARGO_CHANNEL_HMAC_SECRET", "secret");
        let signing = test_signing_key(1);
        set_discord_public_key(&signing);
        let exp = chrono::Utc::now().timestamp() + 60;
        let body = "";
        let req = signed_request(
            &format!("/channels/discord/decision?token=act.approve.{exp}.bad"),
            body,
            &signing,
        );
        let (status, body) = discord_decision_response(&req).await;
        assert_eq!(status, "400 Bad Request");
        assert!(body.contains("signature mismatch"));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn missing_signature_headers_reject_before_token_secret() {
        let _guard = env_lock().await;
        std::env::remove_var("ARGO_CHANNEL_HMAC_SECRET");
        std::env::remove_var("ARGO_DISCORD_PUBLIC_KEY");
        let req = "POST /channels/discord/decision?token=act.approve.999.sig HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let (status, body) = discord_decision_response(req).await;
        assert_eq!(status, "401 Unauthorized");
        assert!(body.contains("missing Discord signature headers"));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn bad_signature_rejects_before_token_secret() {
        let _guard = env_lock().await;
        std::env::remove_var("ARGO_CHANNEL_HMAC_SECRET");
        let signer = test_signing_key(1);
        let public_key = test_signing_key(2);
        set_discord_public_key(&public_key);
        let body = r#"{"data":{"custom_id":"argo_approval:act.approve.999.sig"}}"#;
        let req = signed_request("/channels/discord/decision", body, &signer);

        let (status, body) = discord_decision_response(&req).await;

        assert_eq!(status, "401 Unauthorized");
        assert!(body.contains("invalid X-Signature-Ed25519"));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn invalid_public_key_fails_closed_and_visible() {
        let _guard = env_lock().await;
        std::env::set_var("ARGO_DISCORD_PUBLIC_KEY", "not-hex");
        let signing = test_signing_key(1);
        let body = r#"{"data":{"custom_id":"argo_approval:act.approve.999.sig"}}"#;
        let req = signed_request("/channels/discord/decision", body, &signing);

        let (status, body) = discord_decision_response(&req).await;

        assert_eq!(status, "500 Internal Server Error");
        assert!(body.contains("invalid ARGO_DISCORD_PUBLIC_KEY"));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn valid_signature_and_token_reach_settle() {
        let _guard = env_lock().await;
        let home = tempfile::tempdir().unwrap();
        std::env::set_var("ARGO_HOME", home.path());
        std::env::set_var("ARGO_CHANNEL_HMAC_SECRET", "secret");
        let signing = test_signing_key(1);
        set_discord_public_key(&signing);
        let token = approval_token("act-discord-signed", "approve", "secret");
        let body = format!(
            r#"{{"member":{{"user":{{"id":"U1"}}}},"data":{{"custom_id":"argo_approval:{token}"}}}}"#
        );
        let req = signed_request("/channels/discord/decision", &body, &signing);

        let (status, body) = discord_decision_response(&req).await;

        assert_eq!(status, "500 Internal Server Error");
        assert!(body.contains("no approval"));
        assert!(body.contains("act-discord-signed"));
        std::env::remove_var("ARGO_HOME");
    }

    async fn env_lock() -> tokio::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
            .lock()
            .await
    }

    fn test_signing_key(seed: u8) -> SigningKey {
        SigningKey::from_bytes(&[seed; 32])
    }

    fn set_discord_public_key(signing: &SigningKey) {
        std::env::set_var(
            "ARGO_DISCORD_PUBLIC_KEY",
            hex::encode(signing.verifying_key().to_bytes()),
        );
    }

    fn signed_request(path: &str, body: &str, signing: &SigningKey) -> String {
        let ts = "1710000000";
        let mut message = Vec::with_capacity(ts.len() + body.len());
        message.extend_from_slice(ts.as_bytes());
        message.extend_from_slice(body.as_bytes());
        let sig = signing.sign(&message);
        format!(
            "POST {path} HTTP/1.1\r\nHost: localhost\r\nX-Signature-Ed25519: {}\r\nX-Signature-Timestamp: {ts}\r\nContent-Length: {}\r\n\r\n{body}",
            hex::encode(sig.to_bytes()),
            body.len()
        )
    }

    fn approval_token(id: &str, decision: &str, secret: &str) -> String {
        let exp = chrono::Utc::now().timestamp() + 60;
        let sig = super::super::routes_channels::hmac_hex(
            format!("{id}|{decision}|{exp}").as_bytes(),
            secret.as_bytes(),
        );
        format!("{id}.{decision}.{exp}.{sig}")
    }
}
