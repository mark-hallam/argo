//! Schema types, parser, validator, and render helpers for
//! `argo contract apply/validate/show`. Lives in a sibling file so
//! `contract.rs` stays under the 500-LOC per-file ceiling — same
//! pattern used by `worker_charter.rs` / `worker_okr_types.rs`.
//!
//! Persisted-state mapping (the JSON blob on the worker row's `kpis`
//! column) is encoded by [`build_kpis_blob`] / [`render_contract`] —
//! the inverse pair `argo contract apply` and `argo contract show`
//! depend on for the round-trip acceptance test.

use argo_store::worker_okr::{Comparison, KeyResultRow, KeyResultSpec, ObjectiveRow};
use argo_types::{ArgoError, ArgoResult};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::Path;

// ───────────────────────── TOML schema ─────────────────────────

/// Top-level shape of a `<role>.contract.toml` file. All collection
/// fields default to empty so a contract may declare just a charter
/// (the minimum viable delegation).
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub(crate) struct ContractFile {
    pub(crate) contract: ContractMeta,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) duties: Vec<DutyEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) kpis: Vec<KpiEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) objectives: Vec<ObjectiveEntry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) success_criteria: Option<SuccessCriteria>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub(crate) struct ContractMeta {
    pub(crate) role: String,
    pub(crate) charter: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub(crate) struct DutyEntry {
    pub(crate) id: String,
    pub(crate) description: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub(crate) struct KpiEntry {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) target: f64,
    pub(crate) comparison: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub(crate) struct ObjectiveEntry {
    pub(crate) period: String,
    pub(crate) title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) key_results: Vec<KeyResultEntry>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub(crate) struct KeyResultEntry {
    pub(crate) kpi: String,
    pub(crate) target: f64,
    pub(crate) comparison: String,
    pub(crate) weight: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub(crate) struct SuccessCriteria {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) min_sessions: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) min_success_rate: Option<f64>,
}

// ───────────────────────── parse + validate ─────────────────────────

/// Read the file off disk and run schema-level checks. The handler's
/// `apply` / `validate` paths share this entry point.
pub(crate) fn read_and_validate(file: &Path) -> ArgoResult<ContractFile> {
    let body = std::fs::read_to_string(file).map_err(|e| {
        ArgoError::Provider(format!("could not read contract at {}: {e}", file.display()))
    })?;
    parse_contract(&body, file)
}

/// Parse the in-memory body and check it. Surfaces line numbers for
/// malformed TOML and structured semantic errors for invalid
/// comparisons / non-positive weights / empty fields.
pub(crate) fn parse_contract(body: &str, file: &Path) -> ArgoResult<ContractFile> {
    let parsed: ContractFile = toml::from_str(body).map_err(|e| {
        let location = e
            .span()
            .map(|s| {
                let line = body[..s.start].matches('\n').count() + 1;
                let col = body[..s.start]
                    .rfind('\n')
                    .map(|nl| s.start - nl)
                    .unwrap_or(s.start + 1);
                format!("{}:{line}:{col}", file.display())
            })
            .unwrap_or_else(|| file.display().to_string());
        ArgoError::Schema(format!("{location}: {}", e.message()))
    })?;
    semantic_check(&parsed)?;
    Ok(parsed)
}

fn semantic_check(c: &ContractFile) -> ArgoResult<()> {
    if c.contract.role.trim().is_empty() {
        return Err(ArgoError::Schema("[contract].role must not be empty".into()));
    }
    if c.contract.charter.trim().is_empty() {
        return Err(ArgoError::Schema(
            "[contract].charter must not be empty".into(),
        ));
    }
    for (i, duty) in c.duties.iter().enumerate() {
        if duty.id.trim().is_empty() {
            return Err(ArgoError::Schema(format!(
                "[[duties]][{i}].id must not be empty"
            )));
        }
    }
    for (i, k) in c.kpis.iter().enumerate() {
        if k.name.trim().is_empty() {
            return Err(ArgoError::Schema(format!(
                "[[kpis]][{i}].name must not be empty"
            )));
        }
        Comparison::parse(&k.comparison)
            .map_err(|e| ArgoError::Schema(format!("[[kpis]][{i}].comparison: {e}")))?;
        if !k.target.is_finite() {
            return Err(ArgoError::Schema(format!(
                "[[kpis]][{i}].target must be finite"
            )));
        }
    }
    for (i, o) in c.objectives.iter().enumerate() {
        if o.title.trim().is_empty() {
            return Err(ArgoError::Schema(format!(
                "[[objectives]][{i}].title must not be empty"
            )));
        }
        if o.key_results.is_empty() {
            return Err(ArgoError::Schema(format!(
                "[[objectives]][{i}] must declare at least one key_result"
            )));
        }
        for (j, kr) in o.key_results.iter().enumerate() {
            if kr.kpi.trim().is_empty() {
                return Err(ArgoError::Schema(format!(
                    "[[objectives]][{i}].key_results[{j}].kpi must not be empty"
                )));
            }
            Comparison::parse(&kr.comparison).map_err(|e| {
                ArgoError::Schema(format!(
                    "[[objectives]][{i}].key_results[{j}].comparison: {e}"
                ))
            })?;
            if !kr.target.is_finite() {
                return Err(ArgoError::Schema(format!(
                    "[[objectives]][{i}].key_results[{j}].target must be finite"
                )));
            }
            if !kr.weight.is_finite() || kr.weight <= 0.0 {
                return Err(ArgoError::Schema(format!(
                    "[[objectives]][{i}].key_results[{j}].weight must be > 0"
                )));
            }
        }
    }
    if let Some(sc) = &c.success_criteria {
        if let Some(rate) = sc.min_success_rate {
            if !(0.0..=1.0).contains(&rate) {
                return Err(ArgoError::Schema(
                    "[success_criteria].min_success_rate must be in [0, 1]".into(),
                ));
            }
        }
    }
    Ok(())
}

// ───────────────────────── apply plan + KR conversion ─────────────────────────

#[derive(Debug, Default)]
pub(crate) struct ApplyPlan {
    pub(crate) duty_count: usize,
    pub(crate) kpi_count: usize,
    pub(crate) objective_count: usize,
    pub(crate) kr_count: usize,
}

impl From<&ContractFile> for ApplyPlan {
    fn from(c: &ContractFile) -> Self {
        Self {
            duty_count: c.duties.len(),
            kpi_count: c.kpis.len(),
            objective_count: c.objectives.len(),
            kr_count: c.objectives.iter().map(|o| o.key_results.len()).sum(),
        }
    }
}

/// Convert a parsed `[[objectives.key_results]]` entry into the
/// `--kr` shape `worker_okr::set_objective` already accepts.
pub(crate) fn kr_entry_to_spec(kr: &KeyResultEntry) -> ArgoResult<KeyResultSpec> {
    let comparison = Comparison::parse(&kr.comparison)?;
    if !kr.target.is_finite() {
        return Err(ArgoError::Schema(format!(
            "key_result for kpi '{}': target must be finite",
            kr.kpi
        )));
    }
    if !kr.weight.is_finite() || kr.weight <= 0.0 {
        return Err(ArgoError::Schema(format!(
            "key_result for kpi '{}': weight must be > 0",
            kr.kpi
        )));
    }
    Ok(KeyResultSpec {
        kpi_name: kr.kpi.clone(),
        comparison,
        target_value: kr.target,
        weight: kr.weight,
    })
}

// ───────────────────────── persisted blob round-trip ─────────────────────────

/// Bundle the contract's non-charter, non-objective state into the
/// JSON blob persisted on the worker row's `kpis` column. This is
/// the source of truth for round-tripping `argo contract show`.
pub(crate) fn build_kpis_blob(c: &ContractFile) -> Value {
    json!({
        "role": c.contract.role,
        "duties": c.duties,
        "kpis": c.kpis,
        "success_criteria": c.success_criteria,
    })
}

/// Inverse of [`build_kpis_blob`]: rebuild a `ContractFile` from the
/// charter markdown body, the persisted kpis JSON blob, and the
/// objective/KR rows. Missing pieces fall back to sensible defaults
/// so a partially-populated worker still renders cleanly.
pub(crate) fn render_contract(
    charter: &Option<String>,
    kpis: Option<&Value>,
    bundles: &[(ObjectiveRow, Vec<KeyResultRow>)],
    worker_id: &str,
) -> ContractFile {
    let role = kpis
        .and_then(|v| v.get("role"))
        .and_then(Value::as_str)
        .unwrap_or(worker_id)
        .to_string();
    let duties: Vec<DutyEntry> = kpis
        .and_then(|v| v.get("duties"))
        .map(|v| serde_json::from_value(v.clone()).unwrap_or_default())
        .unwrap_or_default();
    let kpi_defs: Vec<KpiEntry> = kpis
        .and_then(|v| v.get("kpis"))
        .map(|v| serde_json::from_value(v.clone()).unwrap_or_default())
        .unwrap_or_default();
    let success_criteria: Option<SuccessCriteria> = kpis
        .and_then(|v| v.get("success_criteria"))
        .and_then(|v| serde_json::from_value(v.clone()).ok());

    let objectives = bundles
        .iter()
        .map(|(obj, krs)| ObjectiveEntry {
            period: obj.period.clone(),
            title: obj.title.clone(),
            description: obj.description.clone(),
            key_results: krs
                .iter()
                .map(|kr| KeyResultEntry {
                    kpi: kr.kpi_name.clone(),
                    target: kr.target_value,
                    comparison: kr.comparison.as_str().to_string(),
                    weight: kr.weight,
                })
                .collect(),
        })
        .collect();

    ContractFile {
        contract: ContractMeta {
            role,
            charter: charter.clone().unwrap_or_default(),
        },
        duties,
        kpis: kpi_defs,
        objectives,
        success_criteria,
    }
}
