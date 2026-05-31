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
//! 3. Emit `[tools]` (least-privilege allow = the action vocabulary; deny =
//!    the structural sensitive verbs not in the allow-set), `[mcp_servers]`
//!    (draft), and `[escalation]` — none of which Genesis writes today.
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
use crate::{memory_schema, pipeline::GenesisPlan};
use argo_vertical::{
    ActionDeclaration, ActionsSection, EscalationSection, ExecutorDeclaration, KpiComparator,
    KpiDefinition, KpiGate, KpiKind, McpServerDeclaration, McpServersSection, RoleSection,
    SuccessMetricsSection, ToolsSection, VerticalIdentity, VerticalManifest, SCHEMA_VERSION,
};
use std::collections::BTreeMap;
use std::path::Path;

/// The slug stamped on the synthesized vertical's id prefix, distinguishing a
/// from-scratch teammate from the 12 archetype ids (`marketing-intern-…`
/// etc.). A novel role → `genesis-<persona>`.
const FROM_SCRATCH_ID_PREFIX: &str = "genesis";

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
        tools: Some(build_tools(&actions)),
        memory: Some(memory_schema::manifest_section()),
        escalation: Some(build_escalation(spec)),
        kpis: build_kpis(spec),
        success_metrics: Some(default_success_metrics()),
        actions: Some(actions),
        mcp_servers: build_mcp_servers(spec),
        ..VerticalManifest::default()
    };
    Ok(manifest)
}

/// Wrap every [`crate::CapabilityDecl`] into an `[actions.<name>]` entry. The
/// executor is real (from [`ExecutorIntent`]) — but per DEC-025 #6 an `Mcp`
/// intent does NOT auto-bind a live server: the action still runs `human` and
/// the server name is surfaced only in `[mcp_servers]` (recorded as draft).
fn build_actions(spec: &CapabilitySpec) -> ActionsSection {
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

/// Build the least-privilege `[tools]` section.
///
/// - `allowed`: the teammate's own action vocabulary (the deny-by-default
///   fence's allow-set is derived from these + this list at hire).
/// - `denied`: every structural sensitive verb that is NOT a declared action
///   — an explicit belt-and-braces fence so even a hallucinated tool name
///   carrying `send`/`pay`/`delete`/… is refused.
///
/// A spec with no capabilities would yield `allowed=[]` (deny-all) — but
/// [`materialize_spec`] rejects an empty spec upstream, so the materialized
/// teammate always has a real, non-empty least-privilege profile.
fn build_tools(actions: &ActionsSection) -> ToolsSection {
    let allowed: Vec<String> = actions.actions.keys().cloned().collect();
    let denied: Vec<String> = safety_judge::SENSITIVE_VERBS
        .iter()
        .map(|verb| (*verb).to_string())
        .filter(|verb| !allowed.iter().any(|a| a == verb))
        .collect();
    ToolsSection { allowed, denied }
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

/// Build typed `[[kpi]]` entries from the spec's free-text success metrics.
/// Each metric becomes an `Inform`-gated `Count` KPI with a `>= 1` threshold
/// (a non-blocking telemetry hint the operator tunes later); the legacy
/// SLA skeleton in `[success_metrics]` still provides the release-gating
/// floor via `all_kpis`. Returns an empty vec when the spec lists no metrics
/// (the legacy block then supplies the KPIs).
fn build_kpis(spec: &CapabilitySpec) -> Vec<KpiDefinition> {
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

/// Derive the teammate's **human display name** for `vertical.name` — the
/// string the SSE `vertical_ready` payload surfaces as `display_name`, which
/// the hire-commit path persists as `persona.name`. This must read like a
/// person's name ("Maya"), NOT the generic `"Genesis Teammate (<persona>)"`
/// placeholder this function used to emit (the demo-blocker: a Genesis hire
/// from a charter naming "Maya" produced a worker called
/// "Genesis Teammate (sales)").
///
/// Resolution order (first hit wins):
/// 1. An explicit name written into the charter (`named Maya`, `call her
///    Maya`, `their name is Maya`) — the operator said it out loud, honour it.
/// 2. The persona slug humanized into a name (`maya` → "Maya", `sam-j` →
///    "Sam J", `maria_garcia` → "Maria Garcia") — UNLESS the slug is a
///    generic synthesis fallback (`worker` / `operator` / `pioneer`), which is
///    not a real name and would read as placeholder text on the roster card.
/// 3. The nearest-archetype role label (`"Marketing Intern"`, `"Sales SDR"`)
///    — a real, human-meaningful role name. Never the bare "Genesis Teammate"
///    placeholder.
fn derive_display_name(spec: &CapabilitySpec) -> String {
    if let Some(named) = name_from_charter(&spec.charter) {
        return named;
    }
    if let Some(from_slug) = humanize_persona_slug(&spec.persona) {
        return from_slug;
    }
    spec.nearest_archetype.label().to_string()
}

/// Slugs `argo_genesis` substitutes when synthesis has no real persona name to
/// work with (see `charter_dialogue::name_to_persona_slug` →`"worker"`,
/// `routes_genesis_stream::derive_persona` → `"operator"`,
/// `routes_onboarding_genesis::persona_slug` → `"pioneer"`). These are NOT
/// names, so humanizing them would surface placeholder text on the roster.
const GENERIC_PERSONA_SLUGS: &[&str] = &["worker", "operator", "pioneer"];

/// Pull an explicit person name out of a free-text charter. Matches the small
/// set of phrasings the Genesis charter renderer and operators actually use
/// (`charter_dialogue::dialogue_to_charter` writes `", named <Name>"`). Returns
/// the Title-Cased name (1-3 tokens) or `None` when no naming phrase is found.
fn name_from_charter(charter: &str) -> Option<String> {
    const TRIGGERS: &[&str] = &[
        "named ",
        "name is ",
        "name's ",
        "call her ",
        "call him ",
        "call them ",
        "called ",
        "goes by ",
    ];
    let lower = charter.to_lowercase();
    for trigger in TRIGGERS {
        let mut from = 0;
        while let Some(rel) = lower[from..].find(trigger) {
            let start = from + rel + trigger.len();
            if let Some(name) = take_name_tokens(&charter[start..]) {
                return Some(name);
            }
            from = start;
        }
    }
    None
}

/// Read the leading name out of `rest` (the text right after a naming
/// trigger). The first alphabetic token is the given name; subsequent tokens
/// are appended ONLY while they are capitalized proper nouns in the source
/// (e.g. "Maria Garcia", "Sam J") — so trailing clauses like "Maya who
/// drafts…" or "Sasha and who books…" stop at the first lower-case word.
/// Each kept token is Title-Cased (the operator may have typed "maya"). Caps
/// at three tokens. Returns `None` if the first token isn't name-like.
fn take_name_tokens(rest: &str) -> Option<String> {
    let mut names: Vec<String> = Vec::new();
    for raw in rest.split_whitespace() {
        let token = raw.trim_end_matches([',', '.', ';', ':', '!', '?', ')', '(', '"', '\'']);
        let is_name_like = !token.is_empty()
            && token.chars().all(|c| c.is_ascii_alphabetic())
            && token.len() <= 20;
        if !is_name_like {
            break;
        }
        // Only the first token is taken unconditionally; continuation tokens
        // must be capitalized in the source to count as part of the name.
        let capitalized = token.chars().next().is_some_and(|c| c.is_ascii_uppercase());
        if !names.is_empty() && !capitalized {
            break;
        }
        names.push(title_case_token(token));
        if names.len() == 3 {
            break;
        }
    }
    (!names.is_empty()).then(|| names.join(" "))
}

/// Humanize a persona slug into a display name: dashes/underscores become
/// spaces, each token Title-Cased (`sam-j` → "Sam J", `maria_garcia` → "Maria
/// Garcia"). Returns `None` for an empty slug or a generic synthesis-fallback
/// slug (those are not real names — the caller falls back to the role label).
fn humanize_persona_slug(persona: &str) -> Option<String> {
    let trimmed = persona.trim();
    if trimmed.is_empty() || GENERIC_PERSONA_SLUGS.contains(&trimmed.to_lowercase().as_str()) {
        return None;
    }
    let humanized = trimmed
        .split(['-', '_'])
        .filter(|t| !t.is_empty())
        .map(title_case_token)
        .collect::<Vec<_>>()
        .join(" ");
    (!humanized.is_empty()).then_some(humanized)
}

/// Title-case a single token: upper-case the first ASCII letter, lower-case the
/// rest. Leaves a token with no leading letter untouched.
fn title_case_token(token: &str) -> String {
    let mut chars = token.chars();
    match chars.next() {
        Some(first) => {
            first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase()
        }
        None => String::new(),
    }
}

/// Truncate a KPI description to the 200-char manifest validation cap.
fn truncate_description(raw: &str) -> String {
    const MAX: usize = 200;
    if raw.chars().count() <= MAX {
        return raw.to_string();
    }
    raw.chars().take(MAX - 1).collect::<String>() + "…"
}

/// Derive a validator-legal vertical id: `genesis-<persona>` for a
/// from-scratch teammate, `<nearest-archetype-slug>-<persona>` for an
/// archetype-prior teammate (matching the archetype path's id shape so a
/// re-classified role stays stable). Lowercased, underscores → dashes.
fn derive_vertical_id(spec: &CapabilitySpec) -> String {
    let persona = spec.persona.to_lowercase().replace('_', "-");
    if spec.from_scratch {
        format!("{FROM_SCRATCH_ID_PREFIX}-{persona}")
    } else {
        format!(
            "{}-{}",
            spec.nearest_archetype.slug().replace('_', "-"),
            persona
        )
    }
}

/// Validate a derived vertical id against the SAME rule
/// [`argo_vertical`]'s loader enforces (lowercase ASCII / digits / `-`,
/// no leading/trailing dash). Fail-closed BEFORE any disk write.
fn validate_vertical_id(id: &str) -> Result<(), GenesisError> {
    let legal = !id.is_empty()
        && !id.starts_with('-')
        && !id.ends_with('-')
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if legal {
        Ok(())
    } else {
        Err(GenesisError::Validation {
            stage: "materialize.vertical_id",
            detail: format!("derived vertical_id '{id}' is not loader-legal"),
        })
    }
}

#[cfg(test)]
#[path = "materialize_tests.rs"]
mod tests;
