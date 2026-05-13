//! Compliance workflows extracted from the CLI handlers.
//!
//! The CLI remains responsible for argument parsing and locating the
//! workspace database; this crate owns the reusable GDPR/SOC2
//! business logic.

pub mod delete;
pub mod export;
pub mod soc2;
pub mod tenant_tables;
