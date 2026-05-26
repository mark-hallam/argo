//! Genesis-invocation helpers for `routes_onboarding::first_hire_response`.
//!
//! The pre-Genesis path templated the operator's prompt into a 35-char
//! preamble and dropped a single hardcoded `daily_check_in` duty on the
//! worker. The investor-demo claim that distinguishes SKYCOT from
//! "ChatGPT with prompts" — *"Skycot turns that sentence into a real
//! vertical — system prompt, action vocabulary, memory schema, eval
//! seeds"* — was unrecordable against that code path.
//!
//! Now: `invoke_genesis` calls the synchronous, keyword-only
//! [`argo_genesis::plan`] entry point at hire time; the returned
//! [`GenesisPlan`] feeds [`synthesise_charter`] and [`derive_duties`].
//! Failure (empty charter, unknown archetype, safety judge) falls
//! through to a deterministic non-Genesis path so the wizard never
//! hard-fails mid-flow.
//!
//! Lives in its own sibling so `routes_onboarding.rs` stays under the
//! 500-LOC per-file budget — same pattern as `routes_onboarding_helpers.rs`.

use argo_genesis::{plan as genesis_plan, Archetype, GenesisError, GenesisPlan};
use argo_identity::HireDutyDraft;
use argo_vertical::{ActionDeclaration, ActionRiskLevel, LoadedVertical};
use sha2::{Digest, Sha256};

/// Maximum number of duties seeded from the picked / synthesised
/// vertical's autonomous-safe actions. Cap exists so a vertical that
/// declares ten low-risk actions doesn't bury the operator with a
/// ten-row schedule on the worker-detail page — the surplus stays
/// declared in the vertical manifest and the operator can promote
/// any of them to a duty later.
pub(super) const MAX_DERIVED_DUTIES: usize = 5;

/// Pre-staggered cron expressions for the derived duties. Each entry
/// is `"0 H * * 1-5"` — weekday-only, starting at 9am and staggered by
/// hour. The operator can edit any of these on the worker-detail
/// Schedules tab once the hire lands.
const STAGGERED_CRONS: &[&str] = &[
    "0 9 * * 1-5",
    "0 10 * * 1-5",
    "0 14 * * 1-5",
    "0 15 * * 1-5",
    "0 16 * * 1-5",
];

/// Outcome of an `invoke_genesis` call. Either Genesis ran cleanly
/// against the operator's brief and we carry a [`GenesisPlan`], or the
/// pipeline rejected the brief and we carry the failure reason so the
/// audit row + downstream fallback can reflect it.
pub(super) struct GenesisHireOutcome {
    pub(super) plan: Option<GenesisPlan>,
    pub(super) fallback_reason: Option<String>,
}

impl GenesisHireOutcome {
    pub(super) fn is_fallback(&self) -> bool {
        self.plan.is_none()
    }

    pub(super) fn archetype(&self) -> Option<Archetype> {
        self.plan.as_ref().map(|p| p.archetype)
    }

    pub(super) fn plan_id(&self) -> Option<&str> {
        self.plan.as_ref().map(|p| p.vertical_id.as_str())
    }
}

/// Run [`argo_genesis::plan`] against the operator's hire brief. The
/// pipeline is synchronous + keyword-only; the LLM-classifier branch
/// is reserved for the CLI `argo vertical genesis` surface and stays
/// out of the hire endpoint to keep latency predictable.
///
/// Empty charter, unknown archetype, or safety-judge rejection all
/// fall through to the fallback branch — the wizard never hard-fails.
pub(super) fn invoke_genesis(description: &str, worker_name: &str) -> GenesisHireOutcome {
    let persona = persona_slug(worker_name);
    match genesis_plan(description, &persona) {
        Ok(plan) => GenesisHireOutcome {
            plan: Some(plan),
            fallback_reason: None,
        },
        Err(e) => {
            tracing::warn!(
                target: "gateway::onboarding",
                error = %e,
                worker_name = %worker_name,
                "genesis hire pipeline failed; falling back to templated charter",
            );
            GenesisHireOutcome {
                plan: None,
                fallback_reason: Some(genesis_error_reason(&e)),
            }
        }
    }
}

/// Convert the parsed worker name to the `[a-zA-Z0-9_-]` slug that
/// [`argo_genesis::plan`] validates against. Strips anything outside
/// the allowed character class so a parsed first name like `"Mariah"`
/// or even a multi-word `"GTM Engineer"` always lands valid.
fn persona_slug(worker_name: &str) -> String {
    let cleaned: String = worker_name
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .collect();
    if cleaned.is_empty() {
        "pioneer".into()
    } else {
        cleaned.to_lowercase()
    }
}

fn genesis_error_reason(err: &GenesisError) -> String {
    match err {
        GenesisError::EmptyCharter => "empty_charter".into(),
        GenesisError::InvalidPersona(_) => "invalid_persona".into(),
        GenesisError::UnknownArchetype { .. } => "unknown_archetype".into(),
        GenesisError::SafetyViolation { .. } => "safety_violation".into(),
        GenesisError::LoadFailed(_) => "load_failed".into(),
        GenesisError::OutputDirNotEmpty(_) => "output_dir_not_empty".into(),
        GenesisError::Io { .. } => "io_failure".into(),
        GenesisError::Json { .. } => "json_failure".into(),
        GenesisError::Toml { .. } => "toml_failure".into(),
        GenesisError::Validation { .. } => "validation_failure".into(),
    }
}

/// Build the charter the worker is persisted with. On the Genesis
/// success path, the charter opens with the archetype label so the
/// `/audit` drill-in narrates "Maya is a Marketing Intern" and the
/// operator's own brief follows verbatim. On fallback, we keep the
/// pre-Genesis templated string so existing UX assumptions (charter
/// surfaces the operator's intent) don't regress.
pub(super) fn synthesise_charter(description: &str, plan: Option<&GenesisPlan>) -> String {
    let trimmed = description.trim();
    match plan {
        Some(plan) => format!(
            "{label} — synthesised by Genesis from your hire brief.\n\n\
             Operator brief:\n{trimmed}\n\n\
             Genesis plan id: `{plan_id}`. The synthesised vertical \
             ships an action vocabulary, memory schema, and eval seeds \
             tuned to this archetype.",
            label = plan.archetype.label(),
            plan_id = plan.vertical_id,
        ),
        None => format!("First hire for this workspace. Operator brief:\n\n{trimmed}"),
    }
}

/// Derive scheduled duties for the new worker from the picked
/// vertical's autonomous-safe actions, preferring the Genesis-
/// synthesised manifest when one is present. Falls back to the
/// pre-Genesis hardcoded `daily_check_in` when neither manifest
/// surfaces a usable action — that keeps the `HireDraft` validator's
/// "at least one enabled duty" invariant intact.
pub(super) fn derive_duties(
    plan: Option<&GenesisPlan>,
    picked: &LoadedVertical,
) -> Vec<HireDutyDraft> {
    let mut duties: Vec<HireDutyDraft> = autonomous_actions_from_plan(plan)
        .into_iter()
        .chain(autonomous_actions_from_vertical(picked))
        .collect();
    dedupe_by_entry_action(&mut duties);
    duties.truncate(MAX_DERIVED_DUTIES);
    if duties.is_empty() {
        duties.push(default_check_in_duty());
    }
    duties
}

fn autonomous_actions_from_plan(plan: Option<&GenesisPlan>) -> Vec<HireDutyDraft> {
    let Some(plan) = plan else {
        return Vec::new();
    };
    let Some(actions) = plan.manifest.actions.as_ref() else {
        return Vec::new();
    };
    autonomous_actions(actions.actions.iter())
}

fn autonomous_actions_from_vertical(picked: &LoadedVertical) -> Vec<HireDutyDraft> {
    let Some(actions) = picked.manifest.actions.as_ref() else {
        return Vec::new();
    };
    autonomous_actions(actions.actions.iter())
}

fn autonomous_actions<'a>(
    iter: impl Iterator<Item = (&'a String, &'a ActionDeclaration)>,
) -> Vec<HireDutyDraft> {
    iter.filter(|(_, decl)| {
        matches!(decl.risk, ActionRiskLevel::Low) && !decl.requires_approval
    })
    .enumerate()
    .map(|(idx, (name, decl))| HireDutyDraft {
        cron: STAGGERED_CRONS
            .get(idx)
            .copied()
            .unwrap_or("0 9 * * 1-5")
            .to_string(),
        entry_action: name.clone(),
        description: if decl.description.trim().is_empty() {
            format!("Autonomous {name} duty.")
        } else {
            decl.description.clone()
        },
        // Every autonomous-safe duty lands enabled so the worker has a
        // populated schedule from day one; the operator can pause any
        // individual duty on the Schedules tab. The identity layer's
        // `assemble_worker_from_draft` filters by `default_enabled`
        // when persisting `duty` rows, so a `false` here silently drops
        // the duty.
        default_enabled: true,
    })
    .collect()
}

fn dedupe_by_entry_action(duties: &mut Vec<HireDutyDraft>) {
    let mut seen = std::collections::BTreeSet::new();
    duties.retain(|d| seen.insert(d.entry_action.clone()));
}

fn default_check_in_duty() -> HireDutyDraft {
    HireDutyDraft {
        cron: "0 9 * * 1-5".into(),
        entry_action: "daily_check_in".into(),
        description: "Daily check-in. Vertical declared no autonomous-safe actions; \
                      operator can replace this duty on the Schedules tab."
            .into(),
        default_enabled: true,
    }
}

/// SHA-256 of the persisted charter, hex-encoded. Surfaced into the
/// audit payload so the `/audit` drill-in can prove the worker's
/// charter is the exact synthesised body (a future regression that
/// reverts to the templated preamble shows up as a charter_hash
/// mismatch).
pub(super) fn charter_hash(charter: &str) -> String {
    let digest = Sha256::digest(charter.as_bytes());
    hex::encode(digest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persona_slug_strips_non_identifier_chars_and_lowercases() {
        assert_eq!(persona_slug("Mariah"), "mariah");
        assert_eq!(persona_slug("GTM Engineer"), "gtmengineer");
        assert_eq!(persona_slug("name+with@symbols"), "namewithsymbols");
        assert_eq!(persona_slug(""), "pioneer");
        assert_eq!(persona_slug("!!!"), "pioneer");
    }

    #[test]
    fn synthesise_charter_includes_archetype_label_on_success() {
        let plan = genesis_plan(
            "be my marketing intern, run weekly campaigns to cold leads",
            "maya",
        )
        .expect("plan builds");
        let body = synthesise_charter("be my marketing intern", Some(&plan));
        assert!(body.starts_with("Marketing Intern"), "got: {body}");
        assert!(
            body.contains("Genesis plan id"),
            "synthesised charter should name the plan id: {body}",
        );
    }

    #[test]
    fn synthesise_charter_falls_back_to_templated_preamble() {
        let body = synthesise_charter("Mariah, you book outbound demo calls", None);
        assert!(
            body.starts_with("First hire for this workspace"),
            "fallback must preserve pre-Genesis preamble: {body}",
        );
        assert!(body.contains("Mariah, you book outbound demo calls"));
    }

    #[test]
    fn charter_hash_is_deterministic_and_distinguishes_bodies() {
        let a = charter_hash("body-a");
        let b = charter_hash("body-a");
        let c = charter_hash("body-b");
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_eq!(a.len(), 64, "sha256 hex is 64 chars");
    }

    #[test]
    fn invoke_genesis_returns_fallback_for_unparseable_charter() {
        let out = invoke_genesis("xyzzy plugh nonsense filler text", "spock");
        assert!(out.is_fallback(), "no archetype keyword should fall back");
        assert_eq!(out.fallback_reason.as_deref(), Some("unknown_archetype"));
    }
}
