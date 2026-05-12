//! Integration smoke for `WhatsAppCloudClient`.
//!
//! Gated by three env vars — when ANY is unset/empty the test prints a skip
//! line and returns success (CI without secrets is a green pass):
//!
//! - `ARGO_TEST_WHATSAPP_CLOUD_ACCESS_TOKEN` — long-lived system-user token
//! - `ARGO_TEST_WHATSAPP_CLOUD_PHONE_NUMBER_ID` — sending phone-number id
//! - `ARGO_TEST_WHATSAPP_CLOUD_RECIPIENT_E164` — recipient in E.164 format
//!
//! IMPORTANT: the recipient SHOULD be the operator's own test number.
//! WhatsApp Cloud applies aggressive abuse-detection to cross-account
//! messaging and the operator's business account will be flagged or
//! disabled if smoke-test traffic looks unsolicited.
//!
//! WhatsApp does NOT support outbound-message deletion, so the smoke
//! leaves the test message in the recipient's history — no cleanup.

use argo_mcp_connector_whatsapp_cloud::{WhatsAppCloudClient, WhatsAppTemplateVariable};

const TOKEN_ENV: &str = "ARGO_TEST_WHATSAPP_CLOUD_ACCESS_TOKEN";
const PHONE_ID_ENV: &str = "ARGO_TEST_WHATSAPP_CLOUD_PHONE_NUMBER_ID";
const RECIPIENT_ENV: &str = "ARGO_TEST_WHATSAPP_CLOUD_RECIPIENT_E164";
const TEMPLATE_ENV: &str = "ARGO_TEST_WHATSAPP_CLOUD_TEMPLATE_NAME";

fn live_env() -> Option<(String, String, String)> {
    let token = non_empty(TOKEN_ENV)?;
    let phone_id = non_empty(PHONE_ID_ENV)?;
    let recipient = non_empty(RECIPIENT_ENV)?;
    Some((token, phone_id, recipient))
}

fn non_empty(key: &str) -> Option<String> {
    match std::env::var(key) {
        Ok(v) if !v.trim().is_empty() => Some(v),
        Ok(_) => {
            eprintln!("skip: {key} empty");
            None
        }
        Err(_) => {
            eprintln!("skip: {key} not set");
            None
        }
    }
}

#[tokio::test]
async fn send_text_round_trips_against_live_api() {
    let Some((token, phone_id, recipient)) = live_env() else { return };
    let client = WhatsAppCloudClient::new(phone_id).with_token_override(token);
    let id = client
        .send_text(
            &recipient,
            "skycot whatsapp-cloud smoke — please ignore",
        )
        .await
        .expect("send_text smoke should succeed");
    assert!(
        id.as_str().starts_with("wamid."),
        "smoke wamid prefix; got {}",
        id.as_str()
    );
}

#[tokio::test]
async fn send_template_round_trips_against_live_api() {
    let Some((token, phone_id, recipient)) = live_env() else { return };
    let Some(template_name) = non_empty(TEMPLATE_ENV) else {
        eprintln!(
            "skip: {TEMPLATE_ENV} not set — template send needs a pre-approved template name"
        );
        return;
    };
    let client = WhatsAppCloudClient::new(phone_id).with_token_override(token);
    let id = client
        .send_template(
            &recipient,
            &template_name,
            "en_US",
            &[WhatsAppTemplateVariable::new("Skycot smoke")],
        )
        .await
        .expect("send_template smoke should succeed");
    assert!(
        id.as_str().starts_with("wamid."),
        "smoke wamid prefix; got {}",
        id.as_str()
    );
}
