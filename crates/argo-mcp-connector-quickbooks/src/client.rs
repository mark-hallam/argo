//! QuickBooks Online REST client.
//!
//! Constructs typed requests against the `v3/company/{realm_id}/...` surface,
//! authenticates with the OAuth2 bearer token held by [`QuickBooksAuth`], and
//! transparently refreshes the access token once on a 401 before propagating.

use crate::auth::QuickBooksAuth;
use crate::types::{
    cents_to_dollars, AccountWire, CustomerWire, InvoiceCreate, InvoiceLineOut, InvoiceWire,
    PurchaseCreate, PurchaseLineOut, PurchaseWire, QbAccount, QbCustomer, QbExpense,
    QbExpenseDraft, QbInvoice, QbInvoiceDraft, RefOut,
};
use argo_types::{ArgoError, ArgoResult};
use chrono::{DateTime, Utc};
use serde::Deserialize;

/// Which QuickBooks Online host to talk to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuickBooksHost {
    /// `https://sandbox-quickbooks.api.intuit.com` — non-production data.
    Sandbox,
    /// `https://quickbooks.api.intuit.com` — live company books.
    Production,
}

impl QuickBooksHost {
    /// Base URL prefix (no trailing slash).
    #[must_use]
    pub fn base_url(self) -> &'static str {
        match self {
            Self::Sandbox => "https://sandbox-quickbooks.api.intuit.com",
            Self::Production => "https://quickbooks.api.intuit.com",
        }
    }
}

/// QuickBooks Online connector.
pub struct QuickBooksConnector {
    client: reqwest::Client,
    auth: QuickBooksAuth,
    realm_id: String,
    base_url: String,
}

impl QuickBooksConnector {
    /// Build a connector for `realm_id` against the chosen host.
    pub fn new(auth: QuickBooksAuth, realm_id: String, host: QuickBooksHost) -> ArgoResult<Self> {
        let client = reqwest::Client::builder()
            .user_agent(concat!(
                "argo-mcp-connector-quickbooks/",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .map_err(|e| ArgoError::Provider(format!("quickbooks http client: {e}")))?;
        Ok(Self {
            client,
            auth,
            realm_id,
            base_url: host.base_url().to_string(),
        })
    }

    /// Override the API base URL — used by tests to point at a wiremock server.
    #[must_use]
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into().trim_end_matches('/').to_string();
        self
    }

    /// Reference to the auth holder. Useful for tests asserting refresh state.
    #[must_use]
    pub fn auth(&self) -> &QuickBooksAuth {
        &self.auth
    }

    /// GET `?query=SELECT * FROM Customer ...` and decode into [`QbCustomer`]s.
    pub async fn list_customers(&self, query: Option<&str>) -> ArgoResult<Vec<QbCustomer>> {
        let sql = match query {
            Some(filter) => format!("SELECT * FROM Customer WHERE {filter}"),
            None => "SELECT * FROM Customer".to_string(),
        };
        #[derive(Deserialize)]
        struct Resp {
            #[serde(rename = "QueryResponse")]
            query_response: CustomerQuery,
        }
        #[derive(Deserialize, Default)]
        struct CustomerQuery {
            #[serde(rename = "Customer", default)]
            customer: Vec<CustomerWire>,
        }
        let resp: Resp = self.get_query(&sql).await?;
        Ok(resp.query_response.customer.into_iter().map(Into::into).collect())
    }

    /// POST `/invoice` with the given draft. Returns the persisted invoice
    /// including a deep-link `web_url`.
    pub async fn create_invoice(&self, draft: QbInvoiceDraft) -> ArgoResult<QbInvoice> {
        let body = InvoiceCreate {
            customer_ref: RefOut { value: &draft.customer_id },
            line: draft
                .line_items
                .into_iter()
                .map(|l| InvoiceLineOut {
                    detail_type: "SalesItemLineDetail",
                    amount: cents_to_dollars(l.amount_cents),
                    description: l.description,
                    sales_item_line_detail: serde_json::json!({}),
                })
                .collect(),
            due_date: draft.due_date.format("%Y-%m-%d").to_string(),
        };
        #[derive(Deserialize)]
        struct Resp {
            #[serde(rename = "Invoice")]
            invoice: InvoiceWire,
        }
        let resp: Resp = self.post_json("invoice", &body).await?;
        let web_url = format!(
            "{}/app/invoice?txnId={}",
            self.base_url.trim_end_matches('/'),
            resp.invoice.id
        );
        Ok(resp.invoice.into_invoice(web_url))
    }

    /// GET `?query=SELECT * FROM Invoice ...`, optionally filtered by
    /// `MetaData.LastUpdatedTime >= since`.
    pub async fn list_invoices(
        &self,
        since: Option<DateTime<Utc>>,
    ) -> ArgoResult<Vec<QbInvoice>> {
        let sql = match since {
            Some(ts) => format!(
                "SELECT * FROM Invoice WHERE MetaData.LastUpdatedTime >= '{}'",
                ts.format("%Y-%m-%dT%H:%M:%S%z")
            ),
            None => "SELECT * FROM Invoice".to_string(),
        };
        #[derive(Deserialize)]
        struct Resp {
            #[serde(rename = "QueryResponse")]
            query_response: InvoiceQuery,
        }
        #[derive(Deserialize, Default)]
        struct InvoiceQuery {
            #[serde(rename = "Invoice", default)]
            invoice: Vec<InvoiceWire>,
        }
        let resp: Resp = self.get_query(&sql).await?;
        let base = self.base_url.trim_end_matches('/').to_string();
        Ok(resp
            .query_response
            .invoice
            .into_iter()
            .map(|wire| {
                let id = wire.id.clone();
                wire.into_invoice(format!("{base}/app/invoice?txnId={id}"))
            })
            .collect())
    }

    /// POST `/purchase` (QBO's expense entity) with the given draft.
    pub async fn record_expense(&self, draft: QbExpenseDraft) -> ArgoResult<QbExpense> {
        let body = PurchaseCreate {
            account_ref: RefOut { value: &draft.account_id },
            entity_ref: draft.vendor_id.as_deref().map(|v| RefOut { value: v }),
            total_amt: cents_to_dollars(draft.amount_cents),
            txn_date: draft.txn_date.format("%Y-%m-%d").to_string(),
            payment_type: "Cash",
            line: vec![PurchaseLineOut {
                detail_type: "AccountBasedExpenseLineDetail",
                amount: cents_to_dollars(draft.amount_cents),
                account_based_expense_line_detail: serde_json::json!({
                    "AccountRef": { "value": draft.account_id }
                }),
            }],
            private_note: draft.memo,
        };
        #[derive(Deserialize)]
        struct Resp {
            #[serde(rename = "Purchase")]
            purchase: PurchaseWire,
        }
        let resp: Resp = self.post_json("purchase", &body).await?;
        Ok(resp.purchase.into())
    }

    /// GET `?query=SELECT * FROM Account`.
    pub async fn chart_of_accounts(&self) -> ArgoResult<Vec<QbAccount>> {
        #[derive(Deserialize)]
        struct Resp {
            #[serde(rename = "QueryResponse")]
            query_response: AccountQuery,
        }
        #[derive(Deserialize, Default)]
        struct AccountQuery {
            #[serde(rename = "Account", default)]
            account: Vec<AccountWire>,
        }
        let resp: Resp = self.get_query("SELECT * FROM Account").await?;
        Ok(resp.query_response.account.into_iter().map(Into::into).collect())
    }

    async fn get_query<T: serde::de::DeserializeOwned>(&self, sql: &str) -> ArgoResult<T> {
        let url = format!(
            "{}/v3/company/{}/query",
            self.base_url.trim_end_matches('/'),
            self.realm_id
        );
        self.send_with_refresh(|token| {
            self.client
                .get(&url)
                .bearer_auth(token)
                .header(reqwest::header::ACCEPT, "application/json")
                .query(&[("query", sql), ("minorversion", "65")])
        })
        .await
    }

    async fn post_json<B: serde::Serialize, T: serde::de::DeserializeOwned>(
        &self,
        entity: &str,
        body: &B,
    ) -> ArgoResult<T> {
        let url = format!(
            "{}/v3/company/{}/{}",
            self.base_url.trim_end_matches('/'),
            self.realm_id,
            entity
        );
        let payload = serde_json::to_vec(body)
            .map_err(|e| ArgoError::Provider(format!("quickbooks encode {entity}: {e}")))?;
        self.send_with_refresh(|token| {
            self.client
                .post(&url)
                .bearer_auth(token)
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .header(reqwest::header::ACCEPT, "application/json")
                .query(&[("minorversion", "65")])
                .body(payload.clone())
        })
        .await
    }

    async fn send_with_refresh<F, T>(&self, build_req: F) -> ArgoResult<T>
    where
        F: Fn(&str) -> reqwest::RequestBuilder,
        T: serde::de::DeserializeOwned,
    {
        let token = match self.auth.access_token() {
            Some(t) => t,
            None => self
                .auth
                .refresh(&self.client)
                .await
                .map_err(ArgoError::Provider)?,
        };
        let resp = build_req(&token)
            .send()
            .await
            .map_err(|e| ArgoError::Provider(format!("quickbooks request: {e}")))?;
        if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
            let new_token = self
                .auth
                .refresh(&self.client)
                .await
                .map_err(ArgoError::Provider)?;
            let retry = build_req(&new_token)
                .send()
                .await
                .map_err(|e| ArgoError::Provider(format!("quickbooks retry: {e}")))?;
            return decode_response(retry).await;
        }
        decode_response(resp).await
    }
}

async fn decode_response<T: serde::de::DeserializeOwned>(
    resp: reqwest::Response,
) -> ArgoResult<T> {
    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| ArgoError::Provider(format!("quickbooks read body: {e}")))?;
    if !status.is_success() {
        return Err(ArgoError::Provider(format!(
            "quickbooks HTTP {status}: {text}"
        )));
    }
    serde_json::from_str(&text)
        .map_err(|e| ArgoError::Provider(format!("quickbooks decode: {e}: {text}")))
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod client_tests;
