//! Connector spec table + auth types for `connector_authorize`.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthKind {
    OAuth,
    Token,
}

#[derive(Clone, Copy, Debug)]
pub struct OAuthSpec {
    pub client_id_key: &'static str,
    pub client_secret_key: &'static str,
    pub refresh_key: Option<&'static str>,
    pub instance_url_key: Option<&'static str>,
    pub auth_url: &'static str,
    pub token_url: &'static str,
    pub scopes: &'static [&'static str],
}

#[derive(Clone, Copy, Debug)]
pub struct ConnectorSpec {
    pub kind: &'static str,
    pub auth: AuthKind,
    pub token_key: &'static str,
    pub base_url_key: Option<&'static str>,
    pub default_base_url: &'static str,
    pub oauth: Option<OAuthSpec>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthState {
    pub kind: String,
    pub auth_type: AuthKind,
    pub credential_keys: Vec<String>,
    pub authorized_at_unix: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct OAuthTokenCache {
    pub(super) access_token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) refresh_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) expires_at_unix: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) expires_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) instance_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) token_type: Option<String>,
}

pub const CONNECTORS: &[ConnectorSpec] = &[
    oauth(
        "gmail",
        "GMAIL_ACCESS_TOKEN",
        "GMAIL_BASE_URL",
        "https://gmail.googleapis.com",
        OAuthSpec {
            client_id_key: "ARGO_GMAIL_CLIENT_ID",
            client_secret_key: "ARGO_GMAIL_CLIENT_SECRET",
            refresh_key: Some("ARGO_GMAIL_REFRESH_TOKEN"),
            instance_url_key: None,
            auth_url: "https://accounts.google.com/o/oauth2/v2/auth",
            token_url: "https://oauth2.googleapis.com/token",
            scopes: &["https://www.googleapis.com/auth/gmail.modify"],
        },
    ),
    oauth(
        "gcal",
        "GCAL_ACCESS_TOKEN",
        "GCAL_BASE_URL",
        "https://www.googleapis.com/calendar/v3",
        OAuthSpec {
            client_id_key: "ARGO_GCAL_CLIENT_ID",
            client_secret_key: "ARGO_GCAL_CLIENT_SECRET",
            refresh_key: Some("ARGO_GCAL_REFRESH_TOKEN"),
            instance_url_key: None,
            auth_url: "https://accounts.google.com/o/oauth2/v2/auth",
            token_url: "https://oauth2.googleapis.com/token",
            scopes: &["https://www.googleapis.com/auth/calendar"],
        },
    ),
    oauth(
        "hubspot",
        "HUBSPOT_PRIVATE_APP_TOKEN",
        "HUBSPOT_BASE_URL",
        "https://api.hubapi.com",
        OAuthSpec {
            client_id_key: "HUBSPOT_CLIENT_ID",
            client_secret_key: "HUBSPOT_CLIENT_SECRET",
            refresh_key: Some("HUBSPOT_REFRESH_TOKEN"),
            instance_url_key: None,
            auth_url: "https://app.hubspot.com/oauth/authorize",
            token_url: "https://api.hubapi.com/oauth/v1/token",
            scopes: &["crm.objects.contacts.read"],
        },
    ),
    oauth(
        "notion",
        "NOTION_INTEGRATION_TOKEN",
        "NOTION_BASE_URL",
        "https://api.notion.com/v1",
        OAuthSpec {
            client_id_key: "NOTION_CLIENT_ID",
            client_secret_key: "NOTION_CLIENT_SECRET",
            refresh_key: Some("NOTION_REFRESH_TOKEN"),
            instance_url_key: None,
            auth_url: "https://api.notion.com/v1/oauth/authorize",
            token_url: "https://api.notion.com/v1/oauth/token",
            scopes: &[],
        },
    ),
    oauth(
        "salesforce",
        "SF_ACCESS_TOKEN",
        "SF_INSTANCE_URL",
        "https://login.salesforce.com",
        OAuthSpec {
            client_id_key: "SF_CLIENT_ID",
            client_secret_key: "SF_CLIENT_SECRET",
            refresh_key: Some("SF_REFRESH_TOKEN"),
            instance_url_key: Some("SF_INSTANCE_URL"),
            auth_url: "https://login.salesforce.com/services/oauth2/authorize",
            token_url: "https://login.salesforce.com/services/oauth2/token",
            scopes: &["api", "refresh_token"],
        },
    ),
    token(
        "slack",
        "SLACK_BOT_TOKEN",
        "SLACK_API_BASE_URL",
        "https://slack.com/api",
    ),
    token(
        "jira",
        "JIRA_API_TOKEN",
        "JIRA_BASE_URL",
        "https://example.atlassian.net",
    ),
    token(
        "asana",
        "ASANA_PAT",
        "ASANA_BASE_URL",
        "https://app.asana.com/api/1.0",
    ),
    token(
        "linear",
        "LINEAR_API_KEY",
        "LINEAR_BASE_URL",
        "https://api.linear.app/graphql",
    ),
    token(
        "monday",
        "MONDAY_API_TOKEN",
        "MONDAY_BASE_URL",
        "https://api.monday.com/v2",
    ),
    token(
        "pagerduty",
        "PAGERDUTY_API_KEY",
        "PAGERDUTY_BASE_URL",
        "https://api.pagerduty.com",
    ),
    token(
        "fireflies",
        "FIREFLIES_API_KEY",
        "FIREFLIES_BASE_URL",
        "https://api.fireflies.ai/graphql",
    ),
    token(
        "airtable",
        "AIRTABLE_PAT",
        "AIRTABLE_BASE_URL",
        "https://api.airtable.com/v0",
    ),
    token(
        "stripe",
        "STRIPE_API_KEY",
        "STRIPE_BASE_URL",
        "https://api.stripe.com",
    ),
    token(
        "zendesk",
        "ZENDESK_API_TOKEN",
        "ZENDESK_BASE_URL",
        "https://example.zendesk.com",
    ),
    token(
        "clickup",
        "CLICKUP_TOKEN",
        "CLICKUP_BASE_URL",
        "https://api.clickup.com/api/v2",
    ),
    token(
        "discord",
        "DISCORD_BOT_TOKEN",
        "DISCORD_BASE_URL",
        "https://discord.com/api/v10",
    ),
];

const fn oauth(
    kind: &'static str,
    token_key: &'static str,
    base_url_key: &'static str,
    default_base_url: &'static str,
    oauth: OAuthSpec,
) -> ConnectorSpec {
    ConnectorSpec {
        kind,
        auth: AuthKind::OAuth,
        token_key,
        base_url_key: Some(base_url_key),
        default_base_url,
        oauth: Some(oauth),
    }
}

const fn token(
    kind: &'static str,
    token_key: &'static str,
    base_url_key: &'static str,
    default_base_url: &'static str,
) -> ConnectorSpec {
    ConnectorSpec {
        kind,
        auth: AuthKind::Token,
        token_key,
        base_url_key: Some(base_url_key),
        default_base_url,
        oauth: None,
    }
}
