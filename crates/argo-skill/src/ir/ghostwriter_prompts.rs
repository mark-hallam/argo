//! Ghostwriter prompt templates (Track L brief: L6).
//!
//! Two templates live here:
//! - [`build_initial_prompt`] — first-pass structured-output prompt
//!   that asks the model for a JSON `SkillGraph` and lists the schema's
//!   shape, the 9 node kinds, and the edge-condition vocabulary.
//! - [`build_self_correction_prompt`] — retry prompt fed on the first
//!   path's failure. It quotes the malformed model output verbatim and
//!   names the specific validation error so the second pass has the
//!   feedback it needs to converge.
//!
//! The templates are pure functions of their inputs (no random tokens,
//! no time-of-day strings); the same prompt is built every call so the
//! audit row can capture it without an out-of-band log.

/// Field-by-field shape description shared by the initial + self-correction
/// prompts. Lists the JSON keys, the 9 `NodeKind` variants, and the 4
/// `EdgeCond` variants the model must choose from.
const SCHEMA_HINT: &str = r#"
Output a single JSON object with this exact shape (camelCase keys):

{
  "entry": "<id of the first node>",
  "nodes": [
    {
      "id": "<unique node id within graph>",
      "label": "<human readable>",
      "kind": { "type": "<one of: action|condition|branch|await-approval|fetch|transform|sub-skill-call|loop|try-catch>", ...kind-specific fields... },
      "annotations": {}
    }
  ],
  "edges": [
    {
      "id": "<unique edge id>",
      "from": "<source node id>",
      "to": "<dest node id>",
      "cond": { "kind": "<one of: always|on-success|on-failure|on-pattern>" }
    }
  ],
  "variables": [],
  "metadata": { "name": "<string>", "description": "<string>", "source": "ghostwriter" }
}

Per-kind fields:
  action:          { "type": "action", "instruction": "<string>" }
  condition:       { "type": "condition", "expression": "<string>" }
  branch:          { "type": "branch", "arms": [{ "label": "<string>", "pattern": "<string>" }] }
  await-approval:  { "type": "await-approval", "approverRole": "<string>", "timeoutSeconds": <integer|null> }
  fetch:           { "type": "fetch", "target": "<string>", "method": "<GET|POST|...>" }
  transform:       { "type": "transform", "template": "<string>" }
  sub-skill-call:  { "type": "sub-skill-call", "target": { "skillId": "<hex>", "version": "<semver>" } }
  loop:            { "type": "loop", "iterVar": "<string>", "body": ["<node id>", ...] }
  try-catch:       { "type": "try-catch", "tryBody": [...node ids], "catchBody": [...node ids] }

Hard constraints:
  - "entry" MUST be the id of an existing node.
  - Every edge.from and edge.to MUST reference a declared node id.
  - For non-trivial prompts emit AT LEAST 4 nodes covering the natural shape
    of the work (fetch → transform → decide → act). Use awaitApproval for any
    step that spends money, sends external messages, or mutates customer data.
  - Output ONLY the JSON object — no prose, no markdown fence, no commentary.
"#;

/// First-pass Ghostwriter prompt. Renders the operator's natural-language
/// description into the structured-output ask the model must satisfy.
#[must_use]
pub fn build_initial_prompt(operator_prompt: &str) -> String {
    format!(
        "You are Ghostwriter, a structured-output assistant for the Skycot \
         agent platform. The operator has described a skill they want their \
         worker agent to perform. Translate the description into a valid \
         SkillGraph IR document.\n\n\
         Operator description:\n\
         <<<\n{operator}\n>>>\n{schema}",
        operator = operator_prompt.trim(),
        schema = SCHEMA_HINT,
    )
}

/// Self-correction prompt fed on the first attempt's failure. Echoes the
/// validation error and the model's prior output verbatim so the model
/// can repair its own mistake instead of starting from scratch.
#[must_use]
pub fn build_self_correction_prompt(
    operator_prompt: &str,
    prior_output: &str,
    validation_error: &str,
) -> String {
    format!(
        "Your previous SkillGraph output for this operator description \
         failed validation:\n\n\
         Operator description:\n<<<\n{operator}\n>>>\n\n\
         Your previous output:\n<<<\n{prior}\n>>>\n\n\
         Validation error:\n  {err}\n\n\
         Re-emit a corrected SkillGraph that fixes the named error. Keep \
         every node that already worked; only mutate what the error \
         references. Output ONLY the JSON object — no prose, no markdown.\n\
         {schema}",
        operator = operator_prompt.trim(),
        prior = prior_output.trim(),
        err = validation_error.trim(),
        schema = SCHEMA_HINT,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_prompt_carries_operator_text_and_schema() {
        let p = build_initial_prompt("Build a skill that reconciles bank statements");
        assert!(p.contains("reconciles bank statements"));
        assert!(p.contains("\"entry\""));
        assert!(p.contains("await-approval"));
        assert!(p.contains("on-pattern"));
    }

    #[test]
    fn self_correction_quotes_prior_output_and_error() {
        let p = build_self_correction_prompt(
            "Fetch invoices",
            "{ broken: json }",
            "expected `,` or `}` at line 1",
        );
        assert!(p.contains("Fetch invoices"));
        assert!(p.contains("broken: json"));
        assert!(p.contains("expected `,`"));
        assert!(p.contains("\"entry\""));
    }
}
