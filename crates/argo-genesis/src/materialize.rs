//! The materializer — `CapabilitySpec → loadable vertical` (DEC-025 P1).
//!
//! This is **the single missing seam** the design doc names: today
//! [`crate::GenesisOutcome::Broad`] is a terminal value that can be judged
//! and slug-read but **never written** — there is no `write_broad_vertical`
//! anywhere in the workspace, so a genuinely novel role dead-ends. This
//! module closes that: it turns a typed [`CapabilitySpec`] (synthesized from
//! scratch for ANY role) into a [`crate::GenesisPlan`] whose `vertical.toml`
//! is a real, least-privilege, `load_dir`-valid template, and writes it +
//! its companion files to disk.
//!
//! It mirrors `pipeline::write_vertical_with_overlap` but keys every artefact
//! on the **spec**, not on an `Archetype` enum:
//!
//! 1. Derive a validator-legal `vertical_id` (run [`argo_vertical::parse`]'s
//!    id rule BEFORE writing — fail-closed rather than emit an unloadable
//!    dir).
//! 2. Wrap each [`crate::CapabilityDecl`] into an `[actions.<name>]`
//!    declaration carrying `risk` + `requires_approval` + a real `executor`
//!    (closing the `executor="human"`-everywhere gap — but per DEC-025 #6 an
//!    `Mcp` intent is recorded as a draft integration, NOT auto-bound).
//! 3. Emit `[tools]` (least-privilege RUNTIME-TOOL allow-list — safe
//!    baseline + role-conditional grants, with the shell/exec/transmit
//!    family + structural sensitive verbs denied; see
//!    [`crate::materialize_tools`]), `[mcp_servers]` (draft), and
//!    `[escalation]` — none of which Genesis writes today.
//! 4. Emit typed `[[kpi]]` from `success_metrics` (falling back to the SLA
//!    skeleton so `role.kpis` is never empty).
//! 5. Companion files via **archetype-free** writers
//!    ([`crate::materialize_writer`]): system / persona prompts from the
//!    persona + responsibilities, a spec-derived memory schema, the canonical
//!    `eval/seed.jsonl` + `eval-seed.toml`, and the `WorkerConfig` 24h
//!    dry-run.
//! 6. Run [`crate::safety_judge::auto_fix`] + [`crate::safety_judge::judge`]
//!    over the wrapped actions — NON-NEGOTIABLE: the spec path is the one
//!    place sensitive verbs arrive un-classified.
//! 7. Round-trip through [`argo_vertical::load_dir`] and assert zero
//!    `LoadError`s — a spec that can't produce a loadable vertical fails
//!    closed.

use crate::error::GenesisError;
use crate::safety_judge;
use crate::spec::{CapabilitySpec, ExecutorIntent};
use crate::materialize_naming::{
    derive_display_name, derive_vertical_id, truncate_description, validate_vertical_id,
};
use crate::{memory_schema, pipeline::GenesisPlan};
use argo_vertical::{
    ActionDeclaration, ActionsSection, EscalationSection, ExecutorDeclaration, KpiComparator,
    KpiDefinition, KpiGate, KpiKind, McpServerDeclaration, McpServersSection, RoleSection,
    SuccessMetricsSection, VerticalIdentity, VerticalManifest, WorkflowsSection,
    SCHEMA_VERSION,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// Build a [`GenesisPlan`] from a [`CapabilitySpec`] — the archetype-free
/// materialization path. Produces the same `GenesisPlan` shape the archetype
/// path produces (so the entire downstream pipeline — disk write, `load_dir`
/// round-trip, hire gate, capability fence — consumes it unchanged), but
/// every field is derived from the spec, not from an archetype template.
///
/// The plan's `archetype` is the spec's nearest-archetype prior (DEC-025 #3
/// hybrid) so the SSE payload + audit carry a stable slug; the plan also
/// carries the spec itself so the disk writer can use the archetype-free
/// companion-file writers.
///
/// # Errors
///
/// - [`GenesisError::InvalidPersona`] if the persona is empty / illegal.
/// - [`GenesisError::Validation`] if the spec declares no capabilities, or if
///   the derived `vertical_id` is not validator-legal, or if the safety judge
///   rejects the wrapped actions.
pub fn materialize_spec(spec: &CapabilitySpec) -> Result<GenesisPlan, GenesisError> {
    crate::pipeline::validate_persona(&spec.persona)?;
    if spec.capabilities.is_empty() {
        return Err(GenesisError::Validation {
            stage: "materialize.capabilities",
            detail: "spec declares no capabilities; a teammate needs at least one action".into(),
        });
    }

    let vertical_id = derive_vertical_id(spec);
    // Fail-closed: validate the id against the SAME rule the loader enforces
    // BEFORE we build/write anything unloadable.
    validate_vertical_id(&vertical_id)?;

    let manifest = build_spec_manifest(spec, &vertical_id)?;

    Ok(GenesisPlan {
        charter: spec.charter.clone(),
        persona: spec.persona.clone(),
        archetype: spec.nearest_archetype,
        vertical_id,
        manifest,
        spec: Some(spec.clone()),
    })
}

/// Materialize a [`CapabilitySpec`] AND write it to `output`, mirroring
/// `pipeline::write_vertical`. Refuses to clobber a non-empty dir; round-trip
/// validates via [`argo_vertical::load_dir`].
///
/// # Errors
///
/// Propagates [`materialize_spec`] errors plus any IO / load-round-trip
/// failure.
pub fn write_capability_vertical(
    spec: &CapabilitySpec,
    output: &Path,
) -> Result<GenesisPlan, GenesisError> {
    let plan = materialize_spec(spec)?;
    crate::pipeline::write_vertical(&plan, output)?;
    Ok(plan)
}

/// Build the full `VerticalManifest` from a spec. Runs `auto_fix` + `judge`
/// over the wrapped actions before returning (the spec path is the one place
/// sensitive verbs arrive un-classified).
fn build_spec_manifest(
    spec: &CapabilitySpec,
    vertical_id: &str,
) -> Result<VerticalManifest, GenesisError> {
    let mut actions = build_actions(spec);
    // NON-NEGOTIABLE safety pass: promote every sensitive verb to
    // requires_approval, then hard-fail if any slipped through.
    safety_judge::auto_fix(&mut actions);
    safety_judge::judge(&actions)?;

    let manifest = VerticalManifest {
        schema_version: SCHEMA_VERSION.to_string(),
        vertical: VerticalIdentity {
            id: vertical_id.to_string(),
            name: derive_display_name(spec),
            version: "0.0.1".to_string(),
            description: format!("Generated by Genesis from charter: {}", spec.charter),
            tags: vec!["genesis".to_string(), "from-scratch".to_string()],
            ..VerticalIdentity::default()
        },
        role: Some(RoleSection {
            system_prompt: "prompts/system.md".to_string(),
            persona: Some("prompts/persona.md".to_string()),
        }),
        // Runtime-tool allow/deny list (the capability-fence input) — REAL
        // in-core tool names conservatively derived from the role, NEVER the
        // action vocabulary (see `materialize_tools` for the bug this fixed).
        tools: Some(crate::materialize_tools::build_tools(spec, &actions)),
        memory: Some(memory_schema::manifest_section()),
        escalation: Some(build_escalation(spec)),
        kpis: build_kpis(spec),
        // SLA skeleton is a fallback floor ONLY — kept when no typed criteria
        // were captured; when they were, the `[[kpi]]` rows ARE the scored goals.
        success_metrics: spec
            .success_criteria
            .is_empty()
            .then(default_success_metrics),
        actions: Some(actions),
        mcp_servers: build_mcp_servers(spec),
        // Declare one smoke workflow per primary duty so a hired teammate ships
        // with a runnable `[workflows]` block (the files are written by
        // `materialize_writer::write_spec_workflows`).
        workflows: build_workflows_section(spec),
        ..VerticalManifest::default()
    };
    Ok(manifest)
}

/// Wrap every [`crate::CapabilityDecl`] into an `[actions.<name>]` entry. The
/// executor is real (from [`ExecutorIntent`]) — but per DEC-025 #6 an `Mcp`
/// intent does NOT auto-bind a live server: the action still runs `human` and
/// the server name is surfaced only in `[mcp_servers]` (recorded as draft).
pub(crate) fn build_actions(spec: &CapabilitySpec) -> ActionsSection {
    let mut actions: BTreeMap<String, ActionDeclaration> = BTreeMap::new();
    for cap in &spec.capabilities {
        // De-dupe by action name (a sanitised collision keeps the first).
        if actions.contains_key(&cap.action_name) {
            continue;
        }
        actions.insert(
            cap.action_name.clone(),
            ActionDeclaration {
                risk: cap.risk,
                requires_approval: cap.requires_approval,
                description: cap.intent.clone(),
                executor: executor_declaration(&cap.proposed_executor),
                ..ActionDeclaration::default()
            },
        );
    }
    ActionsSection { actions }
}

/// Map an [`ExecutorIntent`] to an [`ExecutorDeclaration`]. `Mcp` and
/// `Builtin` both materialize as `human` at birth (draft-only posture, locked
/// #6); the MCP server name rides separately into `[mcp_servers]`. Keeping
/// the executor `human` means a day-one teammate can draft/propose but
/// physically cannot transmit.
fn executor_declaration(intent: &ExecutorIntent) -> ExecutorDeclaration {
    match intent {
        // Builtin = a side-effect-free helper; still human-dispatched at
        // birth so nothing transmits without an operator in the loop.
        ExecutorIntent::Human | ExecutorIntent::Builtin | ExecutorIntent::Mcp { .. } => {
            ExecutorDeclaration::Legacy("human".to_string())
        }
    }
}

/// Build `[mcp_servers]` from the spec's `Mcp` executor intents. Each named
/// server is declared as a stdio transport placeholder — recorded so the
/// capability fence surfaces it as a *draft integration* (NOT live; locked
/// #6). Returns `None` when no capability references an MCP server.
fn build_mcp_servers(spec: &CapabilitySpec) -> Option<McpServersSection> {
    let mut servers: BTreeMap<String, McpServerDeclaration> = BTreeMap::new();
    for cap in &spec.capabilities {
        if let ExecutorIntent::Mcp { server, .. } = &cap.proposed_executor {
            let key = crate::spec::sanitize_action_name(server);
            servers.entry(key).or_insert_with(|| McpServerDeclaration {
                transport: "stdio".to_string(),
                // Placeholder command: the integration is draft-only at birth.
                // A deliberate post-hire operator grant wires the real one.
                command: "true".to_string(),
                args: Vec::new(),
            });
        }
    }
    (!servers.is_empty()).then_some(McpServersSection { servers })
}

/// Build the `[workflows]` section: list the smoke-workflow files emitted by
/// [`crate::materialize_writer::spec_workflow_files`] (one per primary duty),
/// so the loader knows about them. Mirrors the receptionist manifest's
/// `files = ["workflows/<name>.workflow.toml"]`. Returns `None` only for a
/// capability-less spec — which [`materialize_spec`] already rejects upstream —
/// so a materialized teammate always ships at least one declared workflow.
fn build_workflows_section(spec: &CapabilitySpec) -> Option<WorkflowsSection> {
    let files: Vec<String> = crate::materialize_writer::spec_workflow_files(spec)
        .into_iter()
        .map(|(rel, _body)| rel)
        .collect();
    (!files.is_empty()).then_some(WorkflowsSection { files })
}

/// Build the `[escalation]` section from the spec's typed escalation routing.
fn build_escalation(spec: &CapabilitySpec) -> EscalationSection {
    let mut gated: Vec<String> = spec.escalation.human_approval_required.clone();
    // Belt-and-braces: ensure every approval-gated action is also listed in
    // the escalation set even if synthesis omitted it.
    for cap in &spec.capabilities {
        if cap.requires_approval && !gated.contains(&cap.action_name) {
            gated.push(cap.action_name.clone());
        }
    }
    gated.sort();
    gated.dedup();
    EscalationSection {
        human_approval_required: gated,
        escalate_after_failures: spec.escalation.escalate_after_failures,
        escalate_to: spec.escalation.escalate_to.clone(),
    }
}

/// Build the typed `[[kpi]]` rows the dashboards score against, in priority
/// order so a teammate is scored against its ACTUAL goals (UF-A2):
/// (1) the operator's typed [`crate::spec::SuccessCriterion`] set wins — each
/// becomes a real `gate = Warn` KPI carrying the chosen `kind`/`target`/
/// `comparator`, so DIFFERENT criteria emit DIFFERENT thresholds; (2) else the
/// free-text `success_metrics` become `Count`/`>= 1` `Inform` hints; (3) else
/// empty, so `all_kpis` falls back to the `[success_metrics]` SLA skeleton.
fn build_kpis(spec: &CapabilitySpec) -> Vec<KpiDefinition> {
    if !spec.success_criteria.is_empty() {
        let mut seen: BTreeSet<String> = BTreeSet::new();
        return spec
            .success_criteria
            .iter()
            .enumerate()
            .map(|(idx, criterion)| {
                // KPI names must be unique within a manifest (validation rejects
                // dups). Sanitize the operator label into a slug, suffixing the
                // 1-based index on any collision rather than failing the hire.
                let mut name = crate::spec::sanitize_action_name(&criterion.name);
                if !seen.insert(name.clone()) {
                    name = format!("{name}_{}", idx + 1);
                    seen.insert(name.clone());
                }
                KpiDefinition {
                    name,
                    kind: criterion.kind,
                    threshold: criterion.target,
                    comparator: criterion.comparator,
                    gate: KpiGate::Warn,
                    description: Some(truncate_description(criterion.name.trim())),
                }
            })
            .collect();
    }
    spec.success_metrics
        .iter()
        .enumerate()
        .filter_map(|(idx, metric)| {
            let trimmed = metric.trim();
            (!trimmed.is_empty()).then(|| KpiDefinition {
                name: format!("success_metric_{}", idx + 1),
                kind: KpiKind::Count,
                threshold: 1.0,
                comparator: KpiComparator::Gte,
                gate: KpiGate::Inform,
                description: Some(truncate_description(trimmed)),
            })
        })
        .collect()
}

/// The standard SLA/CSAT skeleton (release-gated), identical to the archetype
/// path's `build_success_metrics`. Operators tune these once the teammate has
/// real data.
fn default_success_metrics() -> SuccessMetricsSection {
    SuccessMetricsSection {
        deflection_target_pct: Some(70),
        median_response_seconds: Some(60),
        csat_threshold: Some(0.85),
        required_audit_chain_intact: Some(true),
    }
}

#[cfg(test)]
#[path = "materialize_tests.rs"]
mod tests;
