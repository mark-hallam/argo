//! PII detection on write paths.
//!
//! `scan` returns a [`PiiFindings`] report listing every recognized
//! span (SSN, Luhn-valid credit card, email, US phone, or IBAN) in
//! the input. Callers either soft-flag findings (log + persist
//! original) or call [`PiiFindings::redact`] to replace each finding
//! with `<redacted:<kind>>` before persisting.
//!
//! Three env knobs control behavior at the call site:
//!
//! - `ARGO_PII_DETECT=0` short-circuits [`scan`] to an empty report.
//! - `ARGO_PII_AUTOREDACT=1` makes [`scan_and_maybe_redact`] return
//!   the redacted form instead of the original.
//! - `ARGO_PII_LOG_FINDINGS=0` silences the per-finding `tracing::warn!`
//!   emitted by [`scan_and_maybe_redact`] (default on).
//!
//! ```
//! use argo_security::pii_detect::{scan, PiiKind};
//! let findings = scan("contact me at alice@example.com");
//! assert_eq!(findings.0.len(), 1);
//! assert_eq!(findings.0[0].kind, PiiKind::Email);
//! ```

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::ops::Range;
use std::sync::OnceLock;

/// Categories of PII the scanner recognizes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PiiKind {
    /// US Social Security Number (`123-45-6789`, shape-filtered).
    Ssn,
    /// Credit card number, validated against the Luhn checksum.
    CreditCard,
    /// RFC-5321-ish email address.
    Email,
    /// US phone number (10 digits, common separators).
    UsPhone,
    /// International Bank Account Number, mod-97 validated.
    Iban,
}

impl PiiKind {
    /// Stable wire label used in logs and `<redacted:label>` markers.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            PiiKind::Ssn => "ssn",
            PiiKind::CreditCard => "credit_card",
            PiiKind::Email => "email",
            PiiKind::UsPhone => "us_phone",
            PiiKind::Iban => "iban",
        }
    }
}

/// One match: kind, byte span in the original input, and a masked
/// rendering safe to surface in logs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PiiFinding {
    /// Which kind of PII matched.
    pub kind: PiiKind,
    /// Byte range in the input string (suitable for slicing).
    pub span: Range<usize>,
    /// Masked rendering (`ab****yz`) for safe logging.
    pub masked_value: String,
}

/// Set of all findings from a single [`scan`] call. Spans are sorted
/// by `start` and de-overlapped (a later regex match that overlaps an
/// earlier one is dropped).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PiiFindings(
    /// Underlying findings, sorted by `span.start`, non-overlapping.
    pub Vec<PiiFinding>,
);

impl PiiFindings {
    /// True when no findings were recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Replace every finding's span with `<redacted:<kind>>` and
    /// return the rewritten string. Findings are processed in
    /// span-start order so the byte offsets stay coherent.
    #[must_use]
    pub fn redact(&self, input: &str) -> String {
        if self.0.is_empty() {
            return input.to_string();
        }
        let mut out = String::with_capacity(input.len());
        let mut cursor = 0usize;
        for f in &self.0 {
            if f.span.start < cursor || f.span.end > input.len() {
                continue;
            }
            out.push_str(&input[cursor..f.span.start]);
            out.push_str("<redacted:");
            out.push_str(f.kind.label());
            out.push('>');
            cursor = f.span.end;
        }
        out.push_str(&input[cursor..]);
        out
    }
}

struct Patterns {
    ssn: Option<Regex>,
    cc: Option<Regex>,
    email: Option<Regex>,
    phone: Option<Regex>,
    iban: Option<Regex>,
}

fn patterns() -> &'static Patterns {
    static P: OnceLock<Patterns> = OnceLock::new();
    P.get_or_init(|| Patterns {
        ssn: Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").ok(),
        cc: Regex::new(
            r"\b\d{4}[ -]\d{4}[ -]\d{4}[ -]\d{1,4}\b|\b\d{4}[ -]\d{6}[ -]\d{4,5}\b|\b\d{13,19}\b",
        )
        .ok(),
        email: Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,24}\b").ok(),
        phone: Regex::new(r"\b\(?\d{3}\)?[-.\s]?\d{3}[-.\s]?\d{4}\b").ok(),
        iban: Regex::new(r"\b[A-Z]{2}\d{2}[A-Z0-9]{11,30}\b").ok(),
    })
}

fn ssn_shape_valid(s: &str) -> bool {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 {
        return false;
    }
    let area = parts[0];
    let group = parts[1];
    let serial = parts[2];
    if area == "000" || area == "666" || area.starts_with('9') {
        return false;
    }
    if group == "00" || serial == "0000" {
        return false;
    }
    true
}

fn luhn_valid(s: &str) -> bool {
    let digits: Vec<u32> = s.chars().filter_map(|c| c.to_digit(10)).collect();
    if digits.len() < 13 || digits.len() > 19 {
        return false;
    }
    let sum: u32 = digits
        .iter()
        .rev()
        .enumerate()
        .map(|(i, &d)| {
            if i % 2 == 1 {
                let dd = d * 2;
                if dd > 9 {
                    dd - 9
                } else {
                    dd
                }
            } else {
                d
            }
        })
        .sum();
    sum.is_multiple_of(10)
}

fn iban_valid(s: &str) -> bool {
    if s.len() < 15 || s.len() > 34 {
        return false;
    }
    let bytes = s.as_bytes();
    let rotated: String = bytes[4..]
        .iter()
        .chain(bytes[..4].iter())
        .map(|&b| b as char)
        .collect();
    let mut rem: u64 = 0;
    for c in rotated.chars() {
        let v: u64 = if c.is_ascii_digit() {
            u64::from(c as u8 - b'0')
        } else if c.is_ascii_uppercase() {
            u64::from(c as u8 - b'A' + 10)
        } else {
            return false;
        };
        if v < 10 {
            rem = (rem * 10 + v) % 97;
        } else {
            rem = (rem * 100 + v) % 97;
        }
    }
    rem == 1
}

fn mask(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let n = chars.len();
    if n <= 4 {
        return "*".repeat(n);
    }
    let prefix: String = chars[..2].iter().collect();
    let suffix: String = chars[n - 2..].iter().collect();
    format!("{prefix}{}{suffix}", "*".repeat(n - 4))
}

/// Env-driven write-path policy. Constructed from the three
/// `ARGO_PII_*` variables but exposed as a plain struct so unit
/// tests can exercise the policy without mutating process env.
#[derive(Debug, Clone, Copy)]
pub struct PolicyConfig {
    /// Run the scanner at all (default true; `ARGO_PII_DETECT=0` flips).
    pub detect: bool,
    /// Replace findings inline before persisting (`ARGO_PII_AUTOREDACT=1`).
    pub autoredact: bool,
    /// Emit a `tracing::warn!` per finding (default true;
    /// `ARGO_PII_LOG_FINDINGS=0` silences).
    pub log_findings: bool,
}

impl PolicyConfig {
    /// Read the three env knobs and build a config.
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            detect: !matches!(std::env::var("ARGO_PII_DETECT").as_deref(), Ok("0")),
            autoredact: matches!(std::env::var("ARGO_PII_AUTOREDACT").as_deref(), Ok("1")),
            log_findings: !matches!(std::env::var("ARGO_PII_LOG_FINDINGS").as_deref(), Ok("0")),
        }
    }
}

/// Scan `input` for PII. Returns an empty report when
/// `ARGO_PII_DETECT=0`. Spans are sorted by start and de-overlapped.
#[must_use]
pub fn scan(input: &str) -> PiiFindings {
    if !PolicyConfig::from_env().detect {
        return PiiFindings::default();
    }
    scan_inner(input)
}

fn scan_inner(input: &str) -> PiiFindings {
    let p = patterns();
    let mut raw: Vec<PiiFinding> = Vec::new();

    if let Some(re) = &p.ssn {
        for m in re.find_iter(input) {
            if !ssn_shape_valid(m.as_str()) {
                continue;
            }
            raw.push(PiiFinding {
                kind: PiiKind::Ssn,
                span: m.range(),
                masked_value: mask(m.as_str()),
            });
        }
    }
    if let Some(re) = &p.cc {
        for m in re.find_iter(input) {
            if !luhn_valid(m.as_str()) {
                continue;
            }
            raw.push(PiiFinding {
                kind: PiiKind::CreditCard,
                span: m.range(),
                masked_value: mask(m.as_str()),
            });
        }
    }
    if let Some(re) = &p.email {
        for m in re.find_iter(input) {
            raw.push(PiiFinding {
                kind: PiiKind::Email,
                span: m.range(),
                masked_value: mask(m.as_str()),
            });
        }
    }
    if let Some(re) = &p.phone {
        for m in re.find_iter(input) {
            raw.push(PiiFinding {
                kind: PiiKind::UsPhone,
                span: m.range(),
                masked_value: mask(m.as_str()),
            });
        }
    }
    if let Some(re) = &p.iban {
        for m in re.find_iter(input) {
            if !iban_valid(m.as_str()) {
                continue;
            }
            raw.push(PiiFinding {
                kind: PiiKind::Iban,
                span: m.range(),
                masked_value: mask(m.as_str()),
            });
        }
    }

    raw.sort_by_key(|f| f.span.start);
    let mut deduped: Vec<PiiFinding> = Vec::with_capacity(raw.len());
    for f in raw {
        if let Some(last) = deduped.last() {
            if f.span.start < last.span.end {
                continue;
            }
        }
        deduped.push(f);
    }
    PiiFindings(deduped)
}

/// Scan `input` and apply the env-driven policy used by message and
/// audit write paths. Logs each finding via `tracing::warn!` (unless
/// `ARGO_PII_LOG_FINDINGS=0`) and returns the redacted form when
/// `ARGO_PII_AUTOREDACT=1`. Otherwise returns the borrowed original.
#[must_use]
pub fn scan_and_maybe_redact<'a>(input: &'a str, site: &'static str) -> Cow<'a, str> {
    apply_policy(input, PolicyConfig::from_env(), site)
}

/// Same as [`scan_and_maybe_redact`] but with an explicit policy.
/// Pure (no env reads) so unit tests can validate every branch
/// without mutating process-global state.
#[must_use]
pub fn apply_policy<'a>(
    input: &'a str,
    cfg: PolicyConfig,
    site: &'static str,
) -> Cow<'a, str> {
    if !cfg.detect {
        return Cow::Borrowed(input);
    }
    let findings = scan_inner(input);
    if findings.is_empty() {
        return Cow::Borrowed(input);
    }
    if cfg.log_findings {
        for f in &findings.0 {
            tracing::warn!(
                site = site,
                kind = f.kind.label(),
                masked = %f.masked_value,
                "pii finding on write path"
            );
        }
    }
    if cfg.autoredact {
        Cow::Owned(findings.redact(input))
    } else {
        Cow::Borrowed(input)
    }
}
