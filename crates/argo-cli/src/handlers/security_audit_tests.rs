//! Unit tests for the `argo security audit` JSON parsers
//! (Mission `mission-vuln-management-workflow`). Five focused
//! tests covering the cargo-audit + cargo-deny parsers, severity
//! counting, and deduplication. Fixtures are intentionally small;
//! we test the parse contract, not the upstream tools.

use super::security_audit::{
    count_by_severity, dedup_by_advisory, parse_cargo_audit, parse_cargo_deny,
    parse_pnpm_audit, Vuln,
};

/// Embedded fixture mirroring the shape of `cargo audit --json`
/// output. Real fixtures have many more fields; the parser only
/// looks at `vulnerabilities.list[].advisory.{id,severity}`,
/// `.package.name`, and `.versions.patched[]`, so the trimmed
/// fixture is enough to exercise the contract.
const CARGO_AUDIT_FIXTURE: &str = r#"{
  "database": { "advisory_count": 700 },
  "lockfile": { "dependency_count": 412 },
  "vulnerabilities": {
    "found": true,
    "count": 2,
    "list": [
      {
        "advisory": {
          "id": "RUSTSEC-2024-0003",
          "package": "hyper",
          "title": "hyper 0.14 panic in HTTP/2 server",
          "severity": "high"
        },
        "package": { "name": "hyper", "version": "0.14.27" },
        "versions": { "patched": [">=0.14.30"], "unaffected": [] }
      },
      {
        "advisory": {
          "id": "RUSTSEC-2024-0099",
          "package": "tinyvec",
          "title": "uninitialized read",
          "severity": "medium"
        },
        "package": { "name": "tinyvec", "version": "1.6.0" },
        "versions": { "patched": [">=1.7.0"], "unaffected": [] }
      }
    ]
  }
}"#;

#[test]
fn parse_cargo_audit_empty_returns_no_rows() {
    assert!(parse_cargo_audit("").expect("empty").is_empty());
    assert!(parse_cargo_audit("   \n  ").expect("whitespace").is_empty());
    let empty_report = r#"{ "vulnerabilities": { "list": [] } }"#;
    assert!(parse_cargo_audit(empty_report).expect("empty list").is_empty());
}

#[test]
fn parse_cargo_audit_extracts_id_severity_crate_fixedin() {
    let parsed = parse_cargo_audit(CARGO_AUDIT_FIXTURE).expect("parse ok");
    assert_eq!(parsed.len(), 2, "fixture has two rows");

    let first = &parsed[0];
    assert_eq!(first.advisory_id, "RUSTSEC-2024-0003");
    assert_eq!(first.severity, "high");
    assert_eq!(first.affected_crate, "hyper");
    assert_eq!(first.fixed_in.as_deref(), Some(">=0.14.30"));
    assert_eq!(first.status, "unmitigated");
    assert!(first.mitigation_note.is_none());

    let second = &parsed[1];
    assert_eq!(second.advisory_id, "RUSTSEC-2024-0099");
    assert_eq!(second.severity, "medium");
    assert_eq!(second.affected_crate, "tinyvec");
}

#[test]
fn parse_cargo_deny_supports_array_form() {
    let fixture = r#"[
        {"advisory":{"id":"RUSTSEC-2023-0050","severity":"critical","package":{"name":"openssl-sys"},"patched":">=0.9.93"}},
        {"advisory":{"id":"RUSTSEC-2023-0051","severity":"low","package":{"name":"chrono"}}},
        {"diagnostic":{"summary":"not an advisory event"}}
    ]"#;
    let parsed = parse_cargo_deny(fixture).expect("parse ok");
    assert_eq!(parsed.len(), 2, "non-advisory events are skipped");
    assert_eq!(parsed[0].advisory_id, "RUSTSEC-2023-0050");
    assert_eq!(parsed[0].severity, "critical");
    assert_eq!(parsed[0].affected_crate, "openssl-sys");
    assert_eq!(parsed[0].fixed_in.as_deref(), Some(">=0.9.93"));
    assert_eq!(parsed[1].advisory_id, "RUSTSEC-2023-0051");
    assert!(parsed[1].fixed_in.is_none(), "missing patched -> None");
}

#[test]
fn parse_cargo_deny_supports_ndjson_form() {
    let fixture = r#"{"advisory":{"id":"RUSTSEC-2024-0001","severity":"HIGH","package":{"name":"ring"}}}
{"advisory":{"id":"RUSTSEC-2024-0002","severity":"low","package":{"name":"time"}}}
"#;
    let parsed = parse_cargo_deny(fixture).expect("parse ok");
    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed[0].advisory_id, "RUSTSEC-2024-0001");
    assert_eq!(parsed[0].severity, "high", "severity is lowercased");
    assert_eq!(parsed[1].advisory_id, "RUSTSEC-2024-0002");
}

#[test]
fn count_and_dedup_match_expected_envelope() {
    // Simulate the merged-list state: cargo audit + cargo deny
    // both produce a "high" row for RUSTSEC-2024-0003, plus a
    // "critical" row only deny saw, plus a "medium" row only audit
    // saw. Dedup should fold the duplicate, count should bucket by
    // lowercased severity, and every defined bucket should appear
    // even if zero.
    let merged = vec![
        Vuln {
            advisory_id: "RUSTSEC-2024-0003".into(),
            severity: "high".into(),
            affected_crate: "hyper".into(),
            fixed_in: Some(">=0.14.30".into()),
            status: "unmitigated".into(),
            mitigation_note: None,
        },
        Vuln {
            advisory_id: "RUSTSEC-2024-0003".into(),
            severity: "high".into(),
            affected_crate: "hyper".into(),
            fixed_in: Some(">=0.14.30".into()),
            status: "unmitigated".into(),
            mitigation_note: None,
        },
        Vuln {
            advisory_id: "RUSTSEC-2024-0050".into(),
            severity: "critical".into(),
            affected_crate: "openssl-sys".into(),
            fixed_in: None,
            status: "unmitigated".into(),
            mitigation_note: None,
        },
        Vuln {
            advisory_id: "RUSTSEC-2024-0099".into(),
            severity: "medium".into(),
            affected_crate: "tinyvec".into(),
            fixed_in: Some(">=1.7.0".into()),
            status: "unmitigated".into(),
            mitigation_note: None,
        },
    ];

    let deduped = dedup_by_advisory(merged);
    assert_eq!(deduped.len(), 3, "duplicate advisory_id folded");

    let counts = count_by_severity(&deduped);
    assert_eq!(counts.get("critical"), Some(&1));
    assert_eq!(counts.get("high"), Some(&1));
    assert_eq!(counts.get("medium"), Some(&1));
    assert_eq!(counts.get("low"), Some(&0), "low bucket present even when empty");
    assert_eq!(counts.get("unknown"), Some(&0));
}

#[test]
fn parse_pnpm_audit_empty_returns_no_rows() {
    assert!(parse_pnpm_audit("").expect("empty").is_empty());
    assert!(parse_pnpm_audit("   \n  ").expect("whitespace").is_empty());
    let empty_report = r#"{ "advisories": {} }"#;
    assert!(parse_pnpm_audit(empty_report).expect("empty advisories").is_empty());
}

#[test]
fn parse_pnpm_merge_dedups_with_cargo_and_buckets_severities() {
    // pnpm advisory id is the JSON object key under `.advisories`.
    // Severity is uppercased on the wire; the parser must lowercase
    // it. `module_name` maps to `affected_crate`, `patched_versions`
    // to `fixed_in`.
    let pnpm_fixture = r#"{
      "advisories": {
        "GHSA-9999-0000-aaaa": {
          "severity": "HIGH",
          "module_name": "axios",
          "vulnerable_versions": "<1.7.4",
          "patched_versions": ">=1.7.4",
          "recommendation": "upgrade to 1.7.4"
        },
        "RUSTSEC-2024-0003": {
          "severity": "high",
          "module_name": "hyper-js-shim",
          "patched_versions": ">=2.0.0"
        }
      }
    }"#;
    let pnpm_rows = parse_pnpm_audit(pnpm_fixture).expect("parse ok");
    assert_eq!(pnpm_rows.len(), 2);

    let axios = pnpm_rows.iter().find(|v| v.advisory_id == "GHSA-9999-0000-aaaa").expect("axios row");
    assert_eq!(axios.severity, "high", "severity is lowercased");
    assert_eq!(axios.affected_crate, "axios", "module_name maps to affected_crate");
    assert_eq!(axios.fixed_in.as_deref(), Some(">=1.7.4"));
    assert_eq!(axios.status, "unmitigated");
    assert!(axios.mitigation_note.is_none());

    // Merge with a cargo-side row that shares `RUSTSEC-2024-0003`.
    // Dedup keeps the first occurrence (cargo here), so the pnpm
    // shim row drops out; bucket counts only see the survivors.
    let cargo_rows = parse_cargo_audit(CARGO_AUDIT_FIXTURE).expect("parse cargo");
    let mut all = cargo_rows;
    all.extend(pnpm_rows);
    let merged = dedup_by_advisory(all);
    assert_eq!(merged.len(), 3, "GHSA-axios + RUSTSEC-2024-0003 + RUSTSEC-2024-0099");
    assert!(merged.iter().any(|v| v.advisory_id == "GHSA-9999-0000-aaaa"));
    let dedup_winner = merged.iter().find(|v| v.advisory_id == "RUSTSEC-2024-0003").expect("dedup winner");
    assert_eq!(dedup_winner.affected_crate, "hyper", "cargo-side row wins dedup");

    let counts = count_by_severity(&merged);
    assert_eq!(counts.get("high"), Some(&2), "axios (pnpm) + hyper (cargo)");
    assert_eq!(counts.get("medium"), Some(&1), "tinyvec (cargo)");
    assert_eq!(counts.get("critical"), Some(&0));
}
