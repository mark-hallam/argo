//! Compiled regex patterns + Luhn validator for the 5 PII categories.
//!
//! Patterns mirror E1-e's TypeScript layer (which is still authoring
//! at brief BLOCKED time of writing). When E1-e lands, both layers
//! must accept identical strings as PII so the masks line up; the
//! `lib_tests.rs` fixtures double as the cross-layer test corpus.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::ops::Range;
use std::sync::OnceLock;

/// One of the five categories the v1 fence recognises.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    /// RFC-5321-ish email address.
    Email,
    /// Phone number (NZ + common international shapes).
    Phone,
    /// Credit card number, Luhn-validated.
    Card,
    /// International Bank Account Number (shape-only; no mod-97 in v1
    /// to align with the client layer's regex-only check).
    Iban,
    /// New Zealand IRD number (8 or 9 digits, dashed or run-together).
    Ird,
}

/// Every category, in stable definition order. Useful for tests and
/// for callers that need to iterate (e.g. building a per-category
/// `RedactionPolicy`).
pub const ALL_CATEGORIES: [Category; 5] = [
    Category::Email,
    Category::Phone,
    Category::Card,
    Category::Iban,
    Category::Ird,
];

impl Category {
    /// Stable wire label used in audit payloads and grant rows.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Email => "email",
            Self::Phone => "phone",
            Self::Card => "card",
            Self::Iban => "iban",
            Self::Ird => "ird",
        }
    }

    /// Sentinel inserted at every match span when the category is
    /// being masked. Byte-identical to the E1-e client layer.
    #[must_use]
    pub fn sentinel(self) -> &'static str {
        match self {
            Self::Email => crate::SENTINEL_EMAIL,
            Self::Phone => crate::SENTINEL_PHONE,
            Self::Card => crate::SENTINEL_CARD,
            Self::Iban => crate::SENTINEL_IBAN,
            Self::Ird => crate::SENTINEL_IRD,
        }
    }

    /// Parse from a wire label. Returns `None` for unknown labels.
    #[must_use]
    pub fn from_label(s: &str) -> Option<Self> {
        Some(match s {
            "email" => Self::Email,
            "phone" => Self::Phone,
            "card" => Self::Card,
            "iban" => Self::Iban,
            "ird" => Self::Ird,
            _ => return None,
        })
    }
}

struct Patterns {
    email: Option<Regex>,
    phone: Option<Regex>,
    card: Option<Regex>,
    iban: Option<Regex>,
    ird: Option<Regex>,
}

fn patterns() -> &'static Patterns {
    static P: OnceLock<Patterns> = OnceLock::new();
    P.get_or_init(|| Patterns {
        email: Regex::new(r"\b[A-Za-z0-9._%+\-]+@[A-Za-z0-9.\-]+\.[A-Za-z]{2,24}\b").ok(),
        // Phone shapes: international prefixed (+CC ...), or NZ-style
        // leading-0 trunk (02x mobile, 0[3-9]x landline). Whitespace
        // ignored via verbose mode for readability.
        phone: Regex::new(
            r"(?x)
            \+\d{1,3}[\s\-]?\(?\d{1,4}\)?[\s\-]?\d{2,4}[\s\-]?\d{3,4}
            |
            \b0\d{1,2}[\s\-]?\d{3}[\s\-]?\d{4}\b
            ",
        )
        .ok(),
        // Card: 13-19 digits with optional spaces / dashes between
        // each pair. Luhn validation happens downstream in
        // `find_matches`.
        card: Regex::new(r"\b(?:\d[ \-]?){12,18}\d\b").ok(),
        // IBAN: country prefix + check digits + 11-30 alphanumerics.
        iban: Regex::new(r"\b[A-Z]{2}\d{2}[A-Z0-9]{11,30}\b").ok(),
        // NZ IRD: 8 or 9 digits, optionally dashed at positions 2-3
        // and 5-6 (`12-345-678` or `123-456-789`).
        ird: Regex::new(r"\b\d{2,3}[\s\-]?\d{3}[\s\-]?\d{3}\b").ok(),
    })
}

/// Internal match record: which category fired and where in the input.
#[derive(Debug, Clone)]
pub(crate) struct PatternMatch {
    pub category: Category,
    pub span: Range<usize>,
}

/// Run every category's regex over `input`, dropping non-Luhn-valid
/// card hits. Returns matches in regex-evaluation order; callers
/// de-overlap by span-start before applying masks.
pub(crate) fn find_matches(input: &str) -> Vec<PatternMatch> {
    let p = patterns();
    let mut out = Vec::new();
    let pairs: [(Category, Option<&Regex>); 5] = [
        (Category::Email, p.email.as_ref()),
        (Category::Iban, p.iban.as_ref()),
        (Category::Card, p.card.as_ref()),
        (Category::Phone, p.phone.as_ref()),
        (Category::Ird, p.ird.as_ref()),
    ];
    for (cat, maybe_re) in pairs {
        let Some(re) = maybe_re else { continue };
        for m in re.find_iter(input) {
            if cat == Category::Card && !luhn_valid(m.as_str()) {
                continue;
            }
            out.push(PatternMatch {
                category: cat,
                span: m.range(),
            });
        }
    }
    out
}

/// Luhn checksum validator. Strips non-digits before computing the
/// checksum, so spaces and dashes inside `s` are fine.
///
/// ```
/// use argo_redact::luhn_valid;
/// assert!(luhn_valid("4111 1111 1111 1111"));
/// assert!(!luhn_valid("1234 5678 9012 3456"));
/// ```
#[must_use]
pub fn luhn_valid(s: &str) -> bool {
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
                let doubled = d * 2;
                if doubled > 9 {
                    doubled - 9
                } else {
                    doubled
                }
            } else {
                d
            }
        })
        .sum();
    sum.is_multiple_of(10)
}
