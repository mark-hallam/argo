use super::sso_saml_xmlsig;
use crate::handlers::gateway::sso_session::{self, set_cookie_header, SessionPayload};

/// Outcome of a SAML ACS POST. Pure-data return type so the
/// async unit test can assert without TCP.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct AcsOutcome {
    pub status: &'static str,
    pub body: String,
    pub location: Option<String>,
    pub set_cookie: Option<String>,
    pub audit_outcome: &'static str,
    pub audit_name_id: Option<String>,
    pub audit_reason: Option<String>,
}

pub(crate) fn build_acs_outcome(
    body: &str,
    tenant: &str,
    cfg: &argo_config::SsoConfig,
    secret: &str,
    now: chrono::DateTime<chrono::Utc>,
    in_response_to_expected: Option<&str>,
) -> AcsOutcome {
    let Some(tcfg) = cfg.saml.get(tenant) else {
        return err_outcome("400 Bad Request", "tenant not configured");
    };
    let form = parse_form(body);
    let raw = match form.iter().find(|(k, _)| k == "SAMLResponse") {
        Some((_, v)) => v.clone(),
        None => return err_outcome("400 Bad Request", "missing SAMLResponse"),
    };
    let xml_bytes = match b64_std_decode(&raw) {
        Ok(b) => b,
        Err(_) => return err_outcome("400 Bad Request", "bad base64"),
    };
    let xml = match std::str::from_utf8(&xml_bytes) {
        Ok(s) => s,
        Err(_) => return err_outcome("400 Bad Request", "response not utf8"),
    };
    let assertion = match sso_saml_xmlsig::verify_saml_response(
        xml,
        tcfg,
        now,
        saml_clock_skew(),
        in_response_to_expected,
    ) {
        Ok(a) => a,
        Err(e) => return unauthorized_outcome(format!("{e:?}")),
    };
    if assertion.used_sha1 {
        tracing::warn!(target: "argo_cli::sso_saml", tenant, "accepted legacy SAML RSA-SHA1 signature");
    }
    let (sub, email) = match extract_name_id(xml) {
        Some(t) => t,
        None => return err_outcome("400 Bad Request", "no NameID"),
    };
    let payload = SessionPayload {
        sub: sub.clone(),
        email,
        tenant: tenant.to_string(),
        mechanism: "saml".into(),
        iat: now.timestamp(),
        exp: now.timestamp() + i64::try_from(cfg.session_ttl_seconds).unwrap_or(28_800),
    };
    let token = match sso_session::sign(&payload, secret) {
        Ok(t) => t,
        Err(_) => return err_outcome("500 Internal Server Error", "sign"),
    };
    AcsOutcome {
        status: "302 Found",
        body: String::new(),
        location: Some("/app/".into()),
        set_cookie: Some(set_cookie_header(
            &cfg.cookie_name,
            &token,
            cfg.session_ttl_seconds,
            "/",
        )),
        audit_outcome: if assertion.used_sha1 {
            "verified_sha1"
        } else {
            "verified"
        },
        audit_name_id: assertion.name_id.or(Some(sub)),
        audit_reason: None,
    }
}

/// Pull `<saml:NameID>...</saml:NameID>` plus optional email
/// attribute out of an XML Response.
pub(crate) fn extract_name_id(xml: &str) -> Option<(String, String)> {
    let name_id = extract_between(xml, "<saml:NameID", "</saml:NameID>")
        .or_else(|| extract_between(xml, "<saml2:NameID", "</saml2:NameID>"))
        .or_else(|| extract_between(xml, "<NameID", "</NameID>"))?;
    let sub = strip_open_tag(&name_id);
    let email = extract_attribute_value(xml, "email")
        .or_else(|| extract_attribute_value(xml, "mail"))
        .unwrap_or_else(|| sub.clone());
    Some((sub, email))
}

fn extract_between(s: &str, open: &str, close: &str) -> Option<String> {
    let start = s.find(open)?;
    let after_open = &s[start..];
    let close_idx = after_open.find(close)?;
    Some(after_open[..close_idx].to_string())
}

fn strip_open_tag(s: &str) -> String {
    if let Some(idx) = s.find('>') {
        s[idx + 1..].trim().to_string()
    } else {
        s.trim().to_string()
    }
}

fn extract_attribute_value(xml: &str, key: &str) -> Option<String> {
    let key_lc = key.to_lowercase();
    let lower = xml.to_lowercase();
    let mut cursor = 0_usize;
    while let Some(pos) = lower[cursor..].find("attribute ") {
        let abs = cursor + pos;
        let after = &xml[abs..];
        let close = after.find('>')?;
        let header = &after[..close];
        if header
            .to_lowercase()
            .contains(&format!("name=\"{key_lc}\""))
        {
            let body_after = &after[close + 1..];
            let body_lc = body_after.to_lowercase();
            let val_start = ["<attributevalue", ":attributevalue"]
                .iter()
                .filter_map(|needle| body_lc.find(needle))
                .min()?;
            let elem_start = body_after[..=val_start].rfind('<')?;
            let after_val = &body_after[elem_start..];
            let val_close = after_val.find('>')?;
            let val_body = &after_val[val_close + 1..];
            let end_lc = val_body.to_lowercase();
            let end = end_lc.find("</")?;
            return Some(val_body[..end].trim().to_string());
        }
        cursor = abs + close + 1;
    }
    None
}

fn parse_form(body: &str) -> Vec<(String, String)> {
    body.split('&')
        .filter(|s| !s.is_empty())
        .filter_map(|kv| {
            let (k, v) = kv.split_once('=')?;
            Some((k.to_string(), percent_decode(v)))
        })
        .collect()
}

fn percent_decode(s: &str) -> String {
    let mut out = Vec::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < bytes.len() => {
                if let (Some(h), Some(l)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2])) {
                    out.push((h << 4) | l);
                    i += 2;
                } else {
                    out.push(b'%');
                }
            }
            b => out.push(b),
        }
        i += 1;
    }
    String::from_utf8(out).unwrap_or_default()
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

pub(super) fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub(crate) fn b64_std_encode(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

pub(crate) fn b64_std_decode(s: &str) -> Result<Vec<u8>, base64::DecodeError> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.decode(s.as_bytes())
}

fn saml_clock_skew() -> chrono::Duration {
    let seconds = std::env::var("ARGO_SAML_CLOCK_SKEW_SECS")
        .ok()
        .and_then(|s| s.parse::<i64>().ok())
        .filter(|s| *s >= 0)
        .unwrap_or(60);
    chrono::Duration::seconds(seconds)
}

fn unauthorized_outcome(reason: String) -> AcsOutcome {
    AcsOutcome {
        status: "401 Unauthorized",
        body: String::new(),
        location: None,
        set_cookie: None,
        audit_outcome: "rejected",
        audit_name_id: None,
        audit_reason: Some(reason),
    }
}

fn err_outcome(status: &'static str, msg: &str) -> AcsOutcome {
    AcsOutcome {
        status,
        body: format!(r#"{{"error":"{msg}"}}"#),
        location: None,
        set_cookie: None,
        audit_outcome: "rejected",
        audit_name_id: None,
        audit_reason: Some(msg.into()),
    }
}
