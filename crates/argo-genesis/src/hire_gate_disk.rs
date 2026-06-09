//! Disk-reconstruction half of the hire gate.
//!
//! Extracted from `hire_gate.rs` to keep that file under the per-file LOC
//! budget. The flagship SSE path builds gate inputs straight off the
//! in-memory [`crate::GenesisPlan`]; the hire-commit fence runs LATER, against
//! a teammate whose vertical is already on disk and whose original plan is
//! gone. This module reconstructs the SAME deterministic inputs + safety-judge
//! projection + truthful role contract from the on-disk `eval-seed.toml`,
//! `prompts/system.md`, and `vertical.toml`, so the SSE path and the hire
//! fence re-run the byte-identical gate (fail-closed parity — no per-surface
//! divergence).

use crate::eval_seed::EvalSeed;
use crate::genesis_judge::JudgeInput;
use crate::hire_gate::{to_input, DeterministicEvalInput};
use std::path::Path;

/// Owned hire-gate inputs reconstructed from a written vertical directory.
#[derive(Debug, Clone)]
pub struct DiskGateInputs {
    /// The deterministic seed tasks parsed from `eval-seed.toml`.
    pub seeds: Vec<DeterministicEvalInput>,
    /// The behaviour contract (lowercased) the deterministic subject answers
    /// with — the on-disk system prompt plus the action vocabulary.
    pub contract_text: String,
    /// Charter text for the safety-judge projection (operator's words).
    pub charter: String,
    /// Persona handle for the safety-judge projection.
    pub persona: String,
    /// Action vocabulary for the safety-judge projection.
    pub action_vocabulary: Vec<String>,
    /// The full parsed manifest, retained so the hire-commit fence can derive
    /// the teammate's truthful [`crate::role_contract::RoleContract`] from the
    /// IDENTICAL `[actions]`/`[escalation]`/`[mcp_servers]` source the SSE
    /// path derives from — no per-surface divergence in what the operator
    /// confirms.
    pub manifest: argo_vertical::VerticalManifest,
}

impl DiskGateInputs {
    /// The teammate's truthful role contract, derived from the on-disk
    /// manifest's actual bound capability profile.
    #[must_use]
    pub fn role_contract(&self) -> crate::role_contract::RoleContract {
        crate::role_contract::RoleContract::derive_from_manifest(&self.manifest)
    }

    /// Borrowed safety-judge projection — same shape both gate paths feed
    /// the judge. Built from the retained manifest so each action's
    /// `requires_approval` flag rides parallel to its name (an approval-gated
    /// dangerous action is safe by construction; only an autonomous one is
    /// block-worthy). Names + flags share one `actions` iteration, so the
    /// disk-fence projection matches the SSE-path projection byte for byte.
    #[must_use]
    pub fn judge_input(&self) -> JudgeInput<'_> {
        JudgeInput::from_plan_projection(
            &self.charter,
            &self.persona,
            self.manifest.actions.as_ref(),
        )
    }
}

#[derive(serde::Deserialize)]
struct SeedDoc {
    #[serde(default)]
    seed: Vec<EvalSeed>,
}

/// Reconstruct hire-gate inputs from a written Genesis vertical directory.
///
/// Reads `eval-seed.toml` (the 10 starter seeds + rubric keywords),
/// `prompts/system.md` (the behaviour contract), and `vertical.toml` (name,
/// description, persona, action vocabulary). Fails closed: a missing /
/// unparseable `eval-seed.toml` yields an empty seed set, which the eval
/// floor treats as a `Blocked` verdict.
///
/// # Errors
///
/// Returns [`crate::GenesisError`] only when `vertical.toml` is missing or
/// unparseable — a teammate whose manifest cannot be read must not be
/// activated.
///
/// # Examples
///
/// ```no_run
/// # fn demo() -> Result<(), argo_genesis::GenesisError> {
/// let dir = std::path::Path::new("/tmp/v/.genesis/op/marketing-intern-maya");
/// let inputs = argo_genesis::gate_inputs_from_dir(dir)?;
/// // The truthful role contract is derivable from the on-disk manifest.
/// let _contract = inputs.role_contract();
/// # Ok(()) }
/// ```
pub fn gate_inputs_from_dir(vertical_dir: &Path) -> Result<DiskGateInputs, crate::GenesisError> {
    let manifest_body = std::fs::read_to_string(vertical_dir.join("vertical.toml"))
        .map_err(|e| crate::GenesisError::Io {
            what: "hire_gate.manifest.read",
            source: e,
        })?;
    let manifest: argo_vertical::VerticalManifest =
        toml::from_str(&manifest_body).map_err(|e| crate::GenesisError::Validation {
            stage: "hire_gate.manifest.parse",
            detail: e.to_string(),
        })?;

    let seeds = parse_seed_file(vertical_dir);
    let contract_text = disk_contract_text(vertical_dir, &manifest);
    let action_vocabulary = manifest
        .actions
        .as_ref()
        .map(|a| a.actions.keys().cloned().collect())
        .unwrap_or_default();

    Ok(DiskGateInputs {
        seeds,
        contract_text,
        // The verbatim charter isn't persisted as its own field; the system
        // prompt embeds it under "## Charter (operator's words)". Use the
        // vertical description as the charter projection for the judge — the
        // judge also sees the full action vocabulary, which is what the
        // restricted-surface question keys off.
        charter: manifest.vertical.description.clone(),
        // The persona handle isn't a standalone manifest field (role.persona
        // is a doc PATH); the human-readable vertical name is the closest
        // persona projection for the judge's impersonation question.
        persona: manifest.vertical.name.clone(),
        action_vocabulary,
        manifest,
    })
}

fn parse_seed_file(vertical_dir: &Path) -> Vec<DeterministicEvalInput> {
    let Ok(body) = std::fs::read_to_string(vertical_dir.join("eval-seed.toml")) else {
        return Vec::new();
    };
    let Ok(doc) = toml::from_str::<SeedDoc>(&body) else {
        return Vec::new();
    };
    doc.seed.iter().map(to_input).collect()
}

fn disk_contract_text(vertical_dir: &Path, manifest: &argo_vertical::VerticalManifest) -> String {
    let mut out = std::fs::read_to_string(vertical_dir.join("prompts").join("system.md"))
        .unwrap_or_default();
    out.push('\n');
    out.push_str(&manifest.vertical.name);
    out.push('\n');
    out.push_str(&manifest.vertical.description);
    if let Some(actions) = manifest.actions.as_ref() {
        for (name, decl) in &actions.actions {
            out.push('\n');
            out.push_str(name);
            out.push(' ');
            out.push_str(&decl.description);
        }
    }
    out.to_lowercase()
}
