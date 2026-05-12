//! Twilio webhook signature + voice-callback-token verification for the
//! gateway voice route. Split out of `routes_voice.rs`
//! (claude-routes-voice-split) to keep that file under the 500-LOC HARD
//! RULE 6 ceiling. Pure refactor; no behaviour change. All items retain
//! `pub(super)` visibility so existing call sites in `routes_voice.rs`
//! are unchanged.

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use sha2::{Digest, Sha256};

use super::routes_voice::{form_param, header_value, query_value, request_target};

pub(super) struct VoiceTokenClaims {
    pub approval_id: String,
    pub action_id: String,
    pub operator_id: String,
}

pub(super) fn verify_twilio_signature(
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

pub(super) fn public_url_for_signature(req: &str) -> String {
    let target = request_target(req).unwrap_or("/twilio/voice/decision");
    if let Ok(base) = std::env::var("ARGO_TWILIO_VOICE_DECISION_URL") {
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

pub(super) fn voice_token(req: &str, params: &[(String, String)]) -> Option<String> {
    request_target(req)
        .and_then(|target| query_value(target, "vt"))
        .or_else(|| form_param(params, "vt"))
}

pub(super) fn verify_voice_token(token: &str, secret: &str) -> Result<VoiceTokenClaims, String> {
    let (payload_hex, sig) = token
        .rsplit_once('.')
        .ok_or_else(|| "voice callback token must have payload.signature".to_string())?;
    let payload_bytes = hex::decode(payload_hex)
        .map_err(|e| format!("voice callback token payload hex: {e}"))?;
    let payload = std::str::from_utf8(&payload_bytes)
        .map_err(|e| format!("voice callback token payload utf8: {e}"))?;
    let expected = hmac_hex(payload.as_bytes(), secret.as_bytes());
    if !constant_time_eq(expected.as_bytes(), sig.as_bytes()) {
        return Err("voice callback token signature mismatch".into());
    }
    let parts: Vec<&str> = payload.split('|').collect();
    if parts.len() != 5 {
        return Err("voice callback token payload must have 5 fields".into());
    }
    let exp = parts[3]
        .parse::<i64>()
        .map_err(|e| format!("voice callback token exp: {e}"))?;
    if chrono::Utc::now().timestamp() > exp {
        return Err("voice callback token expired".into());
    }
    Ok(VoiceTokenClaims {
        approval_id: parts[0].to_string(),
        action_id: parts[1].to_string(),
        operator_id: parts[2].to_string(),
    })
}

fn hmac_hex(data: &[u8], secret: &[u8]) -> String {
    const BLOCK: usize = 64;
    let mut key = [0_u8; BLOCK];
    if secret.len() > BLOCK {
        let digest = Sha256::digest(secret);
        key[..digest.len()].copy_from_slice(&digest);
    } else {
        key[..secret.len()].copy_from_slice(secret);
    }
    let mut ipad = [0x36_u8; BLOCK];
    let mut opad = [0x5c_u8; BLOCK];
    for i in 0..BLOCK {
        ipad[i] ^= key[i];
        opad[i] ^= key[i];
    }
    let mut inner = Sha256::new();
    inner.update(ipad);
    inner.update(data);
    let inner = inner.finalize();
    let mut outer = Sha256::new();
    outer.update(opad);
    outer.update(inner);
    hex::encode(outer.finalize())
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

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0_u8, |diff, (x, y)| diff | (x ^ y)) == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn twilio_signature_accepts_valid_and_rejects_forged() {
        let url = "https://gw.example.com/twilio/voice/decision?vt=abc";
        let params = vec![("Digits".to_string(), "1".to_string())];
        let sig = STANDARD.encode(hmac_sha1(format!("{url}Digits1").as_bytes(), b"auth"));
        assert!(verify_twilio_signature(url, &params, &sig, "auth"));
        let forged = vec![("Digits".to_string(), "2".to_string())];
        assert!(!verify_twilio_signature(url, &forged, &sig, "auth"));
    }
}
