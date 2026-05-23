//! Structural pin for the MCP connector real-vs-stub inventory.
//!
//! Companion to `docs/skycot/evals/scheduled/2026-05-23/
//! eval-mcp-connector-real-vs-stub-inventory.md`. The inventory is
//! manually authored. If the set of `crates/argo-mcp-connector-*`
//! directories changes (a connector is added, deleted, or renamed),
//! this test fails loudly so the matrix gets re-run before the diff
//! lands. The pin uses an allowlist rather than a count so the
//! failure message names *which* connector is missing or unexpected.
//!
//! When intentionally adding/deleting a connector:
//!   1. Edit `EXPECTED_CONNECTORS` below.
//!   2. Refresh the matrix at the doc path above (the 6 cells: real
//!      connect, tenant-keyed creds, smoke test, op-configurable,
//!      tier, verdict).
//!   3. Re-run `cargo test -p argo-mcp connector_inventory_pin`.

use std::collections::BTreeSet;
use std::path::PathBuf;

/// Connectors and the shared helper as of `origin/main` `7a00c577d`
/// (2026-05-23). Refresh in lockstep with the inventory report.
const EXPECTED_CONNECTORS: &[&str] = &[
    "argo-mcp-connector-airtable",
    "argo-mcp-connector-asana",
    "argo-mcp-connector-calendly",
    "argo-mcp-connector-clickup",
    "argo-mcp-connector-discord",
    "argo-mcp-connector-figma",
    "argo-mcp-connector-fireflies",
    "argo-mcp-connector-gcal",
    "argo-mcp-connector-github-issues",
    "argo-mcp-connector-gmail",
    "argo-mcp-connector-google-meet",
    "argo-mcp-connector-hubspot",
    "argo-mcp-connector-imessage",
    "argo-mcp-connector-jira",
    "argo-mcp-connector-linear",
    "argo-mcp-connector-mailchimp",
    "argo-mcp-connector-monday",
    "argo-mcp-connector-notion",
    "argo-mcp-connector-onedrive",
    "argo-mcp-connector-pagerduty",
    "argo-mcp-connector-postmark",
    "argo-mcp-connector-salesforce",
    "argo-mcp-connector-sendgrid",
    "argo-mcp-connector-shared",
    "argo-mcp-connector-slack",
    "argo-mcp-connector-stripe",
    "argo-mcp-connector-stripe-events",
    "argo-mcp-connector-trello",
    "argo-mcp-connector-typeform",
    "argo-mcp-connector-zendesk",
];

/// Helper crates that do NOT carry connector substrate (no upstream
/// API, no smoke-test against an HTTP fake). Subtracted from the
/// count to derive the operator-facing connector total.
const HELPER_CRATES: &[&str] = &["argo-mcp-connector-shared"];

fn crates_dir() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .map(PathBuf::from)
        .expect("argo-mcp manifest sits inside crates/")
}

fn observed_connector_dirs() -> BTreeSet<String> {
    let crates = crates_dir();
    let entries = std::fs::read_dir(&crates).unwrap_or_else(|e| {
        panic!(
            "failed to read crates dir {}: {e}",
            crates.display()
        )
    });
    entries
        .filter_map(Result::ok)
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|name| name.starts_with("argo-mcp-connector-"))
        .collect()
}

#[test]
fn connector_inventory_matches_pin() {
    let observed = observed_connector_dirs();
    let expected: BTreeSet<String> = EXPECTED_CONNECTORS.iter().map(|s| (*s).to_owned()).collect();

    let missing: Vec<&String> = expected.difference(&observed).collect();
    let unexpected: Vec<&String> = observed.difference(&expected).collect();

    if !missing.is_empty() || !unexpected.is_empty() {
        panic!(
            "Connector inventory drift detected. Refresh \
             docs/skycot/evals/scheduled/2026-05-23/\
             eval-mcp-connector-real-vs-stub-inventory.md AND \
             EXPECTED_CONNECTORS in this file.\n\
             Missing (in pin, not on disk): {missing:?}\n\
             Unexpected (on disk, not in pin): {unexpected:?}",
        );
    }
}

#[test]
fn connector_count_matches_report_headline() {
    let total = EXPECTED_CONNECTORS.len();
    let helpers = HELPER_CRATES.len();
    let connectors = total - helpers;

    assert_eq!(
        connectors, 29,
        "Report headline says 29 connectors + {helpers} helper. \
         Pin has {connectors} connectors + {helpers} helper. \
         Update the report headline (TL;DR + verdict counts) if \
         this changed intentionally."
    );
    assert_eq!(
        helpers, 1,
        "Report assumes exactly 1 helper crate (argo-mcp-connector-shared). \
         If a second helper landed, update HELPER_CRATES and the report \
         methodology section.",
    );
}
