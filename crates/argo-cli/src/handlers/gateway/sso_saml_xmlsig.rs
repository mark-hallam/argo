//! XML-DSig verification for SAML ACS responses.

use bergshamra_dsig::{DsigContext, VerifyResult};
use bergshamra_keys::KeysManager;
use chrono::{DateTime, Duration, Utc};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum SamlXmlSigError {
    MissingSignature,
    SignatureRejected,
    IssuerRejected,
    DestinationRejected,
    InResponseToRejected,
    Expired,
    AudienceRejected,
    UntrustedCertificate,
    UnsignedAssertion,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SamlAssertion {
    pub name_id: Option<String>,
    pub used_sha1: bool,
}

pub(super) fn verify_saml_response(
    xml: &str,
    tenant: &argo_config::SsoSamlTenant,
    now: DateTime<Utc>,
    clock_skew: Duration,
    in_response_to_expected: Option<&str>,
) -> Result<SamlAssertion, SamlXmlSigError> {
    if !has_signature(xml) {
        return Err(SamlXmlSigError::MissingSignature);
    }
    let used_sha1 = xml.contains("rsa-sha1") || xml.contains("xmldsig#sha1");
    if used_sha1 && !tenant.accept_sha1_signatures {
        return Err(SamlXmlSigError::SignatureRejected);
    }

    let key = bergshamra_keys::loader::load_x509_cert_pem(tenant.idp_cert_pem.as_bytes())
        .map_err(|_| SamlXmlSigError::UntrustedCertificate)?;
    let mut keys = KeysManager::new();
    keys.add_key(key);
    let ctx = DsigContext::new(keys)
        .with_trusted_keys_only(true)
        .with_strict_verification(true)
        .with_enabled_key_data_x509(true);

    match bergshamra_dsig::verify::verify(&ctx, xml)
        .map_err(|_| SamlXmlSigError::SignatureRejected)?
    {
        VerifyResult::Valid { references, .. } => {
            if !references_cover_assertion(xml, &references) {
                return Err(SamlXmlSigError::UnsignedAssertion);
            }
            validate_assertion_time(xml, now, clock_skew)?;
            validate_semantics(xml, tenant, in_response_to_expected)?;
            Ok(SamlAssertion {
                name_id: element_text(xml, "NameID"),
                used_sha1,
            })
        }
        VerifyResult::Invalid { .. } => Err(SamlXmlSigError::SignatureRejected),
    }
}

fn has_signature(xml: &str) -> bool {
    xml.contains("<ds:Signature")
        || xml.contains("<dsig:Signature")
        || xml.contains("<Signature")
        || xml.contains(":Signature")
}

fn references_cover_assertion(
    xml: &str,
    references: &[bergshamra_dsig::VerifiedReference],
) -> bool {
    let ids = assertion_ids(xml);
    references.iter().any(|r| {
        r.uri.is_empty()
            || r.uri
                .strip_prefix('#')
                .is_some_and(|target| ids.iter().any(|id| id == target))
    })
}

fn assertion_ids(xml: &str) -> Vec<String> {
    let mut ids = Vec::new();
    let mut cursor = 0;
    while let Some(pos) = xml[cursor..].find("<") {
        let abs = cursor + pos;
        let Some(end) = xml[abs..].find('>') else {
            break;
        };
        let tag = &xml[abs..abs + end + 1];
        if local_name_is(tag, "Assertion") {
            for attr in ["ID", "Id", "AssertionID", "id"] {
                if let Some(value) = attr_value(tag, attr) {
                    ids.push(value);
                }
            }
        }
        cursor = abs + end + 1;
    }
    ids
}

fn local_name_is(tag: &str, expected: &str) -> bool {
    let name = tag
        .trim_start_matches('<')
        .trim_start_matches('/')
        .split_whitespace()
        .next()
        .unwrap_or("")
        .trim_end_matches('>')
        .trim_end_matches('/');
    let local = name.rsplit_once(':').map_or(name, |(_, tail)| tail);
    local == expected
}

fn validate_assertion_time(
    xml: &str,
    now: DateTime<Utc>,
    clock_skew: Duration,
) -> Result<(), SamlXmlSigError> {
    if let Some(not_before) = attr_value(xml, "NotBefore").and_then(|s| parse_saml_time(&s)) {
        if now + clock_skew < not_before {
            return Err(SamlXmlSigError::Expired);
        }
    }
    if let Some(not_on_or_after) = attr_value(xml, "NotOnOrAfter").and_then(|s| parse_saml_time(&s))
    {
        if now - clock_skew >= not_on_or_after {
            return Err(SamlXmlSigError::Expired);
        }
    }
    Ok(())
}

fn validate_semantics(
    xml: &str,
    tenant: &argo_config::SsoSamlTenant,
    in_response_to_expected: Option<&str>,
) -> Result<(), SamlXmlSigError> {
    if element_text(xml, "Issuer").as_deref() != Some(tenant.idp_entity_id.as_str()) {
        return Err(SamlXmlSigError::IssuerRejected);
    }
    if attr_value(xml, "Destination").as_deref() != Some(tenant.acs_url.as_str()) {
        return Err(SamlXmlSigError::DestinationRejected);
    }
    if attr_value(xml, "InResponseTo").as_deref() != in_response_to_expected {
        return Err(SamlXmlSigError::InResponseToRejected);
    }
    if !element_texts(xml, "Audience")
        .iter()
        .any(|aud| aud == &tenant.sp_entity_id)
    {
        return Err(SamlXmlSigError::AudienceRejected);
    }
    Ok(())
}

fn element_text(xml: &str, local: &str) -> Option<String> {
    element_texts(xml, local).into_iter().next()
}

fn element_texts(xml: &str, local: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cursor = 0;
    while let Some(open) = find_open_tag(&xml[cursor..], local) {
        let abs = cursor + open;
        let Some(close_open) = xml[abs..].find('>') else {
            break;
        };
        let body_start = abs + close_open + 1;
        let Some(close) = find_close_tag(&xml[body_start..], local) else {
            break;
        };
        out.push(xml[body_start..body_start + close].trim().to_string());
        cursor = body_start + close;
    }
    out
}

fn find_open_tag(s: &str, local: &str) -> Option<usize> {
    let mut cursor = 0;
    while let Some(pos) = s[cursor..].find('<') {
        let abs = cursor + pos;
        let Some(end) = s[abs..].find('>') else { break };
        let tag = &s[abs..abs + end + 1];
        if !tag.starts_with("</") && local_name_is(tag, local) {
            return Some(abs);
        }
        cursor = abs + end + 1;
    }
    None
}

fn find_close_tag(s: &str, local: &str) -> Option<usize> {
    [format!("</{local}>"), format!(":</{local}>")]
        .iter()
        .filter_map(|needle| s.find(needle))
        .min()
        .or_else(|| {
            let needle = format!(":{local}>");
            s.find("</")
                .and_then(|start| s[start..].find(&needle).map(|_| start))
        })
}

fn attr_value(s: &str, attr: &str) -> Option<String> {
    let quoted = format!("{attr}=\"");
    let start = s.find(&quoted)? + quoted.len();
    let rest = &s[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn parse_saml_time(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

#[cfg(test)]
#[path = "sso_saml_xmlsig_tests.rs"]
pub(super) mod tests;
