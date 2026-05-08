//! `argo-security` — security primitives shared across the workspace.
//!
//! Today the only module is [`pii_detect`], a write-path scanner that
//! recognizes SSN / credit card (Luhn-validated) / email / US phone /
//! IBAN. Findings are returned as byte spans so callers can either
//! soft-flag (log) or auto-redact in place.
//!
//! Future briefs may add: secret-scanning, capability-token verifiers,
//! rate-limit primitives. Keep the crate dep set narrow.
#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

pub mod pii_detect;

#[cfg(test)]
mod pii_detect_tests;
