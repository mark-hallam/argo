//! Archetype-free companion-file writers for the materializer (DEC-025 P1).
//!
//! The archetype path's [`crate::prompt_writer`] keys the system + persona
//! prompts on `archetype.label()` — a 1-of-12 template. For a from-scratch
//! teammate there is no archetype template, so these writers source the
//! prompts from the [`CapabilitySpec`]'s persona + responsibilities +
//! capability vocabulary instead. The safety rules (plan-before-act,
//! approval-gating on sensitive verbs, evidence-citing, escalation) are
//! identical to the archetype prompt so a synthesized teammate is bound by
//! the same behaviour contract.
//!
//! The memory schema reuses [`crate::memory_schema::generate`] (which already
//! takes an `Option<&Archetype>`, so passing `None` is archetype-free) and
//! the eval seed reuses [`crate::eval_seed::generate`] (which takes actions,
//! not an archetype) — no parallel generators.

use crate::error::GenesisError;
use crate::eval_seed::{self, EvalSeedAction};
use crate::materialize_naming::truncate_description;
use crate::memory_schema;
use crate::spec::CapabilitySpec;
use argo_vertical::WORKFLOW_SCHEMA_VERSION;
use std::collections::BTreeSet;
use std::path::Path;

/// Build the system prompt for a synthesized teammate from its spec. Mirrors
/// [`crate::prompt_writer::build_system_prompt`] but the role identity comes
/// from the persona description + responsibilities, not an archetype label.
#[must_use]
pub fn system_prompt_from_spec(spec: &CapabilitySpec) -> String {
    let responsibilities = if spec.responsibilities.is_empty() {
        String::from("- (derive your responsibilities from the charter above)\n")
    } else {
        spec.responsibilities
            .iter()
            .map(|r| format!("- {}\n", r.trim()))
            .collect::<String>()
    };
    let actions = spec
        .capabilities
        .iter()
        .map(|c| format!("- `{}` — {}\n", c.action_name, c.intent.trim()))
        .collect::<String>();
    format!(
        "# System prompt — worker `{persona}`\n\n\
         You are {persona}. {description}\n\n\
         ## Charter (operator's words)\n\n{charter}\n\n\
         ## What you are responsible for\n\n{responsibilities}\n\
         ## Your action vocabulary\n\n{actions}\n\
         ## How to behave\n\n\
         - Plan before you act. Surface your plan to the operator before \
         dispatching any action that touches the outside world.\n\
         - Every action you emit must match an entry in the manifest's \
         `[actions]` vocabulary. Undeclared action_types will land \
         `blocked` rather than executing.\n\
         - Sensitive actions (send, post, pay, publish, delete, share, \
         message) ALWAYS require operator approval. Do not attempt to \
         autonomously dispatch them.\n\
         - You hold NO live integrations at birth. You can draft and propose, \
         but you physically cannot transmit until the operator grants an \
         integration.\n\
         - Cite evidence for every claim. The OS's evidence gate \
         rejects emissions that lack a verifiable source.\n\
         - When a task is outside your charter or you are uncertain, \
         escalate rather than guess.\n\n\
         ## Memory\n\n\
         You have layered memory: working, episodic, and skills. When you \
         learn something the operator might want repeated, propose a skill. \
         The operator approves or declines.\n",
        persona = spec.persona,
        description = spec.persona_description.trim(),
        charter = spec.charter,
        responsibilities = responsibilities,
        actions = actions,
    )
}

/// Build the persona prompt for a synthesized teammate. Mirrors
/// [`crate::prompt_writer::build_persona_prompt`] but the role line is the
/// persona description, not an archetype label.
#[must_use]
pub fn persona_prompt_from_spec(spec: &CapabilitySpec) -> String {
    format!(
        "# Persona — {persona}\n\n\
         **Name**: {persona}  \n\
         **Role**: {description}  \n\
         **Tone**: professional, warm, concise. Always sign messages with \
         `— {persona}`.\n\n\
         **What I'm not**: I'm not a chatbot, not a magic-8-ball, not the \
         operator. When asked something outside my charter, I escalate \
         rather than guess.\n\n\
         **Built by**: my operator, synthesized from scratch by the Skycot \
         Worker OS (Genesis) from a plain-English role description.\n",
        persona = spec.persona,
        description = spec.persona_description.trim(),
    )
}

/// Write the archetype-free companion files (`prompts/system.md`,
/// `prompts/persona.md`, `memory/schema.toml`, `eval/seed.jsonl`) for a
/// materialized teammate. The manifest + worker.toml + eval-seed.toml are
/// written by the shared `pipeline::write_vertical_with_overlap` path.
pub fn write_spec_artefacts(dir: &Path, spec: &CapabilitySpec) -> Result<(), GenesisError> {
    let prompts_dir = dir.join("prompts");
    let memory_dir = dir.join("memory");
    let eval_dir = dir.join("eval");
    for d in [&prompts_dir, &memory_dir, &eval_dir] {
        std::fs::create_dir_all(d).map_err(|e| GenesisError::Io {
            what: "materialize.dir.create",
            source: e,
        })?;
    }
    write(&prompts_dir.join("system.md"), &system_prompt_from_spec(spec))?;
    write(
        &prompts_dir.join("persona.md"),
        &persona_prompt_from_spec(spec),
    )?;
    // Archetype-free memory schema: pass `None` so the generator keys off the
    // role text alone (no archetype label).
    let schema = memory_schema::generate(&spec.charter, None)?;
    write(&memory_dir.join("schema.toml"), &memory_schema::to_toml(&schema))?;
    write(&eval_dir.join("seed.jsonl"), &spec_eval_seed_jsonl(spec))?;
    write_spec_workflows(dir, spec)?;
    Ok(())
}

/// Build the smoke-workflow files for a synthesized teammate — one minimal
/// `workflows/<duty>.workflow.toml` per declared capability ("primary duty"),
/// mirroring the hand-built
/// `verticals/receptionist/workflows/caller_greeting_check.workflow.toml`: two
/// `tool_call` steps that open the teammate's manifest + system prompt so the
/// runner is wired against this generated vertical end-to-end. Returns
/// `(relative_path, toml_body)` pairs; the relative path is exactly what the
/// manifest's `[workflows].files` list references, so the manifest builder and
/// the disk writer agree on the file set.
///
/// De-duped by `action_name` (mirroring `materialize::build_actions`, which
/// keeps the first of any sanitised collision) so a duplicate verb does not
/// emit two same-named workflows.
#[must_use]
pub fn spec_workflow_files(spec: &CapabilitySpec) -> Vec<(String, String)> {
    let mut seen: BTreeSet<String> = BTreeSet::new();
    spec.capabilities
        .iter()
        .filter(|cap| seen.insert(cap.action_name.clone()))
        .map(|cap| {
            (
                format!("workflows/{}.workflow.toml", cap.action_name),
                workflow_toml(&cap.action_name, &cap.intent),
            )
        })
        .collect()
}

/// Render one minimal `.workflow.toml`, schema-matching the receptionist
/// reference shape: `schema_version` + `[workflow]` + two ordered `tool_call`
/// `[[step]]`s. `name` is a sanitised `action_name` slug (safe to embed
/// unescaped); the free-text `intent` is truncated to the manifest cap and
/// TOML-basic-string-escaped.
fn workflow_toml(action_name: &str, intent: &str) -> String {
    let description = escape_toml_basic(&truncate_description(intent.trim()));
    format!(
        "schema_version = \"{ver}\"\n\n\
         [workflow]\n\
         name        = \"{name}\"\n\
         description = \"{description}\"\n\n\
         [[step]]\n\
         name = \"read_manifest\"\n\
         kind = \"tool_call\"\n\
         tool = \"file_read\"\n\
         args = {{ path = \"vertical.toml\" }}\n\n\
         [[step]]\n\
         name = \"read_anchor\"\n\
         kind = \"tool_call\"\n\
         tool = \"file_read\"\n\
         args = {{ path = \"prompts/system.md\" }}\n\
         depends_on = [\"read_manifest\"]\n",
        ver = WORKFLOW_SCHEMA_VERSION,
        name = action_name,
    )
}

/// Escape a string for a TOML basic (double-quoted) string literal.
fn escape_toml_basic(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04X}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// Write the generated smoke workflows under `<dir>/workflows/`. Each emitted
/// TOML is round-tripped through [`argo_vertical::parse_workflow`] BEFORE it
/// touches disk — fail-closed, so a workflow that won't validate aborts the
/// whole materialization rather than shipping an unloadable file.
fn write_spec_workflows(dir: &Path, spec: &CapabilitySpec) -> Result<(), GenesisError> {
    let files = spec_workflow_files(spec);
    if files.is_empty() {
        return Ok(());
    }
    let workflows_dir = dir.join("workflows");
    std::fs::create_dir_all(&workflows_dir).map_err(|e| GenesisError::Io {
        what: "materialize.workflows.dir.create",
        source: e,
    })?;
    for (rel, body) in &files {
        argo_vertical::parse_workflow(body).map_err(|e| GenesisError::Validation {
            stage: "materialize.workflow",
            detail: format!("emitted {rel} failed workflow validation: {e}"),
        })?;
        let file_name = rel.rsplit('/').next().unwrap_or(rel.as_str());
        write(&workflows_dir.join(file_name), body)?;
    }
    Ok(())
}

/// Render the `eval/seed.jsonl` lines for a synthesized teammate from the
/// canonical [`eval_seed::generate`] (the SAME generator the archetype path
/// uses, keyed on actions not an archetype) so the on-disk seeds match the
/// `eval-seed.toml` the hire gate grades.
fn spec_eval_seed_jsonl(spec: &CapabilitySpec) -> String {
    let actions: Vec<EvalSeedAction> = spec
        .capabilities
        .iter()
        .map(|c| EvalSeedAction {
            name: c.action_name.clone(),
            description: c.intent.clone(),
        })
        .collect();
    let seeds = eval_seed::generate(&spec.charter, &actions).unwrap_or_default();
    let mut out = String::new();
    for seed in &seeds {
        let line = serde_json::json!({
            "id": seed.id,
            "prompt": seed.task,
            "expect_summary": seed.expected_summary,
        });
        out.push_str(&line.to_string());
        out.push('\n');
    }
    out
}

fn write(path: &Path, body: &str) -> Result<(), GenesisError> {
    std::fs::write(path, body).map_err(|e| GenesisError::Io {
        what: "materialize.write",
        source: e,
    })
}

#[cfg(test)]
#[path = "materialize_writer_tests.rs"]
mod tests;
