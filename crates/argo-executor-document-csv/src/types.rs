//! Typed schema for CSV columns + rejection records surfaced in the
//! write report.

use serde::{Deserialize, Serialize};

/// One column of the CSV schema. Drives both the header row and the
/// per-row coercion / validation pass.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CsvColumnSchema {
    /// Header text — emitted in the header row and used as the key
    /// when looking up each row's value in the
    /// `HashMap<String, serde_json::Value>` row body.
    pub header: String,
    /// Expected value type. Drives the coercion table in
    /// [`crate::document_csv`].
    pub kind: CsvColumnKind,
    /// `true` ⇒ a missing or null value in this column rejects the
    /// whole row; `false` ⇒ missing renders as the empty string.
    pub required: bool,
    /// Optional length cap on the rendered string form. Rendered
    /// length is byte length of the UTF-8 string (not codepoints) so
    /// the cap matches downstream byte-size validators.
    pub max_len: Option<u32>,
}

/// The six column kinds the executor knows how to coerce.
///
/// `EmailAddress` validation is intentionally shape-only
/// (`^[^@]+@[^@]+\.[^@]+$`); a stricter validator would be a
/// follow-up brief.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CsvColumnKind {
    /// Any serde-JSON value rendered with `.to_string()`. Null
    /// becomes the empty string.
    Text,
    /// Numeric integer. `Number(i)` accepted as-is;
    /// `String(s)` parsed as `i64` or row is rejected.
    Integer,
    /// Numeric decimal. `Number(f)` formatted with `{}` (trailing
    /// zeros stripped); `String(s)` passed through verbatim so the
    /// caller controls trailing-zero presentation.
    Decimal,
    /// Calendar date. `String(s)` is parsed as RFC 3339 and rendered
    /// as `YYYY-MM-DD`; anything else rejects the row.
    Date,
    /// True/false boolean. `Bool(b)` ⇒ "true"/"false";
    /// `String("true"|"false"|"1"|"0")` is coerced.
    Boolean,
    /// Email address. Shape-only check (`one@two.three`); anything
    /// else rejects the row.
    EmailAddress,
}

/// Line terminator written between records. RFC 4180 default is
/// [`LineTerminator::Crlf`]; Excel's legacy importers reject bare
/// `\n` on some locales, so don't switch to LF unless you have a
/// specific downstream tool that demands it.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum LineTerminator {
    /// `\r\n` — RFC 4180 default.
    #[default]
    Crlf,
    /// `\n` — Unix-style; some legacy tools reject this.
    Lf,
}

impl LineTerminator {
    /// Byte sequence emitted between records.
    #[must_use]
    pub fn bytes(self) -> &'static [u8] {
        match self {
            LineTerminator::Crlf => b"\r\n",
            LineTerminator::Lf => b"\n",
        }
    }
}

/// A row that was rejected during the write pass. The report carries
/// every rejection so the operator can fix the inputs and retry;
/// rejection is record-level — one bad row never aborts the file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RowRejection {
    /// Zero-based index of the row in the request's `rows` vector.
    pub row_index: u32,
    /// Header of the offending column (if the rejection is
    /// column-scoped).
    pub column: Option<String>,
    /// Machine-readable rejection reason.
    pub reason: RejectionReason,
    /// One-line human description for the audit ledger.
    pub message: String,
}

/// Discriminator for why a row was rejected. Distinct variants let
/// the bundle surface specific counts (e.g. "12 missing required, 3
/// max-len exceeded") without string-sniffing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RejectionReason {
    /// A column with `required = true` was absent or null.
    MissingRequired,
    /// The value type couldn't be coerced into the column kind.
    TypeMismatch,
    /// The rendered byte length exceeded the column's `max_len`.
    MaxLenExceeded,
    /// The column is not in the schema.
    UnknownColumn,
}
