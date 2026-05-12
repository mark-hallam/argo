//! Wiremock-backed unit tests for `XeroConnector`.
//!
//! Seven scenarios per the brief, each verifying the `xero-tenant-id`
//! header is present on the wire and (where applicable) bearer auth:
//!  1. `list_invoices(Some(Authorised))` filters via the `where` param.
//!  2. `get_invoice` decodes a single-element `Invoices[]` body.
//!  3. `get_invoice` returns `Ok(None)` on a 404.
//!  4. `create_invoice` posts the Xero envelope and decodes the response.
//!  5. `list_contacts` round-trips a `Contacts[]` array.
//!  6. `record_payment` posts a `Payments[]` envelope and decodes.
//!  7. `list_accounts` round-trips a `Accounts[]` array; a 401 surfaces
//!     as a typed `ArgoError::Provider` with an explicit `HTTP 401` tag.

use super::*;
use crate::types::{NewPayment, NewXeroInvoice, XeroInvoiceStatus, XeroLineItem};
use chrono::NaiveDate;
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

const TOKEN: &str = "test-oauth-token";
const TENANT: &str = "11111111-2222-3333-4444-555555555555";
const INVOICE_ID: &str = "ABC-INV-1";

fn connector(server: &MockServer) -> XeroConnector {
    XeroConnector::new(TOKEN.to_string(), TENANT.to_string()).with_base_url(server.uri())
}

#[tokio::test]
async fn list_invoices_filters_by_status() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/Invoices"))
        .and(header("authorization", format!("Bearer {TOKEN}").as_str()))
        .and(header("xero-tenant-id", TENANT))
        .and(query_param("where", "Status==\"AUTHORISED\""))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "Invoices": [
                {
                    "InvoiceID": "INV-A",
                    "InvoiceNumber": "INV-101",
                    "Contact": {"ContactID": "C-1"},
                    "Status": "AUTHORISED",
                    "Total": 199.99,
                    "AmountDue": 199.99,
                    "DueDate": "/Date(1710374400000+0000)/"
                }
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let xero = connector(&server);
    let invoices = xero
        .list_invoices(Some(XeroInvoiceStatus::Authorised))
        .await
        .expect("list_invoices succeeds");
    assert_eq!(invoices.len(), 1);
    assert_eq!(invoices[0].invoice_id, "INV-A");
    assert_eq!(invoices[0].invoice_number, "INV-101");
    assert_eq!(invoices[0].contact_id, "C-1");
    assert_eq!(invoices[0].status, XeroInvoiceStatus::Authorised);
    assert_eq!(invoices[0].total_cents, 19999);
    assert_eq!(invoices[0].amount_due_cents, 19999);
    assert_eq!(
        invoices[0].due_date,
        Some(NaiveDate::from_ymd_opt(2024, 3, 14).unwrap())
    );
}

#[tokio::test]
async fn get_invoice_decodes_single_envelope() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/Invoices/{INVOICE_ID}")))
        .and(header("xero-tenant-id", TENANT))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "Invoices": [{
                "InvoiceID": INVOICE_ID,
                "InvoiceNumber": "INV-42",
                "Contact": {"ContactID": "C-7"},
                "Status": "PAID",
                "Total": 50.00,
                "AmountDue": 0.00,
                "DueDate": "2026-05-11"
            }]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let xero = connector(&server);
    let inv = xero.get_invoice(INVOICE_ID).await.unwrap().expect("Some");
    assert_eq!(inv.invoice_id, INVOICE_ID);
    assert_eq!(inv.status, XeroInvoiceStatus::Paid);
    assert_eq!(inv.amount_due_cents, 0);
}

#[tokio::test]
async fn get_invoice_returns_none_on_404() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/Invoices/{INVOICE_ID}")))
        .and(header("xero-tenant-id", TENANT))
        .respond_with(ResponseTemplate::new(404).set_body_string("Not found"))
        .expect(1)
        .mount(&server)
        .await;

    let xero = connector(&server);
    let inv = xero.get_invoice(INVOICE_ID).await.unwrap();
    assert!(inv.is_none(), "404 must map to Ok(None), got {inv:?}");
}

#[tokio::test]
async fn create_invoice_posts_envelope_and_decodes() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/Invoices"))
        .and(header("authorization", format!("Bearer {TOKEN}").as_str()))
        .and(header("xero-tenant-id", TENANT))
        .and(header("content-type", "application/json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "Invoices": [{
                "InvoiceID": "NEW-INV-1",
                "InvoiceNumber": "INV-300",
                "Contact": {"ContactID": "C-9"},
                "Status": "DRAFT",
                "Total": 250.00,
                "AmountDue": 250.00,
                "DueDate": "2026-06-01"
            }]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let xero = connector(&server);
    let draft = NewXeroInvoice {
        contact_id: "C-9".into(),
        line_items: vec![
            XeroLineItem {
                description: "Consulting".into(),
                quantity: 5.0,
                unit_amount_cents: 30000,
                account_code: Some("200".into()),
            },
            XeroLineItem {
                description: "Hosting".into(),
                quantity: 1.0,
                unit_amount_cents: 10000,
                account_code: None,
            },
        ],
        due_date: NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
        reference: Some("PO-77".into()),
        status: None,
    };
    let inv = xero.create_invoice(draft).await.expect("create succeeds");
    assert_eq!(inv.invoice_id, "NEW-INV-1");
    assert_eq!(inv.invoice_number, "INV-300");
    assert_eq!(inv.total_cents, 25000);
    assert_eq!(inv.status, XeroInvoiceStatus::Draft);
}

#[tokio::test]
async fn list_contacts_round_trips() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/Contacts"))
        .and(header("xero-tenant-id", TENANT))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "Contacts": [
                {"ContactID": "C-1", "Name": "Acme Co.", "EmailAddress": "ops@acme.test"},
                {"ContactID": "C-2", "Name": "Initech", "EmailAddress": ""}
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let xero = connector(&server);
    let contacts = xero.list_contacts().await.expect("list_contacts succeeds");
    assert_eq!(contacts.len(), 2);
    assert_eq!(contacts[0].contact_id, "C-1");
    assert_eq!(contacts[0].name, "Acme Co.");
    assert_eq!(contacts[0].email.as_deref(), Some("ops@acme.test"));
    assert_eq!(contacts[1].email, None, "empty email coerces to None");
}

#[tokio::test]
async fn record_payment_posts_envelope_and_decodes() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/Payments"))
        .and(header("xero-tenant-id", TENANT))
        .and(header("content-type", "application/json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "Payments": [{
                "PaymentID": "PAY-1",
                "Invoice": {"InvoiceID": INVOICE_ID},
                "Account": {"AccountID": "ACC-100"},
                "Amount": 100.0,
                "Date": "2026-05-11"
            }]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let xero = connector(&server);
    let payment = xero
        .record_payment(
            INVOICE_ID,
            NewPayment {
                amount_cents: 10000,
                date: NaiveDate::from_ymd_opt(2026, 5, 11).unwrap(),
                account_id: "ACC-100".into(),
            },
        )
        .await
        .expect("record_payment succeeds");
    assert_eq!(payment.payment_id, "PAY-1");
    assert_eq!(payment.invoice_id, INVOICE_ID);
    assert_eq!(payment.amount_cents, 10000);
    assert_eq!(payment.account_id, "ACC-100");
    assert_eq!(
        payment.date,
        NaiveDate::from_ymd_opt(2026, 5, 11).unwrap()
    );
}

#[tokio::test]
async fn list_accounts_round_trips_and_unauthorized_typed_error() {
    let server = MockServer::start().await;
    // Happy path on /Accounts asserts tenant-id header propagates.
    Mock::given(method("GET"))
        .and(path("/Accounts"))
        .and(header("authorization", format!("Bearer {TOKEN}").as_str()))
        .and(header("xero-tenant-id", TENANT))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "Accounts": [
                {"AccountID": "A-1", "Code": "200", "Name": "Sales", "Type": "REVENUE"},
                {"AccountID": "A-2", "Code": "400", "Name": "Office Supplies", "Type": "EXPENSE"}
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;
    let xero = connector(&server);
    let accounts = xero.list_accounts().await.expect("list_accounts succeeds");
    assert_eq!(accounts.len(), 2);
    assert_eq!(accounts[0].code, "200");
    assert_eq!(accounts[0].account_type, "REVENUE");
    assert_eq!(accounts[1].name, "Office Supplies");

    // Separate server: a 401 on /Contacts surfaces as a typed Provider
    // error carrying an explicit `HTTP 401` tag.
    let unauth = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/Contacts"))
        .respond_with(ResponseTemplate::new(401).set_body_string("Unauthorized"))
        .expect(1)
        .mount(&unauth)
        .await;
    let bad = connector(&unauth);
    let err = bad
        .list_contacts()
        .await
        .expect_err("401 must propagate as Err");
    match err {
        argo_types::ArgoError::Provider(msg) => {
            assert!(
                msg.contains("HTTP 401"),
                "401 message must tag HTTP 401, got: {msg}"
            );
        }
        other => panic!("expected ArgoError::Provider, got {other:?}"),
    }
}
