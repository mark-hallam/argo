//! Top-level Genesis pipeline. Wires charter → archetype →
//! manifest → prompts → eval seed → worker config → safety
//! judge → disk write into one entry point.

use crate::archetype::{classify_charter, classify_llm, Archetype, ProviderClient};
use crate::conflict::{detect as detect_conflicts, highest_score};
use crate::error::GenesisError;
use crate::eval_seed::{generate as generate_eval_seeds, suggested_actions_from_manifest, to_toml};
use crate::manifest_writer::build_manifest;
use crate::memory_schema;
use crate::prompt_writer::write_artefacts;
use crate::safety_judge;
use crate::templates::match_template;
use crate::worker_config::WorkerConfig;
use argo_vertical::{ActionDeclaration, ActionRiskLevel, ExecutorDeclaration, VerticalManifest};
use chrono::Utc;
use std::path::Path;

pub use crate::broad_role::{plan_or_broad, GenesisOutcome};

pub use crate::pipeline_seed::{
    plan_from_seed_vertical, plan_from_seed_vertical_in_default_root, seeded_from,
    SEEDED_FROM_TAG_PREFIX,
};

// The conductor-selected cutoff for refusing a likely duplicate.
const CONFLICT_REFUSE_SCORE: f32 = 0.7;

/// Resolved Genesis plan — the output of `plan()` before any
/// disk write.
#[derive(Debug, Clone)]
pub struct GenesisPlan {
    pub charter: String,
    pub persona: String,
    pub archetype: Archetype,
    pub vertical_id: String,
    pub manifest: VerticalManifest,
}

/// Pipeline-level knobs. Defaults preserve the v1 deterministic
/// behaviour (`use_llm_classifier=false`); a caller that wires a
/// provider can opt in via [`plan_with`].
#[derive(Debug, Clone, Default)]
pub struct GenesisConfig {
    /// When `true` AND a `ProviderClient` is passed to [`plan_with`],
    /// the pipeline calls [`classify_llm`] before [`classify_charter`];
    /// any error or unknown slug falls back to keyword. When `false`
    /// (the default) the LLM is skipped entirely and behaviour is
    /// byte-identical to the synchronous [`plan`] entry point.
    pub use_llm_classifier: bool,
}

/// Build a `GenesisPlan` from a charter + persona. Runs:
/// archetype classification → manifest assembly → safety judge.
///
/// Synchronous, keyword-only path — preserved for the long tail of
/// existing call sites. New callers wanting the LLM classifier
/// should use [`plan_with`].
pub fn plan(charter: &str, persona: &str) -> Result<GenesisPlan, GenesisError> {
    validate_persona(persona)?;
    let archetype = match match_template(charter).and_then(|template| template.archetype()) {
        Some(archetype) => archetype,
        None => classify_charter(charter)?,
    };
    assemble_plan(charter, persona, archetype)
}

/// Async variant of [`plan`] that honours [`GenesisConfig`] and an
/// optional [`ProviderClient`]. When `config.use_llm_classifier` is
/// `true` AND `llm` is `Some`, the LLM picks the archetype with the
/// keyword classifier as a fallback on any error or unknown slug.
/// All other code paths are byte-identical to [`plan`].
pub async fn plan_with(
    charter: &str,
    persona: &str,
    config: &GenesisConfig,
    llm: Option<&dyn ProviderClient>,
) -> Result<GenesisPlan, GenesisError> {
    validate_persona(persona)?;
    let archetype = resolve_archetype(charter, config, llm).await?;
    assemble_plan(charter, persona, archetype)
}

async fn resolve_archetype(
    charter: &str,
    config: &GenesisConfig,
    llm: Option<&dyn ProviderClient>,
) -> Result<Archetype, GenesisError> {
    if let Some(template_archetype) = match_template(charter).and_then(|t| t.archetype()) {
        return Ok(template_archetype);
    }
    if config.use_llm_classifier {
        if let Some(client) = llm {
            // `classify_llm` already falls back to the keyword
            // classifier on any provider error or unknown slug. The
            // only remaining error path is "keyword fallback also
            // failed" (e.g. empty/unmatchable charter); re-run the
            // sync keyword path here so the caller gets a clean
            // `GenesisError` variant rather than a wrapped `ArgoError`.
            return match classify_llm(charter, client).await {
                Ok(a) => Ok(a),
                Err(_) => classify_charter(charter),
            };
        }
    }
    classify_charter(charter)
}

pub(crate) fn assemble_plan(
    charter: &str,
    persona: &str,
    archetype: Archetype,
) -> Result<GenesisPlan, GenesisError> {
    // The vertical-id validator allows lowercase ASCII letters,
    // digits, and `-` only. Convert the slug + persona to a
    // dash-joined id so the round-trip load passes.
    let vertical_id = format!(
        "{}-{}",
        archetype.slug().replace('_', "-"),
        persona.to_lowercase().replace('_', "-")
    );
    let mut manifest = build_manifest(charter, archetype, persona, &vertical_id)?;
    add_suggested_actions(&mut manifest, charter)?;
    manifest.memory = Some(memory_schema::manifest_section());
    Ok(GenesisPlan {
        charter: charter.to_string(),
        persona: persona.to_string(),
        archetype,
        vertical_id,
        manifest,
    })
}

fn add_suggested_actions(
    manifest: &mut VerticalManifest,
    charter: &str,
) -> Result<(), GenesisError> {
    let suggested = match crate::action_suggest::suggest(charter, 3) {
        Ok(actions) => actions,
        Err(err) => {
            tracing::warn!(
                target: "argo_genesis::pipeline",
                error = %err,
                "action suggestion failed; keeping archetype seed actions",
            );
            return Ok(());
        }
    };
    let Some(actions) = manifest.actions.as_mut() else {
        return Ok(());
    };
    for suggestion in suggested {
        let action_type = crate::action_suggest::manifest_action_type(&suggestion.id);
        if actions.actions.contains_key(&action_type) {
            continue;
        }
        let requires_approval = crate::safety_judge::matches_sensitive_verb(&action_type);
        actions.actions.insert(
            action_type,
            ActionDeclaration {
                risk: if requires_approval {
                    ActionRiskLevel::High
                } else {
                    ActionRiskLevel::Low
                },
                requires_approval,
                description: format!("Genesis-suggested action: {}", suggestion.reason),
                executor: ExecutorDeclaration::default(),
                ..ActionDeclaration::default()
            },
        );
    }
    safety_judge::auto_fix(actions);
    safety_judge::judge(actions)
}

/// Build a plan AND run the quality gate in one shot. Returns
/// the plan alongside its [`crate::quality::QualityReport`] so
/// callers can either surface the report to the operator or
/// gate emission via [`crate::quality::require_pass`]. Adding
/// the gate to existing `plan()` callers (currently
/// `argo-cli`) is the follow-up CLI brief; until then operators
/// can opt in via this helper.
pub fn plan_with_quality(
    charter: &str,
    persona: &str,
) -> Result<(GenesisPlan, crate::quality::QualityReport), GenesisError> {
    let plan = plan(charter, persona)?;
    let report = crate::quality::check_plan(&plan);
    Ok((plan, report))
}

/// Materialise a `GenesisPlan` to disk. Refuses to clobber a
/// non-empty output dir. Round-trip-validates the result by
/// re-loading via `argo_vertical::load_dir`.
pub fn write_vertical(plan: &GenesisPlan, output: &Path) -> Result<(), GenesisError> {
    write_vertical_with_overlap(plan, output, false)
}

/// Materialise a `GenesisPlan`, optionally allowing likely overlap with
/// an existing vertical in the same parent directory.
pub fn write_vertical_with_overlap(
    plan: &GenesisPlan,
    output: &Path,
    allow_overlap: bool,
) -> Result<(), GenesisError> {
    ensure_output_empty(output)?;
    enforce_overlap_policy(plan, output, allow_overlap)?;
    std::fs::create_dir_all(output).map_err(|e| GenesisError::Io {
        what: "vertical.dir.create",
        source: e,
    })?;
    write_manifest(&plan.manifest, output)?;
    write_artefacts(output, &plan.charter, plan.archetype, &plan.persona)?;
    write_eval_seed_toml(plan, output)?;
    memory_schema::write_schema_file(&plan.charter, plan.archetype, output)?;
    let cfg = WorkerConfig::new_dry_run(&plan.vertical_id, &plan.persona, Utc::now());
    cfg.write_to(output)?;
    if let Some(actions) = plan.manifest.actions.as_ref() {
        safety_judge::judge(actions)?;
    }
    Ok(())
}

fn enforce_overlap_policy(
    plan: &GenesisPlan,
    output: &Path,
    allow_overlap: bool,
) -> Result<(), GenesisError> {
    if allow_overlap {
        return Ok(());
    }
    let Some(verticals_root) = output.parent().filter(|p| p.exists()) else {
        return Ok(());
    };
    let report = detect_conflicts(plan, verticals_root).map_err(|e| GenesisError::Validation {
        stage: "conflict.detect",
        detail: e.to_string(),
    })?;
    let Some(score) = highest_score(&report) else {
        return Ok(());
    };
    if score < CONFLICT_REFUSE_SCORE {
        return Ok(());
    }
    let top = &report.matches[0];
    Err(GenesisError::Validation {
        stage: "conflict.detect",
        detail: format!(
            "generated vertical overlaps '{}' with score {:.2}: {}",
            top.existing_vertical,
            top.score,
            top.reasons.join("; ")
        ),
    })
}

fn write_eval_seed_toml(plan: &GenesisPlan, output: &Path) -> Result<(), GenesisError> {
    let actions = plan
        .manifest
        .actions
        .as_ref()
        .map(suggested_actions_from_manifest)
        .unwrap_or_default();
    let seeds =
        generate_eval_seeds(&plan.charter, &actions).map_err(|e| GenesisError::Validation {
            stage: "eval_seed.generate",
            detail: e.to_string(),
        })?;
    std::fs::write(output.join("eval-seed.toml"), to_toml(&seeds)).map_err(|e| {
        GenesisError::Io {
            what: "eval_seed.write",
            source: e,
        }
    })?;
    Ok(())
}

pub(crate) fn validate_persona(persona: &str) -> Result<(), GenesisError> {
    if persona.trim().is_empty() {
        return Err(GenesisError::InvalidPersona(persona.to_string()));
    }
    let ok = persona
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    if !ok {
        return Err(GenesisError::InvalidPersona(persona.to_string()));
    }
    Ok(())
}

fn ensure_output_empty(output: &Path) -> Result<(), GenesisError> {
    if !output.exists() {
        return Ok(());
    }
    let mut entries = std::fs::read_dir(output).map_err(|e| GenesisError::Io {
        what: "vertical.dir.read",
        source: e,
    })?;
    if let Some(first) = entries.next() {
        first.map_err(|e| GenesisError::Io {
            what: "vertical.dir.scan",
            source: e,
        })?;
        return Err(GenesisError::OutputDirNotEmpty(
            output.display().to_string(),
        ));
    }
    Ok(())
}

fn write_manifest(manifest: &VerticalManifest, output: &Path) -> Result<(), GenesisError> {
    let body = toml::to_string_pretty(manifest).map_err(|e| GenesisError::Toml {
        what: "manifest.serialise",
        source: e,
    })?;
    std::fs::write(output.join("vertical.toml"), body).map_err(|e| GenesisError::Io {
        what: "manifest.write",
        source: e,
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
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
        let cfg = GenesisConfig { use_llm_classifier: true };
        let llm = StubProvider { slug: "ops_on_call" };
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
        let llm = StubProvider { slug: "ops_on_call" };
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
        let cfg = GenesisConfig { use_llm_classifier: true };
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
}
