//! Tests for SAML XML-DSig verification.

use super::*;

pub(in super::super) const TEST_CERT_PEM: &str = r#"-----BEGIN CERTIFICATE-----
MIIDFTCCAf2gAwIBAgIUPPayFv73ZH8tvavRgk+6LG1S3HEwDQYJKoZIhvcNAQEL
BQAwGjEYMBYGA1UEAwwPc2t5Y290LXRlc3QtaWRwMB4XDTI2MDUxNDE5MTIwMFoX
DTM2MDUxMTE5MTIwMFowGjEYMBYGA1UEAwwPc2t5Y290LXRlc3QtaWRwMIIBIjAN
BgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAzfjE97tcbJMtY7UWr26FpTyjF2d2
pR63gwSaqN7aPxk1qkOzkh0qdEBiiAza3syQTsdHaFtOJ6LyQxoHz+3MTP6J91YE
8jaA2hyTsNApHCeQ7tjb8CCjrS4AXqbtILPu7YMkPxjVd1yzq7OooF2GcRZgxkr8
D3TcMv0ap/gwZ82Rp1yY6EdfrITr4OB646u0xCBxuSyqcNQd3Use8CjWjSwJy/Y9
Hc3VhoDQWoV39vFNLakYkaiC1S4gRjVwmkUkwaDJr+S96fNcbuJzwuzcH3jeo6Vo
bX7f0dEt3dINo+BBH3umH/Fg6vHk3J/NHZJa6hhUYZ+tEII/xx1/D3ZexwIDAQAB
o1MwUTAdBgNVHQ4EFgQUvxa+3FFhqYPL1B9jsc4GcemE59UwHwYDVR0jBBgwFoAU
vxa+3FFhqYPL1B9jsc4GcemE59UwDwYDVR0TAQH/BAUwAwEB/zANBgkqhkiG9w0B
AQsFAAOCAQEANfZLw9g8sjq2jnSR8v6+nT4WvRGbETAEL+rvRRbTV5gcRflPAncj
Gxr+1G4g42EOTo5UACzNf/JEchqVcNE//br597RH50GpZ7iQsDNEukP9jTbWmIXV
MMrL9kHM5gBv54iNh8Qm17OdDef6Z5DS6QFcXGu47P/6otdrfj7jq7cz3yGcxCId
kW+4n9Lzp5rmtwyjHzBPO+fdArBqnqJHjcnrA4oPcgUC6b+PDqMiMgFIf4svsXQG
62NRYDxnDxcyOBjt5+8d6uywrE11CXdkmNA9kg87n6rtwTQiIDktBhjB9OF3XEP4
5KV3UjsU9EpWO0SbqY1osLavrn7P4ntgGA==
-----END CERTIFICATE-----"#;

const WRONG_CERT_PEM: &str = r#"-----BEGIN CERTIFICATE-----
MIIDFzCCAf+gAwIBAgIUPnpn5R9UqjjSB6e5/vPvkoQpyXcwDQYJKoZIhvcNAQEL
BQAwGzEZMBcGA1UEAwwQc2t5Y290LXdyb25nLWlkcDAeFw0yNjA1MTQxOTEzNDha
Fw0zNjA1MTExOTEzNDhaMBsxGTAXBgNVBAMMEHNreWNvdC13cm9uZy1pZHAwggEi
MA0GCSqGSIb3DQEBAQUAA4IBDwAwggEKAoIBAQC3lNk8F/YK8WfIYzij5Qc/5ZhH
JuduY3yHTAYsLhEsoXmxdKVz58x17yCzecpiU/TY+npr45VndLQYQljVMprBeWgw
GtlUH98Ugx03pXu2xERgfscrdntpEQkE/aaAgI83NikDFyQXWyXo4JaQksl+imSo
tsV9tns1gwfdiwCcmmKzBC9fAbE79kGN8GwmVIYf9v5a+cMY3fck+pzSRfhaM2nZ
WXP/mN6fzexdrdSQ/3AqplP+TBrbZSZOiLpgLx281C+G+yyn4eAvwi7pFYRBMtvF
I2A5HlNiLGAFDIzUo3sdYH4KSE9nwXU2VuR1yAPvuBllG7JX8PHJVDCY5tG5AgMB
AAGjUzBRMB0GA1UdDgQWBBTqXceDhURG+fzgrA2DgFq6K9SeTzAfBgNVHSMEGDAW
gBTqXceDhURG+fzgrA2DgFq6K9SeTzAPBgNVHRMBAf8EBTADAQH/MA0GCSqGSIb3
DQEBCwUAA4IBAQB0LSR9X7u83tjn3qnWdr7NjeG3QQC7uGFqyqNJgd0bapTQnHgR
u33dOmgpRlVMYy6NtIa/DW7FOFyOOgzVM+OD38S58Z8xuQxYZpOo+nWKBv007mFG
t4yuz+GFbe2wOvFGPs2kOkiPn4AjE7SwM5+eFD5U8gIQ7ClyPPGrgcdmxOji5SpF
q6Zm2nYXa8c8odyTD8vJwL16217IPKjQHlX5b9aSHc3+YCRXorC0C8ZGuxy1JxU5
ELwFnVPPeSIin5c+BX4cLB/UdRcIC1PDXm6VapqUShUAGmS2JAnabLvCta6+RAJm
etvq9qtXX7J1YhgVtIUrMGC/wMEK7n3Ip3Mc
-----END CERTIFICATE-----"#;

const TEST_KEY_PEM: &str = r#"-----BEGIN PRIVATE KEY-----
MIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQDN+MT3u1xsky1j
tRavboWlPKMXZ3alHreDBJqo3to/GTWqQ7OSHSp0QGKIDNrezJBOx0doW04novJD
GgfP7cxM/on3VgTyNoDaHJOw0CkcJ5Du2NvwIKOtLgBepu0gs+7tgyQ/GNV3XLOr
s6igXYZxFmDGSvwPdNwy/Rqn+DBnzZGnXJjoR1+shOvg4Hrjq7TEIHG5LKpw1B3d
Sx7wKNaNLAnL9j0dzdWGgNBahXf28U0tqRiRqILVLiBGNXCaRSTBoMmv5L3p81xu
4nPC7NwfeN6jpWhtft/R0S3d0g2j4EEfe6Yf8WDq8eTcn80dklrqGFRhn60Qgj/H
HX8Pdl7HAgMBAAECggEAT5psAXRzcXDmM2lK5a040JYhUAvIEmPPN78hSM+sGIPV
NkMpFPjZrgkuK3eQZVg5E4E4C1ByYaY7rAxU5Zr5gO5FAwbHLZc78E/oEoc4xC1U
ecLtKZ5rs6QaN3qbHa780eWDrjEaWyHEPMFYE/ZhXbZw22IijZSz/67LTQyCJLNg
F46cnJVeEFdbz+x2DjghyMwNrNDlfmYMgWmk04ESdV86cqK07INozGi0Jie/xKAb
P3XOJJ75B8ZjCpjd0w649H93zOLn5Qav8eqxkDTl0TKzYo8V2aFyE3c+Fenq9YTH
gSYlmZRd1oxmciPAGFc0WBozc4vvluwqV1AJPvDd6QKBgQDtODQhZGH1DTOmf6Bd
mX/sT2E5pB3tN42SwcvZA3OedL9dluwZg8QA811cM+nq4nsnllmN2VxUspnHj0/E
woLNlkBTvbzg6rHTtFspXoUp1nNSWUkULy5MdH95Kidq+acoxnIcWll/PfGhFo9M
VArwJbXtyRk1SN6CHMQJeN8lmQKBgQDeR0IEISoNjZ7cxIFfKA6vNDWIvSzBH0On
WOgtppXRA0IWfaC9nOU2w03l9Y72nthrm6LUQrWXdalTbGnDT/5q4udp+wCiaVSE
BIMAEzWW/Bh88Kjd4DvoznsqQH68cafdWQklguLbhsNb3mu6bC0JZC8FM02Jqnub
jH/FIc+jXwKBgQC7/S5SyBpY+H1WFQ4hHuQhgUarR0PwGXY4Nxw0fmfBywNcR54T
GjjaCAKUbw+Q9KpkVOHjZ72j4WW8n/W3F+5QviF8O4DTPSpvex5DSoR9A1WgjOE8
/abTL/9VJfLW88Fkrt7/WRpTm+NApYAqHPAOH9Rs/FDdyUGMGv/AAenVAQKBgEop
aENMdWV0PO7XuXpwOQYH21y2EXqV3dK1/TFOBWppNeGfJMRe1rNA0nrJRVvPf9HX
aap8X24bWdV1tIvv4qjy79cSYhHOJt6tSq/5uMWZcuosuYrdx99TFRDx19cz7JgT
ntOfKeWovwGl4jee5B/gBzgq99HYyckKRY8WStrtAoGADOYK3KoYUbgne2m43EMx
PviQWUtKHBGBBffkZd6xu5/jPNhYPEi4Y/p/kHzd8a0g/5iORbKQTm5Fm65QR23H
AfYdLFY0fCTH425SBpQz8Ffjl5ZVOCEb5aM4WuT5brvr+gd+c11toAyRXg9rY7fN
iuA3CWSSYZdy3/11D8mABVE=
-----END PRIVATE KEY-----"#;

fn now() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-05-12T15:00:00Z")
        .unwrap()
        .with_timezone(&Utc)
}

fn tenant() -> argo_config::SsoSamlTenant {
    argo_config::SsoSamlTenant {
        idp_entity_id: "https://idp.example.com/saml".into(),
        idp_sso_url: "https://idp.example.com/sso".into(),
        sp_entity_id: "https://sp.example.com/saml".into(),
        acs_url: "https://sp.example.com/auth/saml/default/acs".into(),
        idp_cert_pem: TEST_CERT_PEM.into(),
        accept_sha1_signatures: false,
    }
}

pub(in super::super) fn signed_response(
    subject: &str,
    not_before: &str,
    not_on_or_after: &str,
) -> String {
    let xml = format!(
        r##"<samlp:Response xmlns:samlp="urn:oasis:names:tc:SAML:2.0:protocol" xmlns:saml="urn:oasis:names:tc:SAML:2.0:assertion" ID="_response" Destination="https://sp.example.com/auth/saml/default/acs" InResponseTo="_request">
  <saml:Issuer>https://idp.example.com/saml</saml:Issuer>
  <saml:Assertion ID="_assertion">
    <ds:Signature xmlns:ds="http://www.w3.org/2000/09/xmldsig#">
      <ds:SignedInfo>
        <ds:CanonicalizationMethod Algorithm="http://www.w3.org/2001/10/xml-exc-c14n#"/>
        <ds:SignatureMethod Algorithm="http://www.w3.org/2001/04/xmldsig-more#rsa-sha256"/>
        <ds:Reference URI="#_assertion">
          <ds:Transforms>
            <ds:Transform Algorithm="http://www.w3.org/2000/09/xmldsig#enveloped-signature"/>
            <ds:Transform Algorithm="http://www.w3.org/2001/10/xml-exc-c14n#"/>
          </ds:Transforms>
          <ds:DigestMethod Algorithm="http://www.w3.org/2001/04/xmlenc#sha256"/>
          <ds:DigestValue></ds:DigestValue>
        </ds:Reference>
      </ds:SignedInfo>
      <ds:SignatureValue></ds:SignatureValue>
    </ds:Signature>
    <saml:Subject><saml:NameID>{subject}</saml:NameID></saml:Subject>
    <saml:Conditions NotBefore="{not_before}" NotOnOrAfter="{not_on_or_after}">
      <saml:AudienceRestriction><saml:Audience>https://sp.example.com/saml</saml:Audience></saml:AudienceRestriction>
    </saml:Conditions>
  </saml:Assertion>
</samlp:Response>"##
    );
    let key = bergshamra_keys::loader::load_rsa_private_pem(TEST_KEY_PEM.as_bytes()).unwrap();
    let mut keys = KeysManager::new();
    keys.add_key(key);
    bergshamra_dsig::sign::sign(&DsigContext::new(keys), &xml).unwrap()
}

fn valid_xml() -> String {
    signed_response(
        "alice@example.com",
        "2026-05-12T14:59:00Z",
        "2026-05-12T15:05:00Z",
    )
}

#[test]
fn accepts_valid_signature() {
    let assertion = verify_saml_response(
        &valid_xml(),
        &tenant(),
        now(),
        Duration::seconds(60),
        Some("_request"),
    )
    .unwrap();
    assert_eq!(assertion.name_id.as_deref(), Some("alice@example.com"));
}

#[test]
fn rejects_tampered_signed_content() {
    let xml = valid_xml().replace("alice@example.com", "mallory@example.com");
    assert_eq!(
        verify_saml_response(
            &xml,
            &tenant(),
            now(),
            Duration::seconds(60),
            Some("_request")
        ),
        Err(SamlXmlSigError::SignatureRejected)
    );
}

#[test]
fn rejects_missing_signature() {
    let xml = "<samlp:Response><saml:Assertion ID=\"_a\"><saml:NameID>a</saml:NameID></saml:Assertion></samlp:Response>";
    assert_eq!(
        verify_saml_response(
            xml,
            &tenant(),
            now(),
            Duration::seconds(60),
            Some("_request")
        ),
        Err(SamlXmlSigError::MissingSignature)
    );
}

#[test]
fn rejects_wrong_certificate() {
    let mut t = tenant();
    t.idp_cert_pem = WRONG_CERT_PEM.into();
    assert_eq!(
        verify_saml_response(
            &valid_xml(),
            &t,
            now(),
            Duration::seconds(60),
            Some("_request")
        ),
        Err(SamlXmlSigError::SignatureRejected)
    );
}

#[test]
fn rejects_expired_not_on_or_after() {
    let xml = signed_response("a", "2026-05-12T14:00:00Z", "2026-05-12T14:58:59Z");
    assert_eq!(
        verify_saml_response(
            &xml,
            &tenant(),
            now(),
            Duration::seconds(60),
            Some("_request")
        ),
        Err(SamlXmlSigError::Expired)
    );
}

#[test]
fn rejects_future_not_before() {
    let xml = signed_response("a", "2026-05-12T15:01:01Z", "2026-05-12T16:00:00Z");
    assert_eq!(
        verify_saml_response(
            &xml,
            &tenant(),
            now(),
            Duration::seconds(60),
            Some("_request")
        ),
        Err(SamlXmlSigError::Expired)
    );
}

#[test]
fn accepts_not_before_within_clock_skew() {
    let xml = signed_response("a", "2026-05-12T15:00:45Z", "2026-05-12T16:00:00Z");
    assert!(verify_saml_response(
        &xml,
        &tenant(),
        now(),
        Duration::seconds(60),
        Some("_request")
    )
    .is_ok());
}

#[test]
fn rejects_not_before_outside_clock_skew() {
    let xml = signed_response("a", "2026-05-12T15:00:45Z", "2026-05-12T16:00:00Z");
    assert_eq!(
        verify_saml_response(
            &xml,
            &tenant(),
            now(),
            Duration::seconds(30),
            Some("_request")
        ),
        Err(SamlXmlSigError::Expired)
    );
}

#[test]
fn rejects_wrong_issuer() {
    let mut t = tenant();
    t.idp_entity_id = "https://other.example.com/saml".into();
    assert_eq!(
        verify_saml_response(
            &valid_xml(),
            &t,
            now(),
            Duration::seconds(60),
            Some("_request")
        ),
        Err(SamlXmlSigError::IssuerRejected)
    );
}

#[test]
fn rejects_wrong_destination() {
    let mut t = tenant();
    t.acs_url = "https://sp.example.com/other".into();
    assert_eq!(
        verify_saml_response(
            &valid_xml(),
            &t,
            now(),
            Duration::seconds(60),
            Some("_request")
        ),
        Err(SamlXmlSigError::DestinationRejected)
    );
}

#[test]
fn rejects_wrong_in_response_to() {
    assert_eq!(
        verify_saml_response(
            &valid_xml(),
            &tenant(),
            now(),
            Duration::seconds(60),
            Some("_other")
        ),
        Err(SamlXmlSigError::InResponseToRejected)
    );
}

#[test]
fn rejects_wrong_audience() {
    let mut t = tenant();
    t.sp_entity_id = "https://sp.example.com/other".into();
    assert_eq!(
        verify_saml_response(
            &valid_xml(),
            &t,
            now(),
            Duration::seconds(60),
            Some("_request")
        ),
        Err(SamlXmlSigError::AudienceRejected)
    );
}
