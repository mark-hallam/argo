//! Deterministic starter eval-seed generation for Genesis.

use argo_types::ArgoResult;
use argo_vertical::ActionsSection;
use serde::{Deserialize, Serialize};

const SEED_COUNT: usize = 10;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x00000100000001b3;

/// One starter task the eval runner can grade.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvalSeed {
    /// Stable hash id derived from the task text.
    pub id: String,
    /// Task prompt to present to the worker.
    pub task: String,
    /// Coarse expected output summary.
    pub expected_summary: String,
    /// Difficulty tier for sampling and reporting.
    pub difficulty: Difficulty,
    /// Lowercase, punctuation-free judge keywords.
    pub rubric_keywords: Vec<String>,
}

/// Seed difficulty. Genesis emits a 40/40/20 split.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Difficulty {
    /// Simple single-step task.
    Easy,
    /// Multi-step but routine task.
    Medium,
    /// Ambiguous, risky, or escalation-heavy task.
    Hard,
}

/// Normalised action metadata used by the seed generator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuggestedAction {
    /// Manifest action id.
    pub name: String,
    /// Human-readable action description.
    pub description: String,
}

/// Generate 10 starter eval seeds from the role description and actions.
pub fn generate(role_description: &str, actions: &[SuggestedAction]) -> ArgoResult<Vec<EvalSeed>> {
    let role = clean_sentence(role_description)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "worker role".into());
    let primary = primary_keyword(&role).unwrap_or_else(|| "worker".into());
    let candidates = action_candidates(actions, &primary);
    let mut seeds = Vec::with_capacity(SEED_COUNT);
    for idx in 0..SEED_COUNT {
        let action = &candidates[idx % candidates.len()];
        let difficulty = difficulty_for(idx);
        let task = task_for(idx, difficulty, &role, action);
        let expected_summary = expected_for(difficulty, action);
        let rubric_keywords = keywords(&primary, action);
        seeds.push(EvalSeed {
            id: stable_id(&task),
            task,
            expected_summary,
            difficulty,
            rubric_keywords,
        });
    }
    Ok(seeds)
}

/// Convert manifest actions into generator input.
#[must_use]
pub fn suggested_actions_from_manifest(actions: &ActionsSection) -> Vec<SuggestedAction> {
    actions
        .actions
        .iter()
        .map(|(name, decl)| SuggestedAction {
            name: name.clone(),
            description: decl.description.clone(),
        })
        .collect()
}

/// Render seeds as TOML using `[[seed]]` tables.
#[must_use]
pub fn to_toml(seeds: &[EvalSeed]) -> String {
    let mut out = String::new();
    for seed in seeds {
        out.push_str("[[seed]]\n");
        out.push_str(&format!("id = \"{}\"\n", escape(&seed.id)));
        out.push_str(&format!("task = \"{}\"\n", escape(&seed.task)));
        out.push_str(&format!(
            "expected_summary = \"{}\"\n",
            escape(&seed.expected_summary)
        ));
        out.push_str(&format!(
            "difficulty = \"{}\"\n",
            difficulty_label(seed.difficulty)
        ));
        out.push_str("rubric_keywords = [");
        for (idx, keyword) in seed.rubric_keywords.iter().enumerate() {
            if idx > 0 {
                out.push_str(", ");
            }
            out.push_str(&format!("\"{}\"", escape(keyword)));
        }
        out.push_str("]\n\n");
    }
    out
}

fn action_candidates(actions: &[SuggestedAction], primary: &str) -> Vec<SuggestedAction> {
    if actions.is_empty() {
        return vec![
            SuggestedAction {
                name: primary.to_string(),
                description: format!("Handle a {primary} task"),
            },
            SuggestedAction {
                name: "summarize_context".into(),
                description: "Summarize the relevant context".into(),
            },
        ];
    }
    actions.to_vec()
}

fn difficulty_for(idx: usize) -> Difficulty {
    match idx {
        0..=3 => Difficulty::Easy,
        4..=7 => Difficulty::Medium,
        _ => Difficulty::Hard,
    }
}

fn task_for(idx: usize, difficulty: Difficulty, role: &str, action: &SuggestedAction) -> String {
    let scenario = idx + 1;
    match difficulty {
        Difficulty::Easy => format!(
            "Scenario {scenario}: use `{}` to handle a straightforward {} request: {}.",
            action.name, role, action.description
        ),
        Difficulty::Medium => format!(
            "Scenario {scenario}: review a realistic {} case, run `{}`, and explain the expected next step.",
            role, action.name
        ),
        Difficulty::Hard => format!(
            "Scenario {scenario}: handle an ambiguous {} workflow using `{}`, identify blockers, and state when human review is required.",
            role, action.name
        ),
    }
}

fn expected_for(difficulty: Difficulty, action: &SuggestedAction) -> String {
    match difficulty {
        Difficulty::Easy => format!("Completes `{}` with a concise result.", action.name),
        Difficulty::Medium => format!(
            "Uses `{}` and explains assumptions, evidence, and follow-up.",
            action.name
        ),
        Difficulty::Hard => format!(
            "Uses `{}`, surfaces risk, and escalates before irreversible work.",
            action.name
        ),
    }
}

fn keywords(primary: &str, action: &SuggestedAction) -> Vec<String> {
    let mut out = Vec::new();
    push_keyword(&mut out, primary);
    for part in action.name.split('_') {
        push_keyword(&mut out, part);
    }
    for part in action.description.split_whitespace() {
        push_keyword(&mut out, part);
    }
    out.truncate(5);
    if out.is_empty() {
        out.push("worker".into());
    }
    out
}

fn push_keyword(out: &mut Vec<String>, raw: &str) {
    let cleaned = clean_keyword(raw);
    if cleaned.len() >= 3 && !out.contains(&cleaned) {
        out.push(cleaned);
    }
}

fn primary_keyword(role: &str) -> Option<String> {
    role.split_whitespace()
        .map(clean_keyword)
        .find(|w| w.len() >= 3 && !is_stopword(w))
}

fn clean_sentence(raw: &str) -> Option<String> {
    let words: Vec<_> = raw.split_whitespace().collect();
    (!words.is_empty()).then(|| words.join(" "))
}

fn clean_keyword(raw: &str) -> String {
    raw.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn is_stopword(word: &str) -> bool {
    matches!(
        word,
        "the" | "and" | "for" | "with" | "that" | "this" | "role" | "agent"
    )
}

fn stable_id(task: &str) -> String {
    let mut hash = FNV_OFFSET;
    for byte in task.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    format!("seed-{hash:016x}")
}

fn difficulty_label(difficulty: Difficulty) -> &'static str {
    match difficulty {
        Difficulty::Easy => "easy",
        Difficulty::Medium => "medium",
        Difficulty::Hard => "hard",
    }
}

fn escape(raw: &str) -> String {
    raw.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
#[path = "eval_seed_tests.rs"]
mod tests;
