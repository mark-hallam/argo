//! `argo genesis interactive` — Q&A walkthrough that builds a
//! charter conversationally and feeds it through the existing
//! Genesis pipeline (Stream `claude-genesis-charter-conversation`).
//!
//! The Q&A loop itself lives in [`super::genesis_interactive_qa`]
//! so the dialogue logic can be unit-tested without spawning a
//! subprocess; this file is the cli wrapper that picks the input
//! source (stdin vs `--non-interactive` YAML), persists the
//! generated vertical, and prints the post-create report.
use crate::handlers::genesis_interactive_qa::{prompt_dialogue, RealReader};
use argo_genesis::{dialogue_to_charter, name_to_persona_slug, CharterDialogue};
use argo_types::{ArgoError, ArgoResult};
use serde_json::json;
use std::path::{Path, PathBuf};

/// Top-level `argo genesis interactive` entry point. Picks the
/// input source, validates the dialogue, renders the charter,
/// then defers the deterministic + LLM-refinement pipeline to
/// the existing [`super::genesis::create`] handler so the
/// post-create UX is identical between the two surfaces.
pub(crate) fn run(
    non_interactive: Option<PathBuf>,
    output: &Path,
    json: bool,
    refine: bool,
) -> ArgoResult<()> {
    let dialogue = match non_interactive.as_ref() {
        Some(path) => load_dialogue_yaml(path)?,
        None => {
            let stdin = std::io::stdin();
            let mut reader = RealReader::new(stdin.lock());
            let mut writer = std::io::stdout();
            prompt_dialogue(&mut reader, &mut writer)?
        }
    };
    dialogue
        .validate()
        .map_err(|e| ArgoError::Provider(format!("genesis interactive: {e}")))?;
    let charter = dialogue_to_charter(&dialogue);
    let persona = name_to_persona_slug(&dialogue.name);
    if json {
        emit_dialogue_json(&dialogue, &charter, &persona, output);
    } else {
        emit_dialogue_summary(&dialogue, &persona);
    }
    super::genesis::create(&charter, &persona, output, json, refine, false)
}

/// Read a `CharterDialogue` from a YAML file. Used by the
/// `--non-interactive` path so CI / scripted setup can drive the
/// walkthrough without TTY input.
pub(crate) fn load_dialogue_yaml(path: &Path) -> ArgoResult<CharterDialogue> {
    let body = std::fs::read_to_string(path).map_err(|e| {
        ArgoError::Provider(format!(
            "genesis interactive: read {}: {e}",
            path.display()
        ))
    })?;
    serde_yaml::from_str(&body).map_err(|e| {
        ArgoError::Provider(format!(
            "genesis interactive: parse {}: {e}",
            path.display()
        ))
    })
}

fn emit_dialogue_summary(d: &CharterDialogue, persona_slug: &str) {
    println!("Captured charter dialogue:");
    println!("  role:       {}", d.role);
    println!("  name:       {} (persona slug: {persona_slug})", d.name);
    if !d.industry.trim().is_empty() {
        println!("  industry:   {}", d.industry);
    }
    println!("  duties:");
    for (i, duty) in d
        .duties
        .iter()
        .filter(|x| !x.trim().is_empty())
        .enumerate()
    {
        println!("    {}. {}", i + 1, duty.trim());
    }
    println!("  channels:   {}", d.channels.join(", "));
    println!("  spend cap:  ${:.2}/day", d.spend_cap_per_day_usd);
    println!();
}

fn emit_dialogue_json(d: &CharterDialogue, charter: &str, persona_slug: &str, output: &Path) {
    let envelope = json!({
        "stage": "dialogue_captured",
        "dialogue": d,
        "rendered_charter": charter,
        "persona_slug": persona_slug,
        "output_dir": output.display().to_string(),
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&envelope).unwrap_or_else(|_| "{}".into())
    );
}

#[cfg(test)]
#[path = "genesis_interactive_tests.rs"]
mod tests;
