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
use crate::memory_schema;
use crate::spec::CapabilitySpec;
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
