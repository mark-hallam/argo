//! Unit tests for the SAML SSO middleware. Each test pins the
//! request id + clock so the AuthnRequest XML / ACS outcomes are
//! reproducible.
use super::*;
use argo_config::{SsoConfig, SsoSamlTenant};
use std::collections::BTreeMap;

fn cfg_with_tenant(name: &str) -> SsoConfig {
    let mut saml = BTreeMap::new();
    saml.insert(
        name.into(),
        SsoSamlTenant {
            idp_entity_id: "https://idp.example.com/saml".into(),
            idp_sso_url: "https://idp.example.com/sso".into(),
            sp_entity_id: "https://sp.example.com/saml".into(),
            acs_url: format!("https://sp.example.com/auth/saml/{name}/acs"),
            idp_cert_pem: String::new(),
        },
    );
    SsoConfig {
        enabled: true,
        session_ttl_seconds: 3600,
        cookie_name: "argo_sso".into(),
        oidc: BTreeMap::new(),
        saml,
    }
}

fn fixed_instant() -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::parse_from_rfc3339("2026-05-12T15:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc)
}

#[test]
fn parse_saml_path_matches_login_and_acs() {
    assert_eq!(
        parse_saml_path("/auth/saml/default/login"),
        Some(SamlKind::Login("default".into()))
    );
    assert_eq!(
        parse_saml_path("/auth/saml/acme/acs"),
        Some(SamlKind::Acs("acme".into()))
    );
    assert!(parse_saml_path("/auth/saml/default/bogus").is_none());
    assert!(parse_saml_path("/auth/saml//login").is_none());
}

#[test]
fn build_authn_request_emits_valid_redirect_url() {
    let cfg = cfg_with_tenant("default");
    let r = build_authn_request("default", &cfg, [9_u8; 16], fixed_instant()).unwrap();
    assert!(r.url.starts_with("https://idp.example.com/sso?SAMLRequest="));
    // Decode the SAMLRequest base64 and confirm key fields appear
    // verbatim in the XML body.
    let qs = r.url.split_once("?SAMLRequest=").unwrap().1;
    let decoded_b64 = qs
        .replace("%2F", "/")
        .replace("%2B", "+")
        .replace("%3D", "=");
    let xml_bytes = b64_std_decode(&decoded_b64).unwrap();
    let xml = std::str::from_utf8(&xml_bytes).unwrap();
    assert!(xml.contains("samlp:AuthnRequest"));
    assert!(xml.contains("https://sp.example.com/saml"));
    assert!(xml.contains("Destination=\"https://idp.example.com/sso\""));
    assert!(xml.contains("urn:oasis:names:tc:SAML:2.0:bindings:HTTP-POST"));
    assert!(r.request_id.starts_with('_'));
}

#[test]
fn build_authn_request_rejects_unknown_tenant() {
    let cfg = cfg_with_tenant("default");
    let err = build_authn_request("other", &cfg, [0; 16], fixed_instant()).unwrap_err();
    assert!(err.contains("no [sso.saml.other]"));
}

#[test]
fn extract_name_id_pulls_value_and_attribute_email() {
    let xml = r#"<samlp:Response xmlns:samlp="urn:oasis:names:tc:SAML:2.0:protocol" xmlns:saml="urn:oasis:names:tc:SAML:2.0:assertion">
      <saml:Assertion>
        <saml:Subject>
          <saml:NameID Format="urn:oasis:names:tc:SAML:1.1:nameid-format:unspecified">u-77</saml:NameID>
        </saml:Subject>
        <saml:AttributeStatement>
          <saml:Attribute Name="email"><saml:AttributeValue>bob@example.com</saml:AttributeValue></saml:Attribute>
        </saml:AttributeStatement>
      </saml:Assertion>
    </samlp:Response>"#;
    let (sub, email) = extract_name_id(xml).unwrap();
    assert_eq!(sub, "u-77");
    assert_eq!(email, "bob@example.com");
}

#[test]
fn extract_name_id_falls_back_to_sub_when_no_email_attribute() {
    let xml = r#"<saml:NameID Format="emailAddress">carol@example.com</saml:NameID>"#;
    let (sub, email) = extract_name_id(xml).unwrap();
    assert_eq!(sub, "carol@example.com");
    assert_eq!(email, "carol@example.com");
}

#[test]
fn build_acs_outcome_sets_session_cookie_on_success() {
    let cfg = cfg_with_tenant("default");
    let xml = "<saml:NameID>alice</saml:NameID>";
    let b64 = b64_std_encode(xml.as_bytes());
    let body = format!(
        "SAMLResponse={}&RelayState=",
        super::super::sso_oidc::url_encode(&b64)
    );
    let outcome = build_acs_outcome(
        &body,
        "default",
        &cfg,
        "saml-test-secret-must-be-32-chars-or-more",
        fixed_instant(),
    );
    assert_eq!(outcome.status, "302 Found");
    assert_eq!(outcome.location.as_deref(), Some("/app/"));
    let sc = outcome.set_cookie.unwrap();
    assert!(sc.contains("argo_sso="));
    assert!(sc.contains("HttpOnly"));
    assert!(sc.contains("Secure"));
}
