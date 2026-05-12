//! Xero accounting DTOs.
//!
//! Xero speaks decimal dollars on the wire; conversions to integer cents
//! happen at the serde boundary so callers see only `i64`. Dates are
//! tolerant: ISO-8601 (`2026-05-11`) and Microsoft `.NET` epoch
//! (`/Date(1747008000000+0000)/`) both decode to `NaiveDate`.

use chrono::NaiveDate;
use serde::{de::Error as DeError, Deserialize, Deserializer, Serialize};

/// Xero invoice lifecycle status. Unknown / new wire values fall through
/// to [`XeroInvoiceStatus::Other`] so the connector keeps working past
/// a Xero-side enum extension.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XeroInvoiceStatus {
    /// Saved but not yet submitted for approval.
    Draft,
    /// Submitted for approval; not yet finalised.
    Submitted,
    /// Approved and customer-facing; awaiting payment.
    Authorised,
    /// Fully paid.
    Paid,
    /// Voided.
    Voided,
    /// Unknown / unrecognised status string.
    Other(String),
}

impl XeroInvoiceStatus {
    /// Parse a Xero wire status. Unknown values become `Other(_)`.
    #[must_use]
    pub fn from_wire(raw: &str) -> Self {
        match raw {
            "DRAFT" => Self::Draft,
            "SUBMITTED" => Self::Submitted,
            "AUTHORISED" => Self::Authorised,
            "PAID" => Self::Paid,
            "VOIDED" => Self::Voided,
            other => Self::Other(other.to_string()),
        }
    }

    /// Serialise back to the Xero wire string.
    #[must_use]
    pub fn as_wire(&self) -> &str {
        match self {
            Self::Draft => "DRAFT",
            Self::Submitted => "SUBMITTED",
            Self::Authorised => "AUTHORISED",
            Self::Paid => "PAID",
            Self::Voided => "VOIDED",
            Self::Other(s) => s.as_str(),
        }
    }
}

/// Xero invoice (subset used by Argo).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XeroInvoice {
    /// Xero invoice id (GUID).
    pub invoice_id: String,
    /// Human-facing document number (e.g. `INV-1042`).
    pub invoice_number: String,
    /// Contact id this invoice bills.
    pub contact_id: String,
    /// Total amount in cents.
    pub total_cents: i64,
    /// Outstanding balance in cents.
    pub amount_due_cents: i64,
    /// Classified status.
    pub status: XeroInvoiceStatus,
    /// Due date, if set.
    pub due_date: Option<NaiveDate>,
}

/// Draft invoice payload accepted by [`crate::XeroConnector::create_invoice`].
#[derive(Debug, Clone)]
pub struct NewXeroInvoice {
    /// Xero contact id this invoice bills.
    pub contact_id: String,
    /// Line items on the invoice.
    pub line_items: Vec<XeroLineItem>,
    /// Due date.
    pub due_date: NaiveDate,
    /// Optional reference / PO number.
    pub reference: Option<String>,
    /// Status to create the invoice in. Defaults to `Draft`.
    pub status: Option<XeroInvoiceStatus>,
}

/// One invoice line item.
#[derive(Debug, Clone)]
pub struct XeroLineItem {
    /// Free-text description.
    pub description: String,
    /// Quantity.
    pub quantity: f64,
    /// Unit amount in cents.
    pub unit_amount_cents: i64,
    /// Optional chart-of-accounts code (e.g. `200`).
    pub account_code: Option<String>,
}

/// Xero contact (subset used by Argo).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XeroContact {
    /// Xero contact id.
    pub contact_id: String,
    /// Display name.
    pub name: String,
    /// Primary email, if recorded.
    pub email: Option<String>,
}

/// Recorded payment returned by [`crate::XeroConnector::record_payment`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XeroPayment {
    /// Xero payment id.
    pub payment_id: String,
    /// Invoice id the payment is allocated against.
    pub invoice_id: String,
    /// Amount in cents.
    pub amount_cents: i64,
    /// Date the payment was applied.
    pub date: NaiveDate,
    /// Bank/clearing account id the payment was booked to.
    pub account_id: String,
}

/// Draft payment payload accepted by [`crate::XeroConnector::record_payment`].
#[derive(Debug, Clone)]
pub struct NewPayment {
    /// Amount in cents.
    pub amount_cents: i64,
    /// Payment date.
    pub date: NaiveDate,
    /// Bank/clearing account id to book the payment to.
    pub account_id: String,
}

/// One row from the Xero chart of accounts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XeroAccount {
    /// Xero account id.
    pub account_id: String,
    /// Account code (e.g. `200`).
    pub code: String,
    /// Account name.
    pub name: String,
    /// Account class (e.g. `REVENUE`, `EXPENSE`).
    pub account_type: String,
}

/// Convert decimal dollars to integer cents (rounds to nearest).
#[must_use]
pub fn dollars_to_cents(amount: f64) -> i64 {
    (amount * 100.0).round() as i64
}

/// Convert integer cents to decimal dollars for outbound payloads.
#[must_use]
pub fn cents_to_dollars(cents: i64) -> f64 {
    cents as f64 / 100.0
}

pub(crate) fn deser_date<'de, D: Deserializer<'de>>(d: D) -> Result<NaiveDate, D::Error> {
    let raw = String::deserialize(d)?;
    parse_date(&raw).ok_or_else(|| D::Error::custom(format!("unrecognised xero date: {raw}")))
}

pub(crate) fn deser_date_opt<'de, D: Deserializer<'de>>(
    d: D,
) -> Result<Option<NaiveDate>, D::Error> {
    let raw: Option<String> = Option::deserialize(d)?;
    match raw {
        None => Ok(None),
        Some(s) if s.trim().is_empty() => Ok(None),
        Some(s) => parse_date(&s)
            .map(Some)
            .ok_or_else(|| D::Error::custom(format!("unrecognised xero date: {s}"))),
    }
}

fn parse_date(raw: &str) -> Option<NaiveDate> {
    if let Ok(d) = NaiveDate::parse_from_str(raw, "%Y-%m-%d") {
        return Some(d);
    }
    let body = raw.strip_prefix("/Date(")?.strip_suffix(")/")?;
    let millis: i64 = body.split(['+', '-']).next()?.parse().ok()?;
    chrono::DateTime::from_timestamp(millis.div_euclid(1000), 0).map(|dt| dt.date_naive())
}

#[derive(Debug, Deserialize)]
pub(crate) struct InvoiceWire {
    #[serde(rename = "InvoiceID")]
    pub invoice_id: String,
    #[serde(rename = "InvoiceNumber", default)]
    pub invoice_number: String,
    #[serde(rename = "Contact", default)]
    pub contact: Option<IdRefWire>,
    #[serde(rename = "Status", default)]
    pub status: String,
    #[serde(rename = "Total", default)]
    pub total: f64,
    #[serde(rename = "AmountDue", default)]
    pub amount_due: f64,
    #[serde(rename = "DueDate", default, deserialize_with = "deser_date_opt")]
    pub due_date: Option<NaiveDate>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct IdRefWire {
    #[serde(
        alias = "ContactID",
        alias = "InvoiceID",
        alias = "AccountID",
        default
    )]
    pub id: String,
}

impl From<InvoiceWire> for XeroInvoice {
    fn from(w: InvoiceWire) -> Self {
        Self {
            invoice_id: w.invoice_id,
            invoice_number: w.invoice_number,
            contact_id: w.contact.map(|c| c.id).unwrap_or_default(),
            total_cents: dollars_to_cents(w.total),
            amount_due_cents: dollars_to_cents(w.amount_due),
            status: XeroInvoiceStatus::from_wire(&w.status),
            due_date: w.due_date,
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct ContactWire {
    #[serde(rename = "ContactID")]
    pub contact_id: String,
    #[serde(rename = "Name", default)]
    pub name: String,
    #[serde(rename = "EmailAddress", default)]
    pub email: Option<String>,
}

impl From<ContactWire> for XeroContact {
    fn from(w: ContactWire) -> Self {
        Self {
            contact_id: w.contact_id,
            name: w.name,
            email: w.email.filter(|s| !s.trim().is_empty()),
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct PaymentWire {
    #[serde(rename = "PaymentID")]
    pub payment_id: String,
    #[serde(rename = "Invoice", default)]
    pub invoice: Option<IdRefWire>,
    #[serde(rename = "Account", default)]
    pub account: Option<IdRefWire>,
    #[serde(rename = "Amount", default)]
    pub amount: f64,
    #[serde(rename = "Date", deserialize_with = "deser_date")]
    pub date: NaiveDate,
}

impl PaymentWire {
    pub(crate) fn into_payment(self, fallback_invoice_id: &str) -> XeroPayment {
        let invoice_id = self
            .invoice
            .map(|i| i.id)
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| fallback_invoice_id.to_string());
        XeroPayment {
            payment_id: self.payment_id,
            invoice_id,
            amount_cents: dollars_to_cents(self.amount),
            date: self.date,
            account_id: self.account.map(|a| a.id).unwrap_or_default(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct AccountWire {
    #[serde(rename = "AccountID")]
    pub account_id: String,
    #[serde(rename = "Code", default)]
    pub code: String,
    #[serde(rename = "Name", default)]
    pub name: String,
    #[serde(rename = "Type", default)]
    pub account_type: String,
}

impl From<AccountWire> for XeroAccount {
    fn from(w: AccountWire) -> Self {
        Self {
            account_id: w.account_id,
            code: w.code,
            name: w.name,
            account_type: w.account_type,
        }
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct InvoiceCreate<'a> {
    #[serde(rename = "Type")]
    pub kind: &'static str,
    #[serde(rename = "Contact")]
    pub contact: NamedIdRef<'a>,
    #[serde(rename = "DueDate")]
    pub due_date: String,
    #[serde(rename = "LineItems")]
    pub line_items: Vec<LineItemOut>,
    #[serde(rename = "Reference", skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    #[serde(rename = "Status")]
    pub status: &'a str,
}

#[derive(Debug, Serialize)]
pub(crate) struct NamedIdRef<'a> {
    #[serde(rename = "ContactID", skip_serializing_if = "Option::is_none")]
    pub contact_id: Option<&'a str>,
    #[serde(rename = "InvoiceID", skip_serializing_if = "Option::is_none")]
    pub invoice_id: Option<&'a str>,
    #[serde(rename = "AccountID", skip_serializing_if = "Option::is_none")]
    pub account_id: Option<&'a str>,
}

impl<'a> NamedIdRef<'a> {
    pub(crate) fn contact(id: &'a str) -> Self {
        Self { contact_id: Some(id), invoice_id: None, account_id: None }
    }
    pub(crate) fn invoice(id: &'a str) -> Self {
        Self { contact_id: None, invoice_id: Some(id), account_id: None }
    }
    pub(crate) fn account(id: &'a str) -> Self {
        Self { contact_id: None, invoice_id: None, account_id: Some(id) }
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct LineItemOut {
    #[serde(rename = "Description")]
    pub description: String,
    #[serde(rename = "Quantity")]
    pub quantity: f64,
    #[serde(rename = "UnitAmount")]
    pub unit_amount: f64,
    #[serde(rename = "AccountCode", skip_serializing_if = "Option::is_none")]
    pub account_code: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct PaymentCreate<'a> {
    #[serde(rename = "Invoice")]
    pub invoice: NamedIdRef<'a>,
    #[serde(rename = "Account")]
    pub account: NamedIdRef<'a>,
    #[serde(rename = "Amount")]
    pub amount: f64,
    #[serde(rename = "Date")]
    pub date: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dollars_to_cents_rounds_to_nearest() {
        assert_eq!(dollars_to_cents(12.34), 1234);
        assert_eq!(dollars_to_cents(0.005), 1);
        assert_eq!(dollars_to_cents(0.004), 0);
    }

    #[test]
    fn invoice_status_round_trips_known_values() {
        for raw in ["DRAFT", "SUBMITTED", "AUTHORISED", "PAID", "VOIDED"] {
            assert_eq!(XeroInvoiceStatus::from_wire(raw).as_wire(), raw);
        }
        match XeroInvoiceStatus::from_wire("DELETED") {
            XeroInvoiceStatus::Other(s) => assert_eq!(s, "DELETED"),
            other => panic!("expected Other, got {other:?}"),
        }
    }

    #[test]
    fn parse_date_accepts_iso_and_dotnet() {
        assert_eq!(
            parse_date("2026-05-11"),
            Some(NaiveDate::from_ymd_opt(2026, 5, 11).unwrap())
        );
        // 2024-03-14 00:00:00 UTC = 1710374400000 ms.
        assert_eq!(
            parse_date("/Date(1710374400000+0000)/"),
            Some(NaiveDate::from_ymd_opt(2024, 3, 14).unwrap())
        );
        assert_eq!(
            parse_date("/Date(1710374400000)/"),
            Some(NaiveDate::from_ymd_opt(2024, 3, 14).unwrap())
        );
        assert!(parse_date("not a date").is_none());
    }
}
