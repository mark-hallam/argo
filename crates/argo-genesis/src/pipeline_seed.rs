//! Seeded Genesis pipeline — build a plan that inherits a related
//! sibling vertical's action vocabulary + workflow shape instead of
//! starting from a fresh archetype.
//!
//! Lives in its own file so the additive surface stays out of the
//! main [`crate::pipeline`] module (which is already close to the
//! per-file LOC ceiling). The public entry points are re-exported
//! from [`crate::pipeline`] so callers see the unified
//! `argo_genesis::pipeline::*` namespace.
//!
//! Mission: `mission-genesis-transfer-from-tenant`. See the brief in
//! `docs/build-plan/streams/mission-genesis-transfer-from-tenant.md`
//! for the full merge semantics.

use crate::error::GenesisError;
use crate::pipeline::{plan, GenesisPlan};
use argo_vertical::{load_dir, ActionRiskLevel, ActionsSection, VerticalManifest};
use std::path::{Path, PathBuf};

/// Tag prefix stamped onto `manifest.vertical.tags` when a plan is
/// built via [`plan_from_seed_vertical`]. The full tag has the form
/// `"seeded-from:<seed_vertical_id>"`; downstream eval / Forge /
/// quality-regression cohort analysis recovers the seed id via
/// [`seeded_from`].
///
/// Lineage is recorded as a tag because the `VerticalManifest`
/// schema is upstream-frozen for this brief (`manifest.rs` is in
/// the must-not-touch list); tags are the existing persistent +
/// queryable side-channel for cohort metadata.
pub const SEEDED_FROM_TAG_PREFIX: &str = "seeded-from:";

/// Recover the seed vertical id from a manifest's tag list when the
/// plan was produced via [`plan_from_seed_vertical`]. Returns `None`
/// for plans built via [`crate::pipeline::plan`] or
/// [`crate::pipeline::plan_with`].
#[must_use]
pub fn seeded_from(manifest: &VerticalManifest) -> Option<&str> {
    manifest
        .vertical
        .tags
        .iter()
        .find_map(|t| t.strip_prefix(SEEDED_FROM_TAG_PREFIX))
}

/// Build a plan that inherits a related sibling vertical's action
/// vocabulary + workflow shape. Operator picks "make me a new
/// vertical like sales-sdr but for legal" and Genesis seeds from
/// sales-sdr's manifest instead of starting from scratch.
///
/// Pipeline:
///
/// 1. Run [`crate::pipeline::plan`] against the charter + persona
///    to get a baseline plan (the standard archetype path).
/// 2. Load the seed `VerticalManifest` via `argo_vertical::load_dir`
///    against `verticals_root.join(seed_vertical_id)`. Errors
///    clearly when the seed id doesn't exist.
/// 3. Merge the seed's `[actions]`, `[skills]`, `[workflows]` into
///    the baseline plan. Conflict rules:
///    - **Actions**: union by key. For overlapping keys the
///      stricter of the two `risk` classifications wins
///      (`High` > `Medium` > `Low`) and `requires_approval` is
///      OR-ed (either side requiring approval forces it on the
///      merged action). The baseline's description / executor /
///      payload_schema win — operator intent persists.
///    - **Skills / workflows**: inherited from the seed when the
///      baseline has none. When both sides have them the baseline
///      wins outright; merging path-ref lists is out of scope.
/// 4. Stamp the lineage tag (`"seeded-from:<seed_vertical_id>"`)
///    onto `manifest.vertical.tags` so downstream eval / Forge
///    cohort analysis can recover the seed id via [`seeded_from`].
///
/// Returns `Err(GenesisError::Validation { stage:
/// "seed_vertical.load", … })` when the seed id is not a directory
/// under `verticals_root` or its manifest fails to parse / validate.
///
/// `verticals_root` is the discovery directory (usually
/// `argo_vertical::default_dir()`); see also
/// [`plan_from_seed_vertical_in_default_root`] for the CLI shorthand.
pub fn plan_from_seed_vertical(
    charter: &str,
    persona: &str,
    seed_vertical_id: &str,
    verticals_root: &Path,
) -> Result<GenesisPlan, GenesisError> {
    let seed_manifest = load_seed_manifest(seed_vertical_id, verticals_root)?;
    let baseline = plan(charter, persona)?;
    Ok(merge_seed_into_plan(
        baseline,
        &seed_manifest,
        seed_vertical_id,
    ))
}

/// Shorthand for [`plan_from_seed_vertical`] that resolves the
/// verticals root via `argo_vertical::default_dir()` — i.e. the
/// `$ARGO_VERTICALS_DIR` env var, falling back to `./verticals`.
pub fn plan_from_seed_vertical_in_default_root(
    charter: &str,
    persona: &str,
    seed_vertical_id: &str,
) -> Result<GenesisPlan, GenesisError> {
    let root = argo_vertical::default_dir();
    plan_from_seed_vertical(charter, persona, seed_vertical_id, &root)
}

fn load_seed_manifest(
    seed_id: &str,
    verticals_root: &Path,
) -> Result<VerticalManifest, GenesisError> {
    let seed_dir: PathBuf = verticals_root.join(seed_id);
    if !seed_dir.is_dir() {
        return Err(GenesisError::Validation {
            stage: "seed_vertical.load",
            detail: format!(
                "seed vertical '{seed_id}' not found at {}",
                seed_dir.display()
            ),
        });
    }
    let (registry, errors) = load_dir(verticals_root).map_err(|e| GenesisError::Validation {
        stage: "seed_vertical.load",
        detail: format!(
            "verticals root {} failed to load: {e}",
            verticals_root.display()
        ),
    })?;
    if let Some(err) = errors.iter().find(|e| e.dir == seed_dir) {
        return Err(GenesisError::Validation {
            stage: "seed_vertical.load",
            detail: format!("seed vertical '{seed_id}' failed to load: {}", err.error),
        });
    }
    registry
        .get(seed_id)
        .map(|loaded| loaded.manifest.clone())
        .ok_or_else(|| GenesisError::Validation {
            stage: "seed_vertical.load",
            detail: format!(
                "seed vertical '{seed_id}' not registered after load (id mismatch with directory?)"
            ),
        })
}

fn merge_seed_into_plan(
    mut baseline: GenesisPlan,
    seed: &VerticalManifest,
    seed_id: &str,
) -> GenesisPlan {
    if let Some(seed_actions) = seed.actions.as_ref() {
        let baseline_actions = baseline
            .manifest
            .actions
            .get_or_insert_with(ActionsSection::default);
        merge_actions(baseline_actions, seed_actions);
    }
    if baseline.manifest.skills.is_none() {
        baseline.manifest.skills = seed.skills.clone();
    }
    if baseline.manifest.workflows.is_none() {
        baseline.manifest.workflows = seed.workflows.clone();
    }
    stamp_seeded_from(&mut baseline.manifest, seed_id);
    baseline
}

fn merge_actions(baseline: &mut ActionsSection, seed: &ActionsSection) {
    for (key, seed_decl) in &seed.actions {
        match baseline.actions.get_mut(key) {
            Some(base_decl) => {
                // Strictness ordering: High > Medium > Low. The
                // stricter side wins — covers both "seed wins on
                // risk classification" (when seed is stricter) and
                // the implementer note "if the operator specifies a
                // stricter risk, the operator's choice must persist".
                base_decl.risk = strictest_risk(base_decl.risk, seed_decl.risk);
                base_decl.requires_approval =
                    base_decl.requires_approval || seed_decl.requires_approval;
                // description / executor / payload_schema: the
                // baseline wins so operator intent persists.
            }
            None => {
                baseline.actions.insert(key.clone(), seed_decl.clone());
            }
        }
    }
}

fn strictest_risk(a: ActionRiskLevel, b: ActionRiskLevel) -> ActionRiskLevel {
    if risk_rank(a) >= risk_rank(b) {
        a
    } else {
        b
    }
}

fn risk_rank(r: ActionRiskLevel) -> u8 {
    match r {
        ActionRiskLevel::Low => 0,
        ActionRiskLevel::Medium => 1,
        ActionRiskLevel::High => 2,
    }
}

fn stamp_seeded_from(manifest: &mut VerticalManifest, seed_id: &str) {
    let tag = format!("{SEEDED_FROM_TAG_PREFIX}{seed_id}");
    if !manifest.vertical.tags.iter().any(|t| t == &tag) {
        manifest.vertical.tags.push(tag);
    }
}

#[cfg(test)]
#[path = "pipeline_seed_tests.rs"]
mod tests;
