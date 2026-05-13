//! Unit tests for [`crate::planner_bias`].

use super::*;
use crate::planner::{PlannerContext, TemplatePlanner};
use argo_types::PlanProvenance;

const RESEARCH_YAML: &str = r#"
id: learned-research
description: Learned research template.
tasks:
  - id: gather
    worker: ""
  - id: write
    worker: ""
    depends_on:
      - gather
"#;

const ALT_YAML: &str = r#"
id: learned-alt
description: Alternative learned template.
tasks:
  - id: pull
    worker: ""
  - id: report
    worker: ""
    depends_on:
      - pull
"#;

const BROKEN_YAML: &str = "this is: not: a: workflow:::\n";

fn ctx() -> PlannerContext {
    PlannerContext::for_worker("wkr-1").with_vertical("ebay-recon")
}

fn candidate(id: &str, anchor: &str, yaml: &str, recency: i64) -> LearnedTemplateCandidate {
    LearnedTemplateCandidate {
        id: id.into(),
        name: format!("learned-{id}"),
        anchor_goal: anchor.into(),
        yaml: yaml.into(),
        recency_micros: recency,
    }
}

fn arc_source(rows: Vec<LearnedTemplateCandidate>) -> std::sync::Arc<dyn LearnedPlanTemplateSource> {
    std::sync::Arc::new(InMemoryTemplateSource::new(rows))
}

fn arc_failing() -> std::sync::Arc<dyn LearnedPlanTemplateSource> {
    std::sync::Arc::new(InMemoryTemplateSource::failing())
}

#[tokio::test]
async fn no_match_falls_through_to_cold_generate() {
    let inner = TemplatePlanner::shipped_defaults();
    let source = arc_source(vec![candidate(
        "tpl-1",
        "summarise this thing", // tokens won't match "promote our launch"
        RESEARCH_YAML,
        1_000,
    )]);
    let bias = PlannerBias::new(inner, source, BiasConfig::default());
    let (result, prov) = bias
        .decompose("promote our newest launch", &ctx())
        .await
        .expect("decompose");
    // Inner shipped marketing-launch template matches the goal, so
    // provenance demotes to BuiltIn — not FromLearnedTemplate.
    match prov {
        PlanProvenance::BuiltIn { template_name } => {
            assert_eq!(template_name, "marketing-launch");
        }
        other => panic!("expected BuiltIn, got {other:?}"),
    }
    assert_eq!(result.template_name.as_deref(), Some("marketing-launch"));
}

#[tokio::test]
async fn single_match_yields_from_learned_template() {
    let inner = TemplatePlanner::shipped_defaults();
    // Anchor goal shares the same token-set as the calling task so
    // `TaskSignature::from_template` and `from_task` produce equal
    // signatures (the planner-bias match criterion).
    let source = arc_source(vec![candidate(
        "tpl-1",
        "research the market",
        RESEARCH_YAML,
        chrono::Utc::now().timestamp_micros(),
    )]);
    let bias = PlannerBias::new(inner, source, BiasConfig::default());
    let (result, prov) = bias
        .decompose("research the market", &ctx())
        .await
        .expect("decompose");
    match prov {
        PlanProvenance::FromLearnedTemplate {
            template_id,
            signature,
        } => {
            assert_eq!(template_id, "tpl-1");
            assert!(signature.contains("vertical=ebay-recon"));
        }
        other => panic!("expected FromLearnedTemplate, got {other:?}"),
    }
    assert_eq!(result.planner_kind, "template_bias");
    assert_eq!(result.workflow.tasks.len(), 2);
}

#[tokio::test]
async fn multiple_matches_pick_newest() {
    let inner = TemplatePlanner::shipped_defaults();
    let now = chrono::Utc::now().timestamp_micros();
    let older = candidate("tpl-old", "research the market", RESEARCH_YAML, now - 10);
    let newer = candidate("tpl-new", "research the market", ALT_YAML, now);
    let source = arc_source(vec![older, newer]);
    let bias = PlannerBias::new(inner, source, BiasConfig::default());
    let (result, prov) = bias
        .decompose("research the market", &ctx())
        .await
        .expect("decompose");
    match prov {
        PlanProvenance::FromLearnedTemplate { template_id, .. } => {
            assert_eq!(template_id, "tpl-new", "newest recency wins");
        }
        other => panic!("expected FromLearnedTemplate, got {other:?}"),
    }
    assert_eq!(result.workflow.id, "learned-alt");
}

#[tokio::test]
async fn signature_stable_across_whitespace_and_order() {
    let a = TaskSignature::from_task("research the market", &ctx());
    let b = TaskSignature::from_task("  RESEARCH    the   market  ", &ctx());
    let c = TaskSignature::from_task("market the research", &ctx());
    assert_eq!(a, b, "whitespace + casing don't change the signature");
    assert_eq!(a, c, "token-order doesn't change the signature");
}

#[tokio::test]
async fn bias_disabled_always_cold_generates() {
    let inner = TemplatePlanner::shipped_defaults();
    let source = arc_source(vec![candidate(
        "tpl-1",
        "research the market",
        RESEARCH_YAML,
        chrono::Utc::now().timestamp_micros(),
    )]);
    let config = BiasConfig {
        enabled: false,
        ..BiasConfig::default()
    };
    let bias = PlannerBias::new(inner, source, config);
    let (_result, prov) = bias
        .decompose("research the market", &ctx())
        .await
        .expect("decompose");
    // Inner shipped 'research-then-summarise' matches, so we get
    // BuiltIn — never FromLearnedTemplate.
    assert!(
        !matches!(prov, PlanProvenance::FromLearnedTemplate { .. }),
        "disabled bias must not surface FromLearnedTemplate; got {prov:?}"
    );
}

#[tokio::test]
async fn provenance_round_trips_through_serde() {
    let inner = TemplatePlanner::shipped_defaults();
    let source = arc_source(vec![candidate(
        "01HQK9N7VV8X4S0M8E0A6R8H3W",
        "research the market",
        RESEARCH_YAML,
        chrono::Utc::now().timestamp_micros(),
    )]);
    let bias = PlannerBias::new(inner, source, BiasConfig::default());
    let (_result, prov) = bias
        .decompose("research the market", &ctx())
        .await
        .expect("decompose");
    let j = serde_json::to_string(&prov).expect("ser");
    let back: PlanProvenance = serde_json::from_str(&j).expect("de");
    assert_eq!(back, prov, "provenance must round-trip through serde");
    assert!(j.contains("from_learned_template"));
}

#[tokio::test]
async fn source_error_falls_through_to_cold_generate() {
    let inner = TemplatePlanner::shipped_defaults();
    let bias = PlannerBias::new(inner, arc_failing(), BiasConfig::default());
    // A degraded source must NOT crash the worker; we expect a
    // successful cold-generate via the inner planner.
    let (result, prov) = bias
        .decompose("research the market", &ctx())
        .await
        .expect("decompose despite source failure");
    assert!(matches!(prov, PlanProvenance::BuiltIn { .. }));
    assert_eq!(
        result.template_name.as_deref(),
        Some("research-then-summarise")
    );
}

#[tokio::test]
async fn recency_filter_skips_stale_candidates() {
    let inner = TemplatePlanner::shipped_defaults();
    // FIXED_NOW chosen well past unix epoch so saturating_sub never
    // underflows past i64::MIN; matches the wall-clock injected via
    // with_now_micros so the test is hermetic.
    const FIXED_NOW: i64 = 2_000_000_000_000_000;
    let day_micros: i64 = 86_400 * 1_000_000;
    let stale = candidate(
        "tpl-stale",
        "research the market",
        RESEARCH_YAML,
        FIXED_NOW - 60 * day_micros,
    );
    let source = arc_source(vec![stale]);
    let config = BiasConfig {
        require_recent_within_days: 30,
        ..BiasConfig::default()
    };
    let bias = PlannerBias::new(inner, source, config).with_now_micros(|| FIXED_NOW);
    let (_result, prov) = bias
        .decompose("research the market", &ctx())
        .await
        .expect("decompose");
    // 60-day-old candidate is outside the 30-day window, so the bias
    // skips it and falls through to the shipped 'research-then-summarise'.
    assert!(
        !matches!(prov, PlanProvenance::FromLearnedTemplate { .. }),
        "stale candidate must be filtered; got {prov:?}"
    );
}

#[tokio::test]
async fn broken_template_yaml_demotes_to_cold_generate() {
    let inner = TemplatePlanner::shipped_defaults();
    let source = arc_source(vec![candidate(
        "tpl-broken",
        "research the market",
        BROKEN_YAML,
        chrono::Utc::now().timestamp_micros(),
    )]);
    let bias = PlannerBias::new(inner, source, BiasConfig::default());
    let (_result, prov) = bias
        .decompose("research the market", &ctx())
        .await
        .expect("decompose despite broken YAML");
    // Bias must NEVER crash the worker on a broken learned template;
    // it falls through to cold-generate.
    assert!(
        !matches!(prov, PlanProvenance::FromLearnedTemplate { .. }),
        "broken-yaml candidate must demote to cold-generate; got {prov:?}"
    );
}
