//! Xero Accounting REST client.
//!
//! Issues typed requests against `https://api.xero.com/api.xro/2.0/...`,
//! authenticating with the OAuth 2.0 bearer token passed at construction
//! time and including the `xero-tenant-id` header on every request
//! (Xero's per-organization multi-tenancy). A 401 response surfaces as
//! a typed `ArgoError::Provider` so callers can route to re-auth — the
//! connector does not refresh tokens itself.

use crate::types::{
    AccountWire, ContactWire, InvoiceCreate, InvoiceWire, LineItemOut, NamedIdRef, NewPayment,
    NewXeroInvoice, PaymentCreate, PaymentWire, XeroAccount, XeroContact, XeroInvoice,
    XeroInvoiceStatus, XeroPayment, cents_to_dollars,
};
use argo_types::{ArgoError, ArgoResult};
use serde::Deserialize;

/// Default Xero Accounting API base URL.
pub const XERO_API_BASE_URL: &str = "https://api.xero.com/api.xro/2.0";

const TENANT_HEADER: &str = "xero-tenant-id";

/// Xero Accounting API connector.
pub struct XeroConnector {
    client: reqwest::Client,
    oauth_token: String,
    tenant_id: String,
    base_url: String,
}

impl XeroConnector {
    /// Build a connector for `tenant_id` (Xero organization) authenticated
    /// with the supplied OAuth 2.0 access token.
    ///
    /// The `xero-tenant-id` header is set per request from `tenant_id` —
    /// a consultant connecting to multiple client orgs constructs one
    /// `XeroConnector` per org.
    #[must_use]
    pub fn new(oauth_token: String, tenant_id: String) -> Self {
        let client = reqwest::Client::builder()
            .user_agent(concat!(
                "argo-mcp-connector-xero/",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            client,
            oauth_token,
            tenant_id,
            base_url: XERO_API_BASE_URL.to_string(),
        }
    }

    /// Override the API base URL — used by tests to point at a wiremock
    /// server. Trailing slash is stripped.
    #[must_use]
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into().trim_end_matches('/').to_string();
        self
    }

    /// GET `/Invoices`, optionally filtered to a single status. Returns
    /// the full result set; pagination beyond 100 invoices is out of
    /// scope for the first cut.
    pub async fn list_invoices(
        &self,
        status: Option<XeroInvoiceStatus>,
    ) -> ArgoResult<Vec<XeroInvoice>> {
        #[derive(Deserialize, Default)]
        struct Resp {
            #[serde(rename = "Invoices", default)]
            invoices: Vec<InvoiceWire>,
        }
        let mut req = self.get("Invoices");
        if let Some(s) = status.as_ref() {
            req = req.query(&[("where", format!("Status==\"{}\"", s.as_wire()))]);
        }
        let resp: Resp = self.send(req).await?;
        Ok(resp.invoices.into_iter().map(Into::into).collect())
    }

    /// GET `/Invoices/{InvoiceID}`. Returns `Ok(None)` on 404 so callers
    /// can distinguish "missing" from "transport error".
    pub async fn get_invoice(&self, invoice_id: &str) -> ArgoResult<Option<XeroInvoice>> {
        #[derive(Deserialize, Default)]
        struct Resp {
            #[serde(rename = "Invoices", default)]
            invoices: Vec<InvoiceWire>,
        }
        let path = format!("Invoices/{invoice_id}");
        let req = self.get(&path);
        match self.send_optional::<Resp>(req).await? {
            None => Ok(None),
            Some(resp) => Ok(resp.invoices.into_iter().next().map(Into::into)),
        }
    }

    /// POST `/Invoices` wrapping `draft` in Xero's `Invoices: [...]`
    /// envelope.
    pub async fn create_invoice(&self, draft: NewXeroInvoice) -> ArgoResult<XeroInvoice> {
        let status = draft
            .status
            .as_ref()
            .map(XeroInvoiceStatus::as_wire)
            .unwrap_or("DRAFT");
        let body = serde_json::json!({
            "Invoices": [InvoiceCreate {
                kind: "ACCREC",
                contact: NamedIdRef::contact(&draft.contact_id),
                due_date: draft.due_date.format("%Y-%m-%d").to_string(),
                line_items: draft
                    .line_items
                    .into_iter()
                    .map(|l| LineItemOut {
                        description: l.description,
                        quantity: l.quantity,
                        unit_amount: cents_to_dollars(l.unit_amount_cents),
                        account_code: l.account_code,
                    })
                    .collect(),
                reference: draft.reference,
                status,
            }],
        });
        #[derive(Deserialize, Default)]
        struct Resp {
            #[serde(rename = "Invoices", default)]
            invoices: Vec<InvoiceWire>,
        }
        let resp: Resp = self.post_json("Invoices", &body).await?;
        resp.invoices
            .into_iter()
            .next()
            .map(Into::into)
            .ok_or_else(|| ArgoError::Provider("xero create_invoice: empty Invoices[]".into()))
    }

    /// GET `/Contacts`. First-page results only.
    pub async fn list_contacts(&self) -> ArgoResult<Vec<XeroContact>> {
        #[derive(Deserialize, Default)]
        struct Resp {
            #[serde(rename = "Contacts", default)]
            contacts: Vec<ContactWire>,
        }
        let resp: Resp = self.send(self.get("Contacts")).await?;
        Ok(resp.contacts.into_iter().map(Into::into).collect())
    }

    /// POST `/Payments` allocating `payment` against `invoice_id`.
    pub async fn record_payment(
        &self,
        invoice_id: &str,
        payment: NewPayment,
    ) -> ArgoResult<XeroPayment> {
        let body = serde_json::json!({
            "Payments": [PaymentCreate {
                invoice: NamedIdRef::invoice(invoice_id),
                account: NamedIdRef::account(&payment.account_id),
                amount: cents_to_dollars(payment.amount_cents),
                date: payment.date.format("%Y-%m-%d").to_string(),
            }],
        });
        #[derive(Deserialize, Default)]
        struct Resp {
            #[serde(rename = "Payments", default)]
            payments: Vec<PaymentWire>,
        }
        let resp: Resp = self.post_json("Payments", &body).await?;
        resp.payments
            .into_iter()
            .next()
            .map(|p| p.into_payment(invoice_id))
            .ok_or_else(|| ArgoError::Provider("xero record_payment: empty Payments[]".into()))
    }

    /// GET `/Accounts`.
    pub async fn list_accounts(&self) -> ArgoResult<Vec<XeroAccount>> {
        #[derive(Deserialize, Default)]
        struct Resp {
            #[serde(rename = "Accounts", default)]
            accounts: Vec<AccountWire>,
        }
        let resp: Resp = self.send(self.get("Accounts")).await?;
        Ok(resp.accounts.into_iter().map(Into::into).collect())
    }

    fn get(&self, path: &str) -> reqwest::RequestBuilder {
        self.authed(self.client.get(self.url(path)))
    }

    fn url(&self, path: &str) -> String {
        format!("{}/{}", self.base_url.trim_end_matches('/'), path)
    }

    fn authed(&self, b: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        b.bearer_auth(&self.oauth_token)
            .header(TENANT_HEADER, &self.tenant_id)
            .header(reqwest::header::ACCEPT, "application/json")
    }

    async fn post_json<B: serde::Serialize, T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> ArgoResult<T> {
        let payload = serde_json::to_vec(body)
            .map_err(|e| ArgoError::Provider(format!("xero encode {path}: {e}")))?;
        let req = self
            .authed(self.client.post(self.url(path)))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(payload);
        self.send(req).await
    }

    async fn send<T: serde::de::DeserializeOwned>(
        &self,
        req: reqwest::RequestBuilder,
    ) -> ArgoResult<T> {
        let resp = req
            .send()
            .await
            .map_err(|e| ArgoError::Provider(format!("xero request: {e}")))?;
        decode_response(resp).await
    }

    async fn send_optional<T: serde::de::DeserializeOwned>(
        &self,
        req: reqwest::RequestBuilder,
    ) -> ArgoResult<Option<T>> {
        let resp = req
            .send()
            .await
            .map_err(|e| ArgoError::Provider(format!("xero request: {e}")))?;
        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        decode_response(resp).await.map(Some)
    }
}

async fn decode_response<T: serde::de::DeserializeOwned>(
    resp: reqwest::Response,
) -> ArgoResult<T> {
    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| ArgoError::Provider(format!("xero read body: {e}")))?;
    if status == reqwest::StatusCode::UNAUTHORIZED {
        return Err(ArgoError::Provider(format!(
            "xero HTTP 401 unauthorized: {text}"
        )));
    }
    if !status.is_success() {
        return Err(ArgoError::Provider(format!("xero HTTP {status}: {text}")));
    }
    serde_json::from_str(&text)
        .map_err(|e| ArgoError::Provider(format!("xero decode: {e}: {text}")))
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod client_tests;
