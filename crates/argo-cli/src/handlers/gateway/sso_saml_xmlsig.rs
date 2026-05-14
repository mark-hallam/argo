//! XML-DSig verification for SAML ACS responses.

use bergshamra_dsig::{DsigContext, VerifyResult};
use bergshamra_keys::KeysManager;
use chrono::{DateTime, Duration, Utc};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum SamlXmlSigError {
    MissingSignature,
    InvalidSignature,
    UntrustedCertificate,
    UnsignedAssertion,
    FutureNotBefore,
    ExpiredNotOnOrAfter,
}

pub(super) fn verify_saml_response(
    xml: &str,
    idp_cert_pem: &str,
    now: DateTime<Utc>,
    clock_skew: Duration,
) -> Result<(), SamlXmlSigError> {
    if !has_signature(xml) {
        return Err(SamlXmlSigError::MissingSignature);
    }

    let key = bergshamra_keys::loader::load_x509_cert_pem(idp_cert_pem.as_bytes())
        .map_err(|_| SamlXmlSigError::UntrustedCertificate)?;
    let mut keys = KeysManager::new();
    keys.add_key(key);
    let ctx = DsigContext::new(keys)
        .with_trusted_keys_only(true)
        .with_strict_verification(true)
        .with_enabled_key_data_x509(true);

    match bergshamra_dsig::verify::verify(&ctx, xml)
        .map_err(|_| SamlXmlSigError::InvalidSignature)?
    {
        VerifyResult::Valid { references, .. } => {
            if !references_cover_assertion(xml, &references) {
                return Err(SamlXmlSigError::UnsignedAssertion);
            }
            validate_assertion_time(xml, now, clock_skew)
        }
        VerifyResult::Invalid { .. } => Err(SamlXmlSigError::InvalidSignature),
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
        .unwrap_or("");
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
            return Err(SamlXmlSigError::FutureNotBefore);
        }
    }
    if let Some(not_on_or_after) = attr_value(xml, "NotOnOrAfter").and_then(|s| parse_saml_time(&s))
    {
        if now - clock_skew >= not_on_or_after {
            return Err(SamlXmlSigError::ExpiredNotOnOrAfter);
        }
    }
    Ok(())
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
