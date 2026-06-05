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
///
/// The `archetype` field is **always** populated: for the archetype path it
/// is the classified archetype; for the from-scratch
/// ([`crate::materialize_spec`]) path it is the spec's nearest-archetype
/// *prior* (DEC-025 #3 hybrid) so the SSE payload + audit carry a stable
/// slug. The optional `spec` distinguishes the two: when `Some`, the plan was
/// synthesized from a [`crate::CapabilitySpec`] and the disk writer uses the
/// archetype-free companion-file writers; when `None`, the archetype seed
/// templates are used.
#[derive(Debug, Clone)]
pub struct GenesisPlan {
    pub charter: String,
    pub persona: String,
    pub archetype: Archetype,
    pub vertical_id: String,
    pub manifest: VerticalManifest,
    /// The typed capability spec this plan was materialized from, when the
    /// from-scratch path produced it. `None` for the archetype path.
    pub spec: Option<crate::spec::CapabilitySpec>,
}

impl GenesisPlan {
    /// Re-sync the manifest's `[actions]` descriptions from this plan's
    /// [`crate::CapabilitySpec`] capability intents.
    ///
    /// The from-scratch (broad-role) path materializes the manifest from the
    /// spec's *templated* fallback intents, then an optional LLM-authoring pass
    /// ([`crate::apply_authored`]) enriches `spec.capabilities[].intent` with a
    /// specific, role-aware one-liner per action. Because the manifest was
    /// already built, the operator-visible action descriptions — the Genesis
    /// review panel derives them from the manifest's `[actions]` via
    /// [`crate::RoleContract::derive_from_manifest`] — would otherwise keep the
    /// fallback text and the LLM author's prose would never reach the screen.
    /// Call this *after* applying authored intents so the review panel shows the
    /// author's description when present.
    ///
    /// No-op when the plan carries no spec (the archetype path) or no
    /// `[actions]` block. Only the human-readable `description` is rewritten —
    /// risk / approval / executor are untouched, so the governance
    /// `contract_digest` (which hashes the gated surface, not the prose) is
    /// unaffected.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn demo() -> Result<(), argo_genesis::GenesisError> {
    /// let mut plan = argo_genesis::plan("be my marketing intern", "maya")?;
    /// // After an LLM-authoring pass has updated `plan.spec`, push the
    /// // authored per-action prose into the operator-visible manifest.
    /// plan.sync_action_descriptions_from_spec();
    /// # Ok(())
    /// # }
    /// ```
    pub fn sync_action_descriptions_from_spec(&mut self) {
        let Some(spec) = self.spec.as_ref() else {
            return;
        };
        let Some(actions) = self.manifest.actions.as_mut() else {
            return;
        };
        for cap in &spec.capabilities {
            if let Some(decl) = actions.actions.get_mut(&cap.action_name) {
                decl.description.clone_from(&cap.intent);
            }
        }
    }
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
        spec: None,
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

/// Like [`write_vertical`], but when `overwrite=true` recursively
/// removes a pre-existing `output` before the write.
pub fn write_vertical_with_overwrite(
    plan: &GenesisPlan,
    output: &Path,
    overwrite: bool,
) -> Result<(), GenesisError> {
    if overwrite && output.exists() {
        std::fs::remove_dir_all(output).map_err(|e| GenesisError::Io {
            what: "vertical.dir.overwrite",
            source: e,
        })?;
    }
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
    // From-scratch (spec) path vs archetype path: the companion files
    // (prompts + memory schema + eval/seed.jsonl) are keyed on the spec when
    // present, so a novel role is NOT jammed into an archetype template.
    if let Some(spec) = plan.spec.as_ref() {
        crate::materialize_writer::write_spec_artefacts(output, spec)?;
    } else {
        write_artefacts(output, &plan.charter, plan.archetype, &plan.persona)?;
        memory_schema::write_schema_file(&plan.charter, plan.archetype, output)?;
    }
    // The eval-seed.toml the hire gate grades is generated from the manifest
    // actions (identical for both paths).
    write_eval_seed_toml(plan, output)?;
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
#[path = "pipeline_tests.rs"]
mod tests;
