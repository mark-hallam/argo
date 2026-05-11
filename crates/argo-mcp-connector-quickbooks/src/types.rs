//! QuickBooks Online DTOs.
//!
//! QBO speaks decimal dollars on the wire (e.g. `12.34`); this module
//! converts at the boundary so callers see only integer cents (`i64`).

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// QuickBooks Online customer (subset of fields used by Argo).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QbCustomer {
    /// QBO id.
    pub id: String,
    /// Display name shown in the QBO UI.
    pub display_name: String,
    /// Primary email, if recorded.
    pub email: Option<String>,
    /// Open balance in cents.
    pub balance_cents: i64,
}

/// Draft invoice payload.
#[derive(Debug, Clone)]
pub struct QbInvoiceDraft {
    /// QBO customer id this invoice bills.
    pub customer_id: String,
    /// Line items on the invoice.
    pub line_items: Vec<QbLine>,
    /// Due date.
    pub due_date: NaiveDate,
}

/// One invoice line item.
#[derive(Debug, Clone)]
pub struct QbLine {
    /// Free-text description.
    pub description: String,
    /// Amount in cents.
    pub amount_cents: i64,
}

/// QBO document status for invoices.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QbDocStatus {
    /// Not yet sent.
    Draft,
    /// Sent, awaiting payment.
    Sent,
    /// Fully paid.
    Paid,
    /// Voided.
    Voided,
    /// Unknown / unrecognised status string.
    Other(String),
}

impl QbDocStatus {
    /// Parse a QBO status string. Unknown values become `Other(_)`.
    #[must_use]
    pub fn from_qbo(raw: &str) -> Self {
        match raw {
            "Draft" => Self::Draft,
            "Sent" | "Pending" => Self::Sent,
            "Paid" => Self::Paid,
            "Voided" => Self::Voided,
            other => Self::Other(other.to_string()),
        }
    }
}

/// QBO invoice (subset of fields used by Argo).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QbInvoice {
    /// QBO id.
    pub id: String,
    /// Human-facing document number (e.g. `INV-1042`).
    pub doc_number: String,
    /// Total amount in cents.
    pub total_cents: i64,
    /// Classified status (`EmailStatus` + paid heuristic).
    pub status: QbDocStatus,
    /// Direct link to the invoice in QBO.
    pub web_url: String,
}

/// Draft expense (QBO `Purchase` entity).
#[derive(Debug, Clone)]
pub struct QbExpenseDraft {
    /// QBO account id (expense or asset).
    pub account_id: String,
    /// Optional QBO vendor id.
    pub vendor_id: Option<String>,
    /// Amount in cents.
    pub amount_cents: i64,
    /// Free-text memo.
    pub memo: Option<String>,
    /// Transaction date.
    pub txn_date: NaiveDate,
}

/// Recorded expense returned after `record_expense`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QbExpense {
    /// QBO id.
    pub id: String,
    /// Amount in cents.
    pub amount_cents: i64,
    /// Account id the expense was booked against.
    pub account_id: String,
    /// Transaction date as returned by QBO.
    pub txn_date: NaiveDate,
}

/// One row from the chart of accounts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QbAccount {
    /// QBO id.
    pub id: String,
    /// Account name.
    pub name: String,
    /// QBO `AccountType` (e.g. `Bank`, `Expense`, `Income`).
    pub account_type: String,
    /// Current balance in cents.
    pub balance_cents: i64,
}

/// Convert decimal dollars (`12.34`) to integer cents. Rounds to the nearest
/// cent per QBO's documented convention.
#[must_use]
pub fn dollars_to_cents(amount: f64) -> i64 {
    (amount * 100.0).round() as i64
}

/// Convert integer cents back to decimal dollars for outbound payloads.
#[must_use]
pub fn cents_to_dollars(cents: i64) -> f64 {
    cents as f64 / 100.0
}

#[derive(Debug, Deserialize)]
pub(crate) struct CustomerWire {
    #[serde(rename = "Id")]
    pub id: String,
    #[serde(rename = "DisplayName")]
    pub display_name: String,
    #[serde(rename = "PrimaryEmailAddr")]
    pub primary_email: Option<EmailAddr>,
    #[serde(rename = "Balance", default)]
    pub balance: f64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct EmailAddr {
    #[serde(rename = "Address")]
    pub address: String,
}

impl From<CustomerWire> for QbCustomer {
    fn from(wire: CustomerWire) -> Self {
        Self {
            id: wire.id,
            display_name: wire.display_name,
            email: wire.primary_email.map(|e| e.address),
            balance_cents: dollars_to_cents(wire.balance),
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct InvoiceWire {
    #[serde(rename = "Id")]
    pub id: String,
    #[serde(rename = "DocNumber", default)]
    pub doc_number: String,
    #[serde(rename = "TotalAmt", default)]
    pub total: f64,
    #[serde(rename = "EmailStatus", default)]
    pub email_status: String,
    #[serde(rename = "Balance", default)]
    pub balance: f64,
}

impl InvoiceWire {
    pub(crate) fn into_invoice(self, web_url: String) -> QbInvoice {
        let status = if self.balance == 0.0 && self.total > 0.0 {
            QbDocStatus::Paid
        } else {
            QbDocStatus::from_qbo(&self.email_status)
        };
        QbInvoice {
            id: self.id,
            doc_number: self.doc_number,
            total_cents: dollars_to_cents(self.total),
            status,
            web_url,
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct PurchaseWire {
    #[serde(rename = "Id")]
    pub id: String,
    #[serde(rename = "TotalAmt", default)]
    pub total: f64,
    #[serde(rename = "AccountRef")]
    pub account_ref: Option<RefValue>,
    #[serde(rename = "TxnDate")]
    pub txn_date: NaiveDate,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RefValue {
    pub value: String,
}

impl From<PurchaseWire> for QbExpense {
    fn from(wire: PurchaseWire) -> Self {
        Self {
            id: wire.id,
            amount_cents: dollars_to_cents(wire.total),
            account_id: wire.account_ref.map(|r| r.value).unwrap_or_default(),
            txn_date: wire.txn_date,
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct AccountWire {
    #[serde(rename = "Id")]
    pub id: String,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "AccountType")]
    pub account_type: String,
    #[serde(rename = "CurrentBalance", default)]
    pub balance: f64,
}

impl From<AccountWire> for QbAccount {
    fn from(wire: AccountWire) -> Self {
        Self {
            id: wire.id,
            name: wire.name,
            account_type: wire.account_type,
            balance_cents: dollars_to_cents(wire.balance),
        }
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct InvoiceCreate<'a> {
    #[serde(rename = "CustomerRef")]
    pub customer_ref: RefOut<'a>,
    #[serde(rename = "Line")]
    pub line: Vec<InvoiceLineOut>,
    #[serde(rename = "DueDate")]
    pub due_date: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct RefOut<'a> {
    pub value: &'a str,
}

#[derive(Debug, Serialize)]
pub(crate) struct InvoiceLineOut {
    #[serde(rename = "DetailType")]
    pub detail_type: &'static str,
    #[serde(rename = "Amount")]
    pub amount: f64,
    #[serde(rename = "Description")]
    pub description: String,
    #[serde(rename = "SalesItemLineDetail")]
    pub sales_item_line_detail: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub(crate) struct PurchaseCreate<'a> {
    #[serde(rename = "AccountRef")]
    pub account_ref: RefOut<'a>,
    #[serde(rename = "EntityRef", skip_serializing_if = "Option::is_none")]
    pub entity_ref: Option<RefOut<'a>>,
    #[serde(rename = "TotalAmt")]
    pub total_amt: f64,
    #[serde(rename = "TxnDate")]
    pub txn_date: String,
    #[serde(rename = "PaymentType")]
    pub payment_type: &'static str,
    #[serde(rename = "Line")]
    pub line: Vec<PurchaseLineOut>,
    #[serde(rename = "PrivateNote", skip_serializing_if = "Option::is_none")]
    pub private_note: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct PurchaseLineOut {
    #[serde(rename = "DetailType")]
    pub detail_type: &'static str,
    #[serde(rename = "Amount")]
    pub amount: f64,
    #[serde(rename = "AccountBasedExpenseLineDetail")]
    pub account_based_expense_line_detail: serde_json::Value,
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
    fn doc_status_classifies_known_values() {
        assert_eq!(QbDocStatus::from_qbo("Paid"), QbDocStatus::Paid);
        assert_eq!(QbDocStatus::from_qbo("Sent"), QbDocStatus::Sent);
        match QbDocStatus::from_qbo("Whatever") {
            QbDocStatus::Other(s) => assert_eq!(s, "Whatever"),
            other => panic!("expected Other, got {other:?}"),
        }
    }
}
