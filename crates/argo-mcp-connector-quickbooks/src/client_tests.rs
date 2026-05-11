//! Wiremock-backed unit tests for `QuickBooksConnector`.
//!
//! Six scenarios per the brief:
//!  1. `list_customers` round-trips a `QueryResponse.Customer` array.
//!  2. `create_invoice` posts a JSON `Line[]` body and decodes the response.
//!  3. `record_expense` posts a QBO `Purchase` and decodes the response.
//!  4. `chart_of_accounts` round-trips a `QueryResponse.Account` array.
//!  5. A 401 triggers exactly one auth refresh, then the retried call wins.
//!  6. `QuickBooksHost::{Sandbox,Production}` route to distinct base URLs.

use super::*;
use crate::auth::QuickBooksAuth;
use crate::types::{QbDocStatus, QbExpenseDraft, QbInvoiceDraft, QbLine};
use chrono::NaiveDate;
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

const REALM: &str = "9341452094938041";

fn connector(server: &MockServer) -> QuickBooksConnector {
    let auth = QuickBooksAuth::new("cid".into(), "secret".into(), "rtok".into())
        .with_token_url(format!("{}/oauth/token", server.uri()));
    auth.set_access_token("seed-access-token");
    QuickBooksConnector::new(auth, REALM.into(), QuickBooksHost::Sandbox)
        .unwrap()
        .with_base_url(server.uri())
}

#[tokio::test]
async fn list_customers_query_round_trips() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/v3/company/{REALM}/query")))
        .and(query_param("query", "SELECT * FROM Customer"))
        .and(header("authorization", "Bearer seed-access-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "QueryResponse": {
                "Customer": [
                    {
                        "Id": "1",
                        "DisplayName": "Acme Co.",
                        "PrimaryEmailAddr": { "Address": "ops@acme.test" },
                        "Balance": 125.5
                    },
                    {
                        "Id": "2",
                        "DisplayName": "Initech",
                        "Balance": 0.0
                    }
                ]
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let qb = connector(&server);
    let customers = qb.list_customers(None).await.unwrap();
    assert_eq!(customers.len(), 2);
    assert_eq!(customers[0].id, "1");
    assert_eq!(customers[0].display_name, "Acme Co.");
    assert_eq!(customers[0].email.as_deref(), Some("ops@acme.test"));
    assert_eq!(customers[0].balance_cents, 12550);
    assert_eq!(customers[1].email, None);
}

#[tokio::test]
async fn create_invoice_posts_line_items_and_returns_web_url() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/v3/company/{REALM}/invoice")))
        .and(header("authorization", "Bearer seed-access-token"))
        .and(header("content-type", "application/json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "Invoice": {
                "Id": "42",
                "DocNumber": "INV-42",
                "TotalAmt": 199.99,
                "EmailStatus": "Sent",
                "Balance": 199.99
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let qb = connector(&server);
    let draft = QbInvoiceDraft {
        customer_id: "C1".into(),
        line_items: vec![
            QbLine {
                description: "Consulting".into(),
                amount_cents: 15000,
            },
            QbLine {
                description: "Hosting".into(),
                amount_cents: 4999,
            },
        ],
        due_date: NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
    };
    let invoice = qb.create_invoice(draft).await.unwrap();
    assert_eq!(invoice.id, "42");
    assert_eq!(invoice.doc_number, "INV-42");
    assert_eq!(invoice.total_cents, 19999);
    assert_eq!(invoice.status, QbDocStatus::Sent);
    assert!(invoice.web_url.contains("/app/invoice?txnId=42"));
    assert!(invoice.web_url.starts_with(&server.uri()));
}

#[tokio::test]
async fn record_expense_posts_purchase_and_decodes() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/v3/company/{REALM}/purchase")))
        .and(header("authorization", "Bearer seed-access-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "Purchase": {
                "Id": "777",
                "TotalAmt": 48.21,
                "AccountRef": { "value": "55" },
                "TxnDate": "2026-05-11"
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let qb = connector(&server);
    let draft = QbExpenseDraft {
        account_id: "55".into(),
        vendor_id: Some("V1".into()),
        amount_cents: 4821,
        memo: Some("AWS".into()),
        txn_date: NaiveDate::from_ymd_opt(2026, 5, 11).unwrap(),
    };
    let expense = qb.record_expense(draft).await.unwrap();
    assert_eq!(expense.id, "777");
    assert_eq!(expense.amount_cents, 4821);
    assert_eq!(expense.account_id, "55");
    assert_eq!(
        expense.txn_date,
        NaiveDate::from_ymd_opt(2026, 5, 11).unwrap()
    );
}

#[tokio::test]
async fn chart_of_accounts_lists_accounts() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/v3/company/{REALM}/query")))
        .and(query_param("query", "SELECT * FROM Account"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "QueryResponse": {
                "Account": [
                    {"Id": "1", "Name": "Checking", "AccountType": "Bank", "CurrentBalance": 10000.0},
                    {"Id": "2", "Name": "Office Supplies", "AccountType": "Expense", "CurrentBalance": 1234.5}
                ]
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let qb = connector(&server);
    let accounts = qb.chart_of_accounts().await.unwrap();
    assert_eq!(accounts.len(), 2);
    assert_eq!(accounts[0].name, "Checking");
    assert_eq!(accounts[0].account_type, "Bank");
    assert_eq!(accounts[0].balance_cents, 1_000_000);
    assert_eq!(accounts[1].balance_cents, 123_450);
}

#[tokio::test]
async fn unauthorized_triggers_one_refresh_then_succeeds() {
    let server = MockServer::start().await;
    // First query call: 401 with the seed token.
    Mock::given(method("GET"))
        .and(path(format!("/v3/company/{REALM}/query")))
        .and(header("authorization", "Bearer seed-access-token"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "fault": { "type": "AUTHENTICATION", "error": [{"code": "3200"}] }
        })))
        .expect(1)
        .mount(&server)
        .await;
    // OAuth refresh endpoint emits a new bearer.
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "refreshed-token",
            "token_type": "Bearer",
            "expires_in": 3600,
            "refresh_token": "rtok-new"
        })))
        .expect(1)
        .mount(&server)
        .await;
    // Retry with the refreshed bearer succeeds.
    Mock::given(method("GET"))
        .and(path(format!("/v3/company/{REALM}/query")))
        .and(header("authorization", "Bearer refreshed-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "QueryResponse": {
                "Customer": [{"Id": "9", "DisplayName": "After-refresh", "Balance": 0.0}]
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let qb = connector(&server);
    let customers = qb.list_customers(None).await.unwrap();
    assert_eq!(customers.len(), 1);
    assert_eq!(customers[0].display_name, "After-refresh");
    assert_eq!(qb.auth().access_token().as_deref(), Some("refreshed-token"));
}

#[tokio::test]
async fn sandbox_and_production_route_to_distinct_hosts() {
    // No HTTP is issued in this test — we only inspect the base URLs the
    // enum yields so a regression in host routing (e.g. swapping the two)
    // would be caught at the unit level rather than via a smoke run.
    assert_eq!(
        QuickBooksHost::Sandbox.base_url(),
        "https://sandbox-quickbooks.api.intuit.com"
    );
    assert_eq!(
        QuickBooksHost::Production.base_url(),
        "https://quickbooks.api.intuit.com"
    );
    assert_ne!(
        QuickBooksHost::Sandbox.base_url(),
        QuickBooksHost::Production.base_url()
    );

    let auth_a = QuickBooksAuth::new("cid".into(), "secret".into(), "rtok".into());
    let prod = QuickBooksConnector::new(auth_a, REALM.into(), QuickBooksHost::Production).unwrap();
    assert_eq!(prod.base_url, "https://quickbooks.api.intuit.com");

    let auth_b = QuickBooksAuth::new("cid".into(), "secret".into(), "rtok".into());
    let sandbox = QuickBooksConnector::new(auth_b, REALM.into(), QuickBooksHost::Sandbox).unwrap();
    assert_eq!(sandbox.base_url, "https://sandbox-quickbooks.api.intuit.com");
}
