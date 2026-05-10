//! Unit tests for `argo contract`. The store-touching paths are
//! exercised end-to-end in `tests/contract_apply_smoke.rs`; this
//! module locks the parser, semantic validator, and the round-trip
//! pair `build_kpis_blob` / `render_contract` so the brief's
//! "schema errors reported with line numbers" + acceptance round-trip
//! stay green without spinning up SQLite.

use crate::handlers::contract_schema::{
    build_kpis_blob, kr_entry_to_spec, parse_contract, render_contract, ApplyPlan, ContractFile,
    ContractMeta, DutyEntry, KeyResultEntry, KpiEntry, ObjectiveEntry, SuccessCriteria,
};
use argo_store::worker_okr::{
    Comparison, KeyResultRow, KeyResultSpec, ObjectiveRow, ObjectiveStatus,
};
use std::path::Path;

fn good_contract_toml() -> &'static str {
    r#"
[contract]
role = "ar-clerk"
charter = "Pursue outstanding receivables under approved policy."

[[duties]]
id = "send-reminder"
description = "Send dunning reminder at +30/+45/+60 days"

[[kpis]]
name = "dso_days"
description = "Days sales outstanding"
target = 35.0
comparison = "lte"

[[objectives]]
period = "2026Q2"
title = "Drive DSO under 30"

[[objectives.key_results]]
kpi = "dso_days"
target = 30.0
comparison = "lte"
weight = 1.0

[success_criteria]
min_sessions = 50
min_success_rate = 0.85
"#
}

fn good_contract_struct() -> ContractFile {
    ContractFile {
        contract: ContractMeta {
            role: "ar-clerk".into(),
            charter: "Pursue outstanding receivables under approved policy.".into(),
        },
        duties: vec![DutyEntry {
            id: "send-reminder".into(),
            description: "Send dunning reminder at +30/+45/+60 days".into(),
        }],
        kpis: vec![KpiEntry {
            name: "dso_days".into(),
            description: "Days sales outstanding".into(),
            target: 35.0,
            comparison: "lte".into(),
        }],
        objectives: vec![ObjectiveEntry {
            period: "2026Q2".into(),
            title: "Drive DSO under 30".into(),
            description: None,
            key_results: vec![KeyResultEntry {
                kpi: "dso_days".into(),
                target: 30.0,
                comparison: "lte".into(),
                weight: 1.0,
            }],
        }],
        success_criteria: Some(SuccessCriteria {
            min_sessions: Some(50),
            min_success_rate: Some(0.85),
        }),
    }
}

#[test]
fn parse_contract_accepts_full_well_formed_file() {
    let parsed = parse_contract(good_contract_toml(), Path::new("ar-clerk.contract.toml"))
        .expect("clean parse");
    assert_eq!(parsed.contract.role, "ar-clerk");
    assert_eq!(parsed.duties.len(), 1);
    assert_eq!(parsed.kpis.len(), 1);
    assert_eq!(parsed.objectives.len(), 1);
    assert_eq!(parsed.objectives[0].key_results.len(), 1);
    assert_eq!(
        parsed.success_criteria.as_ref().unwrap().min_sessions,
        Some(50),
    );
}

#[test]
fn parse_contract_rejects_unknown_comparison_with_path() {
    let body = r#"
[contract]
role = "ar-clerk"
charter = "x"

[[kpis]]
name = "dso_days"
description = "y"
target = 1.0
comparison = "wat"
"#;
    let err = parse_contract(body, Path::new("bad.toml")).expect_err("must reject");
    let msg = err.to_string();
    assert!(
        msg.contains("[[kpis]][0].comparison"),
        "expected indexed path in error, got: {msg}"
    );
}

#[test]
fn parse_contract_surfaces_line_number_for_malformed_toml() {
    // Two valid lines then a bare `=` token at line 4.
    let body = "\n\n[contract]\nrole = \n";
    let err = parse_contract(body, Path::new("bad.toml")).expect_err("must reject");
    let msg = err.to_string();
    assert!(
        msg.contains("bad.toml:4:") || msg.contains("bad.toml:5:"),
        "expected file:line:col prefix in error, got: {msg}"
    );
}

#[test]
fn parse_contract_rejects_zero_weight_key_result() {
    let body = r#"
[contract]
role = "ar-clerk"
charter = "x"

[[objectives]]
period = "2026Q2"
title = "t"

[[objectives.key_results]]
kpi = "k"
target = 1.0
comparison = "gte"
weight = 0.0
"#;
    let err = parse_contract(body, Path::new("zero.toml")).expect_err("must reject");
    assert!(err.to_string().contains("weight must be > 0"));
}

#[test]
fn parse_contract_rejects_objective_without_key_results() {
    let body = r#"
[contract]
role = "ar-clerk"
charter = "x"

[[objectives]]
period = "2026Q2"
title = "lonely"
"#;
    let err = parse_contract(body, Path::new("lonely.toml")).expect_err("must reject");
    assert!(err
        .to_string()
        .contains("must declare at least one key_result"));
}

#[test]
fn parse_contract_rejects_min_success_rate_out_of_range() {
    let body = r#"
[contract]
role = "ar-clerk"
charter = "x"

[success_criteria]
min_success_rate = 1.5
"#;
    let err = parse_contract(body, Path::new("bad-rate.toml")).expect_err("must reject");
    assert!(err.to_string().contains("min_success_rate"));
}

#[test]
fn apply_plan_counts_match_struct() {
    let c = good_contract_struct();
    let plan = ApplyPlan::from(&c);
    assert_eq!(plan.duty_count, 1);
    assert_eq!(plan.kpi_count, 1);
    assert_eq!(plan.objective_count, 1);
    assert_eq!(plan.kr_count, 1);
}

#[test]
fn kr_entry_to_spec_round_trips_through_worker_okr_shape() {
    let kr = KeyResultEntry {
        kpi: "dso_days".into(),
        target: 30.0,
        comparison: "lte".into(),
        weight: 1.0,
    };
    let spec: KeyResultSpec = kr_entry_to_spec(&kr).expect("convert");
    assert_eq!(spec.kpi_name, "dso_days");
    assert_eq!(spec.target_value, 30.0);
    assert!(matches!(spec.comparison, Comparison::Lte));
    assert!((spec.weight - 1.0).abs() < f64::EPSILON);
}

#[test]
fn build_kpis_blob_then_render_contract_round_trips_semantically() {
    // Save → reload through the persisted-blob pair, then prove the
    // reconstructed `ContractFile` matches the original module-by-module.
    let original = good_contract_struct();
    let blob = build_kpis_blob(&original);

    // Synthesise the objective + KR rows the store would have written.
    let objective_row = ObjectiveRow {
        id: 1,
        worker_id: "01900000-0000-7000-8000-000000000000".into(),
        tenant_id: "00000000-0000-0000-0000-000000000000".into(),
        period: original.objectives[0].period.clone(),
        title: original.objectives[0].title.clone(),
        description: original.objectives[0].description.clone(),
        status: ObjectiveStatus::Active,
        created_at: 0,
    };
    let kr_row = KeyResultRow {
        id: 1,
        objective_id: 1,
        kpi_name: original.objectives[0].key_results[0].kpi.clone(),
        target_value: original.objectives[0].key_results[0].target,
        comparison: Comparison::Lte,
        weight: original.objectives[0].key_results[0].weight,
    };
    let bundles = vec![(objective_row, vec![kr_row])];

    let rebuilt = render_contract(
        &Some(original.contract.charter.clone()),
        Some(&blob),
        &bundles,
        "01900000-0000-7000-8000-000000000000",
    );
    assert_eq!(rebuilt, original);

    // And once more: serialise → re-parse → still equals the original.
    let toml = toml::to_string_pretty(&rebuilt).expect("serialise");
    let reparsed = parse_contract(&toml, Path::new("round-trip.toml")).expect("reparse");
    assert_eq!(reparsed, original);
}
