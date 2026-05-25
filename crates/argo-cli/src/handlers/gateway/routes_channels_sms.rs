//! Twilio SMS approval webhook route.
//!
//! Operators reply with `yes <approval-token>` or `no <approval-token>`;
//! the signed token still determines the approval id and intended
//! decision, and the reply keyword must match it.

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use sha1::{Digest, Sha1};

use super::routes_channels::{
    bad_request, clean_token, constant_time_eq, header_value, hmac_secret, percent_decode,
    server_error, settle, unauthorized, verify_token, Decision, TokenClaims,
};

struct SmsReply {
    claims: TokenClaims,
    actor: String,
    body: String,
}

#[derive(Debug)]
enum SmsRouteError {
    BadRequest(String),
    Unauthorized(String),
}

pub(super) fn is_sms_webhook(req: &str) -> bool {
    req.starts_with("POST /twilio/sms/decision ") || req.starts_with("POST /twilio/sms/decision?")
}

pub(super) async fn sms_webhook_response(req: &str) -> (&'static str, String) {
    let auth_token = match twilio_auth_token() {
        Ok(s) => s,
        Err(e) => return server_error(&e),
    };
    let params = match verified_form_params(req, &auth_token) {
        Ok(params) => params,
        Err(e) => return e.response(),
    };
    let secret = match hmac_secret() {
        Ok(s) => s,
        Err(e) => return server_error(&e),
    };
    let reply = match parse_sms_reply(&params, &secret) {
        Ok(reply) => reply,
        Err(e) => return e.response(),
    };
    match settle(&reply.claims, &reply.actor, Some(&reply.body), "sms").await {
        Ok(v) => ("200 OK", v),
        Err(e) => server_error(&e.to_string()),
    }
}

impl SmsRouteError {
    fn bad_request(msg: impl Into<String>) -> Self {
        Self::BadRequest(msg.into())
    }

    fn unauthorized(msg: impl Into<String>) -> Self {
        Self::Unauthorized(msg.into())
    }

    fn response(self) -> (&'static str, String) {
        match self {
            Self::BadRequest(msg) => bad_request(&msg),
            Self::Unauthorized(msg) => unauthorized(&msg),
        }
    }
}

fn verified_form_params(
    req: &str,
    auth_token: &str,
) -> Result<Vec<(String, String)>, SmsRouteError> {
    let Some(sig) = header_value(req, "X-Twilio-Signature") else {
        return Err(SmsRouteError::unauthorized("missing X-Twilio-Signature"));
    };
    let params = form_params(super::body_from_request(req));
    let url = public_url_for_signature(req);
    if !verify_twilio_signature(&url, &params, sig, auth_token) {
        return Err(SmsRouteError::unauthorized("invalid X-Twilio-Signature"));
    }
    Ok(params)
}

fn parse_sms_reply(params: &[(String, String)], secret: &str) -> Result<SmsReply, SmsRouteError> {
    let Some(body) = form_param(params, "Body") else {
        return Err(SmsRouteError::bad_request("missing SMS Body"));
    };
    let Some(reply_decision) = decision_from_reply(&body) else {
        return Err(SmsRouteError::bad_request(
            "reply must start with yes/y/approve or no/n/reject/deny",
        ));
    };
    let Some(token) = approval_token_from_reply(&body) else {
        return Err(SmsRouteError::bad_request("reply missing approval token"));
    };
    let claims = verify_token(&token, secret).map_err(SmsRouteError::unauthorized)?;
    if claims.decision != reply_decision {
        return Err(SmsRouteError::unauthorized(
            "reply keyword does not match approval token decision",
        ));
    }
    let actor = form_param(params, "From")
        .map(|from| format!("sms:{from}"))
        .unwrap_or_else(|| "sms-operator".into());
    Ok(SmsReply {
        claims,
        actor,
        body,
    })
}

fn twilio_auth_token() -> Result<String, String> {
    // tenant-unscoped: gateway process-level secret (no per-request tenant context for this slot)
    argo_config::cred::get_unscoped("TWILIO_AUTH_TOKEN")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| "missing TWILIO_AUTH_TOKEN".to_string())
}

fn decision_from_reply(body: &str) -> Option<Decision> {
    let first = body
        .split_whitespace()
        .next()
        .unwrap_or("")
        .trim_matches(|c: char| !c.is_alphanumeric())
        .to_ascii_lowercase();
    match first.as_str() {
        "yes" | "y" | "approve" => Some(Decision::Approve),
        "no" | "n" | "reject" | "deny" => Some(Decision::Reject),
        _ => None,
    }
}

fn approval_token_from_reply(body: &str) -> Option<String> {
    for marker in ["approval_token=", "token="] {
        if let Some((_, rest)) = body.split_once(marker) {
            let token = clean_reply_token(rest);
            if !token.is_empty() {
                return Some(token);
            }
        }
    }
    body.split_whitespace()
        .skip(1)
        .map(clean_reply_token)
        .find(|token| token.split('.').count() == 4)
}

fn clean_reply_token(s: &str) -> String {
    clean_token(s)
        .split('&')
        .next()
        .unwrap_or("")
        .trim_end_matches([',', ';', '.'])
        .to_string()
}

fn form_params(body: &str) -> Vec<(String, String)> {
    body.split('&')
        .filter(|part| !part.is_empty())
        .filter_map(|kv| {
            let (k, v) = kv.split_once('=')?;
            Some((percent_decode(k)?, percent_decode(v)?))
        })
        .collect()
}

fn form_param(params: &[(String, String)], key: &str) -> Option<String> {
    params
        .iter()
        .find_map(|(k, v)| (k == key).then(|| v.clone()))
}

fn public_url_for_signature(req: &str) -> String {
    let target = request_target(req).unwrap_or("/twilio/sms/decision");
    if let Ok(base) = std::env::var("ARGO_TWILIO_SMS_DECISION_URL") {
        if !base.is_empty() {
            return match target.split_once('?') {
                Some((_, q)) if !q.is_empty() => {
                    let sep = if base.contains('?') { '&' } else { '?' };
                    format!("{base}{sep}{q}")
                }
                _ => base,
            };
        }
    }
    let proto = header_value(req, "X-Forwarded-Proto").unwrap_or("http");
    let host = header_value(req, "X-Forwarded-Host")
        .or_else(|| header_value(req, "Host"))
        .unwrap_or("localhost");
    format!("{proto}://{host}{target}")
}

fn request_target(req: &str) -> Option<&str> {
    req.lines().next()?.split_whitespace().nth(1)
}

fn verify_twilio_signature(
    url: &str,
    params: &[(String, String)],
    signature: &str,
    auth_token: &str,
) -> bool {
    let mut sorted = params.to_vec();
    sorted.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    let mut data = url.to_string();
    for (k, v) in sorted {
        data.push_str(&k);
        data.push_str(&v);
    }
    let expected = STANDARD.encode(hmac_sha1(data.as_bytes(), auth_token.as_bytes()));
    constant_time_eq(expected.as_bytes(), signature.trim().as_bytes())
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

#[cfg(test)]
mod tests {
    use super::super::routes_channels::hmac_hex;
    use super::*;

    const AUTH: &str = "twilio-auth";
    const HMAC: &str = "channel-secret";

    #[test]
    fn predicate_recognises_sms_twilio_route() {
        assert!(is_sms_webhook(
            "POST /twilio/sms/decision HTTP/1.1\r\n\r\nBody=yes"
        ));
        assert!(is_sms_webhook(
            "POST /twilio/sms/decision?tenant=acme HTTP/1.1\r\n\r\nBody=yes"
        ));
        assert!(!is_sms_webhook(
            "POST /channels/sms/webhook HTTP/1.1\r\n\r\nBody=yes"
        ));
    }

    #[test]
    fn valid_approve_reply_parses_signed_token() {
        let token = approval_token("act-sms-1", Decision::Approve);
        let req = signed_request(&format!("Body=yes+{token}&From=%2B15555550199"));
        let params = verified_form_params(&req, AUTH).expect("signature verifies");
        let reply = parse_sms_reply(&params, HMAC).expect("reply parses");

        assert_eq!(reply.claims.approval_id, "act-sms-1");
        assert_eq!(reply.claims.decision, Decision::Approve);
        assert_eq!(reply.actor, "sms:+15555550199");
    }

    #[test]
    fn valid_reject_reply_parses_token_marker() {
        let token = approval_token("act-sms-2", Decision::Reject);
        let req = signed_request(&format!(
            "Body=no+approval_token%3D{token}&From=%2B15555550199"
        ));
        let params = verified_form_params(&req, AUTH).expect("signature verifies");
        let reply = parse_sms_reply(&params, HMAC).expect("reply parses");

        assert_eq!(reply.claims.approval_id, "act-sms-2");
        assert_eq!(reply.claims.decision, Decision::Reject);
    }

    #[test]
    fn bad_signature_rejects_before_reply_validation() {
        let req = "POST /twilio/sms/decision HTTP/1.1\r\nHost: gw.example.com\r\n\
                   X-Forwarded-Proto: https\r\nX-Twilio-Signature: forged\r\n\r\n\
                   Body=yes";
        let err = verified_form_params(req, AUTH).expect_err("signature rejected");

        assert_eq!(err.response(), unauthorized("invalid X-Twilio-Signature"));
    }

    #[test]
    fn missing_token_is_bad_request() {
        let req = signed_request("Body=yes&From=%2B15555550199");
        let params = verified_form_params(&req, AUTH).expect("signature verifies");
        let err = reply_err(&params, "token required");

        assert_eq!(err.response(), bad_request("reply missing approval token"));
    }

    #[test]
    fn mismatched_reply_and_token_decision_is_unauthorized() {
        let token = approval_token("act-sms-3", Decision::Reject);
        let req = signed_request(&format!("Body=yes+token%3D{token}"));
        let params = verified_form_params(&req, AUTH).expect("signature verifies");
        let err = reply_err(&params, "decision mismatch");

        assert_eq!(
            err.response(),
            unauthorized("reply keyword does not match approval token decision")
        );
    }

    #[test]
    fn unsupported_reply_is_bad_request() {
        let token = approval_token("act-sms-4", Decision::Approve);
        let req = signed_request(&format!("Body=maybe+{token}"));
        let params = verified_form_params(&req, AUTH).expect("signature verifies");
        let err = reply_err(&params, "unsupported reply");

        assert_eq!(
            err.response(),
            bad_request("reply must start with yes/y/approve or no/n/reject/deny")
        );
    }

    fn signed_request(body: &str) -> String {
        let url = "https://gw.example.com/twilio/sms/decision";
        let params = form_params(body);
        let sig = twilio_signature(url, &params, AUTH);
        format!(
            "POST /twilio/sms/decision HTTP/1.1\r\nHost: gw.example.com\r\n\
             X-Forwarded-Proto: https\r\nX-Twilio-Signature: {sig}\r\n\r\n{body}"
        )
    }

    fn approval_token(approval_id: &str, decision: Decision) -> String {
        let decision = match decision {
            Decision::Approve => "approve",
            Decision::Reject => "reject",
        };
        let exp = chrono::Utc::now().timestamp() + 3600;
        let sig = hmac_hex(
            format!("{approval_id}|{decision}|{exp}").as_bytes(),
            HMAC.as_bytes(),
        );
        format!("{approval_id}.{decision}.{exp}.{sig}")
    }

    fn reply_err(params: &[(String, String)], label: &str) -> SmsRouteError {
        match parse_sms_reply(params, HMAC) {
            Ok(_) => panic!("{label}: expected parse error"),
            Err(err) => err,
        }
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
}
