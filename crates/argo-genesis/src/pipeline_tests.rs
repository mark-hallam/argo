//! Tests for the top-level Genesis pipeline. Extracted from `pipeline.rs`
//! to keep that file under the per-file 500-LOC budget (the crate's
//! established `#[path = "..._tests.rs"]` pattern).

use super::*;

#[test]
fn plan_for_marketing_charter_resolves_archetype_and_passes_safety() {
    let p = plan(
        "be my marketing intern, run weekly campaigns to cold leads",
        "maya",
    )
    .expect("plan builds");
    assert_eq!(p.archetype, Archetype::MarketingIntern);
    assert_eq!(p.persona, "maya");
    assert!(p.vertical_id.starts_with("marketing-intern-"));
}

#[test]
fn plan_rejects_empty_charter() {
    let err = plan("   ", "maya").expect_err("empty charter");
    assert!(matches!(err, GenesisError::EmptyCharter));
}

#[test]
fn plan_rejects_invalid_persona_with_special_chars() {
    let err = plan("be my marketing intern", "maya@").expect_err("bad persona");
    assert!(matches!(err, GenesisError::InvalidPersona(_)));
}

#[test]
fn write_vertical_creates_full_layout_and_passes_round_trip() {
    let p = plan(
        "be my marketing intern, run weekly campaigns to cold leads",
        "maya",
    )
    .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join(&p.vertical_id);
    write_vertical(&p, &out).expect("write succeeds");
    for relpath in [
        "vertical.toml",
        "worker.toml",
        "eval-seed.toml",
        "prompts/system.md",
        "prompts/persona.md",
        "memory/schema.toml",
        "eval/seed.jsonl",
    ] {
        assert!(out.join(relpath).exists(), "missing: {relpath}");
    }
    let (registry, errors) = argo_vertical::load_dir(dir.path()).expect("load_dir");
    assert!(errors.is_empty(), "load errors: {errors:?}");
    let loaded = registry.get(&p.vertical_id).expect("vertical loaded by id");
    assert_eq!(loaded.manifest.vertical.id, p.vertical_id);
}

#[test]
fn write_vertical_refuses_to_clobber_existing_dir() {
    let p = plan("be my marketing intern", "maya").unwrap();
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join(&p.vertical_id);
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join("preexisting.txt"), "do not delete").unwrap();
    let err = write_vertical(&p, &out).expect_err("must refuse");
    assert!(matches!(err, GenesisError::OutputDirNotEmpty(_)));
}

#[test]
fn write_vertical_with_overwrite_false_refuses_existing_dir() {
    let p = plan("be my marketing intern", "maya").unwrap();
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join(&p.vertical_id);
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join("preexisting.txt"), "keep me").unwrap();
    let err = write_vertical_with_overwrite(&p, &out, false).expect_err("must refuse");
    assert!(matches!(err, GenesisError::OutputDirNotEmpty(_)));
    assert!(out.join("preexisting.txt").exists(), "must not delete");
}

#[test]
fn write_vertical_with_overwrite_true_wipes_then_writes() {
    let p = plan("be my marketing intern", "maya").unwrap();
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join(&p.vertical_id);
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join("stale.txt"), "stale").unwrap();
    write_vertical_with_overwrite(&p, &out, true).expect("overwrite succeeds");
    assert!(!out.join("stale.txt").exists(), "pre-existing file wiped");
    assert!(out.join("vertical.toml").exists(), "fresh manifest written");
}

// ---- LLM-classifier dispatch (mission-genesis-llm-classifier) ----

use argo_types::ArgoResult;
use async_trait::async_trait;

struct StubProvider {
    slug: &'static str,
}

#[async_trait]
impl ProviderClient for StubProvider {
    async fn classify(&self, _prompt: &str) -> ArgoResult<String> {
        Ok(self.slug.to_string())
    }
}

#[tokio::test]
async fn plan_with_uses_llm_classifier_when_enabled() {
    // Charter would classify to MarketingIntern via keyword; the
    // LLM stub forces a different archetype to prove the LLM path
    // ran instead of the keyword fallback.
    let cfg = GenesisConfig {
        use_llm_classifier: true,
    };
    let llm = StubProvider {
        slug: "ops_on_call",
    };
    let p = plan_with(
        "be my marketing intern, run weekly campaigns to cold leads",
        "maya",
        &cfg,
        Some(&llm),
    )
    .await
    .expect("plan_with builds");
    assert_eq!(p.archetype, Archetype::OpsOnCall);
}

#[tokio::test]
async fn plan_with_falls_back_to_keyword_when_llm_disabled() {
    let cfg = GenesisConfig::default(); // use_llm_classifier=false
    let llm = StubProvider {
        slug: "ops_on_call",
    };
    let p = plan_with(
        "be my marketing intern, run weekly campaigns to cold leads",
        "maya",
        &cfg,
        Some(&llm),
    )
    .await
    .expect("plan_with builds");
    assert_eq!(p.archetype, Archetype::MarketingIntern);
}

#[tokio::test]
async fn plan_with_falls_back_to_keyword_when_no_provider() {
    let cfg = GenesisConfig {
        use_llm_classifier: true,
    };
    let p = plan_with(
        "be my marketing intern, run weekly campaigns to cold leads",
        "maya",
        &cfg,
        None,
    )
    .await
    .expect("plan_with builds");
    assert_eq!(p.archetype, Archetype::MarketingIntern);
}
