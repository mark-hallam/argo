//! Twilio WhatsApp approval webhook route.
//!
//! Sibling of [`super::routes_channels`]. Owns Twilio signature
//! verification, form parsing, and free-form reply keyword parsing.

use super::routes_channels::{
    bad_request, clean_token, constant_time_eq, header_value, hmac_secret, percent_decode,
    server_error, settle, unauthorized, verify_token, Decision,
};

pub(super) fn is_whatsapp_webhook(req: &str) -> bool {
    req.starts_with("POST /twilio/whatsapp/decision ")
        || req.starts_with("POST /twilio/whatsapp/decision?")
}

pub(super) async fn whatsapp_webhook_response(req: &str) -> (&'static str, String) {
    let auth_token = match twilio_auth_token() {
        Ok(s) => s,
        Err(e) => return server_error(&e),
    };
    let Some(sig) = header_value(req, "X-Twilio-Signature") else {
        return unauthorized("missing X-Twilio-Signature");
    };
    let params = form_params(super::body_from_request(req));
    let url = public_url_for_signature(req);
    if !verify_twilio_signature(&url, &params, sig, &auth_token) {
        return unauthorized("invalid X-Twilio-Signature");
    }
    let secret = match hmac_secret() {
        Ok(s) => s,
        Err(e) => return server_error(&e),
    };
    let Some(body) = form_param(&params, "Body") else {
        return bad_request("missing WhatsApp Body");
    };
    let Some(reply_decision) = decision_from_reply(&body) else {
        return bad_request("reply must start with yes/y/approve or no/n/reject");
    };
    let Some(token) = approval_token_from_reply(&body) else {
        return bad_request("reply missing approval token");
    };
    let claims = match verify_token(&token, &secret) {
        Ok(c) => c,
        Err(e) => return unauthorized(&e),
    };
    if claims.decision != reply_decision {
        return unauthorized("reply keyword does not match approval token decision");
    }
    let actor = form_param(&params, "From")
        .or_else(|| form_param(&params, "WaId").map(|id| format!("whatsapp:{id}")))
        .unwrap_or_else(|| "whatsapp-operator".into());
    match settle(&claims, &actor, Some(&body), "whatsapp").await {
        Ok(v) => ("200 OK", v),
        Err(e) => server_error(&e.to_string()),
    }
}

fn twilio_auth_token() -> Result<String, String> {
    argo_config::cred::get("TWILIO_AUTH_TOKEN")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| "missing TWILIO_AUTH_TOKEN".to_string())
}

fn decision_from_reply(body: &str) -> Option<Decision> {
    let first = body
        .split_whitespace()
        .next()
        .unwrap_or("")
        .trim_matches(|c: char| !c.is_alphanumeric() && c != '\u{2713}' && c != '\u{2717}')
        .to_ascii_lowercase();
    match first.as_str() {
        "yes" | "y" | "approve" | "approved" => Some(Decision::Approve),
        "no" | "n" | "deny" | "denied" | "reject" | "rejected" => Some(Decision::Reject),
        "\u{2713}" => Some(Decision::Approve),
        "\u{2717}" => Some(Decision::Reject),
        _ => None,
    }
}

fn approval_token_from_reply(body: &str) -> Option<String> {
    for marker in ["approval_token=", "token="] {
        if let Some((_, rest)) = body.split_once(marker) {
            let token = clean_token(rest);
            if !token.is_empty() {
                return Some(token);
            }
        }
    }
    body.split_whitespace()
        .skip(1)
        .map(clean_token)
        .find(|token| token.split('.').count() == 4)
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
    let target = request_target(req).unwrap_or("/twilio/whatsapp/decision");
    if let Ok(base) = std::env::var("ARGO_TWILIO_WHATSAPP_DECISION_URL") {
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
    let expected = base64_encode(&hmac_sha1(data.as_bytes(), auth_token.as_bytes()));
    constant_time_eq(expected.as_bytes(), signature.trim().as_bytes())
}

fn hmac_sha1(data: &[u8], secret: &[u8]) -> [u8; 20] {
    const BLOCK: usize = 64;
    let mut key = [0_u8; BLOCK];
    if secret.len() > BLOCK {
        key[..20].copy_from_slice(&sha1_digest(secret));
    } else {
        key[..secret.len()].copy_from_slice(secret);
    }
    let mut ipad = [0x36_u8; BLOCK];
    let mut opad = [0x5c_u8; BLOCK];
    for i in 0..BLOCK {
        ipad[i] ^= key[i];
        opad[i] ^= key[i];
    }
    let mut inner = Vec::with_capacity(BLOCK + data.len());
    inner.extend_from_slice(&ipad);
    inner.extend_from_slice(data);
    let inner = sha1_digest(&inner);
    let mut outer = Vec::with_capacity(BLOCK + inner.len());
    outer.extend_from_slice(&opad);
    outer.extend_from_slice(&inner);
    sha1_digest(&outer)
}

fn sha1_digest(data: &[u8]) -> [u8; 20] {
    let mut h0 = 0x6745_2301_u32;
    let mut h1 = 0xEFCD_AB89_u32;
    let mut h2 = 0x98BA_DCFE_u32;
    let mut h3 = 0x1032_5476_u32;
    let mut h4 = 0xC3D2_E1F0_u32;
    let bit_len = (data.len() as u64) * 8;
    let mut msg = data.to_vec();
    msg.push(0x80);
    while (msg.len() % 64) != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());
    for chunk in msg.chunks_exact(64) {
        let mut w = [0_u32; 80];
        for (slot, bytes) in w.iter_mut().take(16).zip(chunk.chunks_exact(4)) {
            *slot = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }
        let (mut a, mut b, mut c, mut d, mut e) = (h0, h1, h2, h3, h4);
        for (i, &wi) in w.iter().enumerate() {
            let (f, k) = match i {
                0..=19 => ((b & c) | ((!b) & d), 0x5A82_7999),
                20..=39 => (b ^ c ^ d, 0x6ED9_EBA1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1B_BCDC),
                _ => (b ^ c ^ d, 0xCA62_C1D6),
            };
            let temp = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(wi);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = temp;
        }
        h0 = h0.wrapping_add(a);
        h1 = h1.wrapping_add(b);
        h2 = h2.wrapping_add(c);
        h3 = h3.wrapping_add(d);
        h4 = h4.wrapping_add(e);
    }
    let mut out = [0_u8; 20];
    for (slot, word) in out.chunks_exact_mut(4).zip([h0, h1, h2, h3, h4]) {
        slot.copy_from_slice(&word.to_be_bytes());
    }
    out
}

fn base64_encode(data: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0];
        let b1 = chunk.get(1).copied().unwrap_or(0);
        let b2 = chunk.get(2).copied().unwrap_or(0);
        out.push(TABLE[(b0 >> 2) as usize] as char);
        out.push(TABLE[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);
        out.push(if chunk.len() > 1 {
            TABLE[(((b1 & 0x0F) << 2) | (b2 >> 6)) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[(b2 & 0x3F) as usize] as char
        } else {
            '='
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reply_keywords_map_to_decisions() {
        assert_eq!(
            decision_from_reply("yes act.approve.999.sig"),
            Some(Decision::Approve)
        );
        assert_eq!(
            decision_from_reply("Y act.approve.999.sig"),
            Some(Decision::Approve)
        );
        assert_eq!(
            decision_from_reply("\u{2713} act.approve.999.sig"),
            Some(Decision::Approve)
        );
        assert_eq!(
            decision_from_reply("no act.reject.999.sig"),
            Some(Decision::Reject)
        );
        assert_eq!(
            decision_from_reply("n act.reject.999.sig"),
            Some(Decision::Reject)
        );
        assert_eq!(
            decision_from_reply("\u{2717} act.reject.999.sig"),
            Some(Decision::Reject)
        );
        assert_eq!(decision_from_reply("maybe act.reject.999.sig"), None);
    }

    #[test]
    fn reply_token_extractor_accepts_marker_or_second_word() {
        assert_eq!(
            approval_token_from_reply("yes act.approve.999.sig"),
            Some("act.approve.999.sig".into())
        );
        assert_eq!(
            approval_token_from_reply("yes approval_token=act.approve.999.sig"),
            Some("act.approve.999.sig".into())
        );
    }

    #[test]
    fn twilio_signature_accepts_valid_and_rejects_forged() {
        let url = "https://gw.example.com/twilio/whatsapp/decision";
        let params = vec![
            ("Body".to_string(), "yes act.approve.999.sig".to_string()),
            ("From".to_string(), "whatsapp:+15555550199".to_string()),
        ];
        let sig = base64_encode(&hmac_sha1(
            format!("{url}Bodyyes act.approve.999.sigFromwhatsapp:+15555550199").as_bytes(),
            b"auth",
        ));
        assert!(verify_twilio_signature(url, &params, &sig, "auth"));
        let forged = vec![("Body".to_string(), "no act.reject.999.sig".to_string())];
        assert!(!verify_twilio_signature(url, &forged, &sig, "auth"));
    }
}
