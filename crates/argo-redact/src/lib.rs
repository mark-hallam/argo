//! `argo-redact` — server-side PII fence for gateway response paths.
//!
//! Track J brief 2 / E1-e companion: the gateway calls into this crate
//! before serializing an approval row so PII strings (email, phone,
//! Luhn-valid card, IBAN, NZ IRD) are replaced with sentinels unless
//! the viewer's `Principal` carries an `unredacted_<category>`
//! permission or an active elevated-view grant.
//!
//! ```
//! use argo_redact::{redact_text, RedactionPolicy};
//! let (masked, hits) =
//!     redact_text("Contact me at sam@acmeco.com", &RedactionPolicy::default());
//! assert_eq!(masked, "Contact me at [EMAIL]");
//! assert_eq!(hits.len(), 1);
//! ```
//!
//! Sentinels are byte-identical to the E1-e client-layer
//! (`[EMAIL]`, `[PHONE]`, `[CARD]`, `[IBAN]`, `[IRD]`) so the two
//! defenses mask the same way and grep / cross-layer tests align.
//!
//! v1 scope:
//! - 5 regex-driven categories matching the client layer.
//! - `ViewerPermissions` carries per-category overrides; no LLM NER.
//! - `GrantSet::empty()` for v1 mocking; J16 wires DB lookup.
//! - Audit row format exposed via [`audit_payload`]; the gateway
//!   emits it on a structured tracing target until the audit-kind
//!   variant lands (see PR notes — argo-audit is out of may-touch).
#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

mod grants;
mod patterns;
mod policy;
mod redact;

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;

pub use grants::{active_grants_for_principal, Grant, GrantSet};
pub use patterns::{luhn_valid, Category, ALL_CATEGORIES};
pub use policy::{resolve_policy, PolicyMode, RedactionPolicy, ViewerPermissions};
pub use redact::{
    audit_payload, redact_approval_row, redact_text, ApprovalRedaction, AUDIT_EVENT_TYPE,
};

/// Sentinel inserted in place of email matches when policy masks email.
pub const SENTINEL_EMAIL: &str = "[EMAIL]";
/// Sentinel inserted in place of phone matches when policy masks phone.
pub const SENTINEL_PHONE: &str = "[PHONE]";
/// Sentinel inserted in place of Luhn-valid card matches when policy
/// masks card.
pub const SENTINEL_CARD: &str = "[CARD]";
/// Sentinel inserted in place of IBAN matches when policy masks IBAN.
pub const SENTINEL_IBAN: &str = "[IBAN]";
/// Sentinel inserted in place of NZ IRD matches when policy masks IRD.
pub const SENTINEL_IRD: &str = "[IRD]";
