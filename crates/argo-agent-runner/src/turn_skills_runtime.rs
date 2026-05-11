//! mission-skill-select-variant-runtime-wire — wire skill A/B
//! `select_variant` + `record_outcome` into the runtime skill load.
//!
//! Loaded via `#[path = "turn_skills_runtime.rs"] mod
//! turn_skills_runtime;` from `turn.rs` (mirrors the
//! `turn_memory.rs` / `turn_canary.rs` sibling pattern). Keeps the
//! resolve-and-pick + post-turn record_outcome logic out of `turn.rs`
//! so that file stays under the 500-LOC HARD RULE 6 ceiling.
//!
//! ## What this module does
//!
//! 1. After active skills are loaded for a turn, group them by
//!    `name`. For every name with ≥2 active rows (the A/B condition):
//!    look up the most recent `status = 'running'` row in
//!    `skill_ab_test_run`. If one exists, call [`select_variant`] to
//!    deterministically pick A or B and replace the multi-variant
//!    list with the chosen single skill.
//! 2. Queue a [`PendingAbOutcome`] per selection so the post-turn
//!    hook can call [`record_outcome`] when success/failure is known.
//!
//! Single-variant case: no `select_variant` call, no
//! `PendingAbOutcome` — the skill flows through unchanged.
//!
//! Error handling: lookup errors warn-and-return-`None` (treated as
//! "no active run"); `record_outcome` errors warn but never fail the
//! turn — the brief is explicit that this is a side channel.

use argo_skill::{record_outcome, select_variant, Skill, Variant};
use argo_store::SessionStore;
use sqlx::Row;
use std::collections::BTreeMap;

/// One A/B selection that needs an outcome recorded after the turn
/// settles. Carried out of [`resolve_variants_for_turn`] and back
/// into [`record_pending_outcomes`] by the caller.
#[derive(Debug, Clone)]
pub(crate) struct PendingAbOutcome {
    /// `skill_ab_test_run.id` — the run the outcome is bound to.
    pub run_id: String,
    /// Turn-stable invocation id; SHA-256 seed for [`select_variant`]
    /// and the `(run_id, invocation_id)` dedup key for
    /// [`record_outcome`].
    pub invocation_id: String,
    /// Which variant got picked (A or B). Stored so the post-turn
    /// hook writes the same arm that was actually exposed to the
    /// model.
    pub variant: Variant,
}

/// Resolve A/B variants out of an `active` skill list.
///
/// For each `name` with ≥2 active rows AND an open
/// `skill_ab_test_run`: pick A or B deterministically via
/// [`select_variant`] and keep only the chosen variant. Names with
/// one variant — or names with ≥2 variants but no open run — pass
/// through untouched.
///
/// Returns the filtered skill list + the pending outcomes to record
/// after the turn settles.
pub(crate) async fn resolve_variants_for_turn(
    store: &SessionStore,
    invocation_id: &str,
    active: Vec<Skill>,
) -> (Vec<Skill>, Vec<PendingAbOutcome>) {
    let mut groups: BTreeMap<String, Vec<Skill>> = BTreeMap::new();
    for s in active {
        groups.entry(s.frontmatter.name.clone()).or_default().push(s);
    }
    let mut out = Vec::new();
    let mut pending = Vec::new();
    for (name, mut variants) in groups {
        if variants.len() <= 1 {
            out.extend(variants);
            continue;
        }
        match find_active_ab_run(store, &name).await {
            Some(run) => {
                let chosen = select_variant(&run.run_id, invocation_id);
                let target_version = match chosen {
                    Variant::A => run.variant_a.as_str(),
                    Variant::B => run.variant_b.as_str(),
                };
                if let Some(idx) = variants
                    .iter()
                    .position(|s| s.frontmatter.version == target_version)
                {
                    let picked = variants.swap_remove(idx);
                    pending.push(PendingAbOutcome {
                        run_id: run.run_id,
                        invocation_id: invocation_id.to_string(),
                        variant: chosen,
                    });
                    out.push(picked);
                } else {
                    tracing::warn!(
                        skill = name.as_str(),
                        run_id = run.run_id.as_str(),
                        target_version,
                        "skill A/B select_variant chose a version not present in \
                         the active list; falling back to first variant"
                    );
                    out.push(first_by_version(variants));
                }
            }
            None => {
                // No open run for this multi-variant skill — keep all
                // rows to preserve current pre-wire behaviour.
                out.extend(variants);
            }
        }
    }
    (out, pending)
}

/// Record outcomes for each pending A/B selection. Errors are
/// logged at `warn` and SWALLOWED — the brief is explicit that this
/// is a side channel that must never fail the turn.
pub(crate) async fn record_pending_outcomes(
    store: &SessionStore,
    outcomes: &[PendingAbOutcome],
    succeeded: bool,
    latency_ms: Option<i64>,
) {
    for p in outcomes {
        if let Err(e) = record_outcome(
            store,
            &p.run_id,
            &p.invocation_id,
            p.variant,
            succeeded,
            latency_ms,
        )
        .await
        {
            tracing::warn!(
                run_id = p.run_id.as_str(),
                invocation_id = p.invocation_id.as_str(),
                "skill A/B record_outcome failed (turn outcome unchanged): {e}"
            );
        }
    }
}

struct ActiveAbRun {
    run_id: String,
    variant_a: String,
    variant_b: String,
}

/// Best-effort lookup of the most recent `status = 'running'` row in
/// `skill_ab_test_run` for `skill_name`. Returns `None` on any error
/// (e.g. table missing because no operator has yet kicked off an
/// A/B test on this workspace); the caller treats `None` as "no
/// active run".
async fn find_active_ab_run(store: &SessionStore, skill_name: &str) -> Option<ActiveAbRun> {
    let row = sqlx::query(
        "SELECT id, variant_a, variant_b FROM skill_ab_test_run \
         WHERE skill_name = ? AND status = 'running' \
         ORDER BY started_at DESC LIMIT 1",
    )
    .bind(skill_name)
    .fetch_optional(store.pool())
    .await;
    match row {
        Ok(Some(r)) => Some(ActiveAbRun {
            run_id: r.get("id"),
            variant_a: r.get("variant_a"),
            variant_b: r.get("variant_b"),
        }),
        Ok(None) => None,
        Err(e) => {
            tracing::debug!(
                skill = skill_name,
                "skill A/B run lookup failed (treating as no-active-run): {e}"
            );
            None
        }
    }
}

fn first_by_version(mut variants: Vec<Skill>) -> Skill {
    variants.sort_by(|a, b| a.frontmatter.version.cmp(&b.frontmatter.version));
    variants.swap_remove(0)
}

#[cfg(test)]
#[path = "turn_skills_runtime_tests.rs"]
mod tests;
