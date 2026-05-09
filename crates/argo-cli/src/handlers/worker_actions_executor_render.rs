//! Bundle-rendering helpers for `HumanExecutor` (#731 / #732).
//!
//! Pure functions that turn an `ActionRecord` (plus optional
//! manifest-supplied template) into the operator-facing markdown
//! files written under `.skycot/outputs/<job>/actions/<action>/`.
//! Split out of `worker_actions_executor.rs` to keep that file
//! under the 500-LOC ceiling; behaviour is unchanged.

use super::worker_actions::ActionRecord;
use argo_vertical::ActionDeclaration;

/// Choose which instruction template to use for an action, in
/// the precedence order:
///   1. `instruction_templates.<platform>` (per-platform override)
///   2. `instruction_template` (default for this action_type)
///   3. `None` → executor falls back to the generic dump
pub(crate) fn pick_template<'a>(
    decl: &'a ActionDeclaration,
    platform: Option<&str>,
) -> Option<&'a str> {
    if let Some(p) = platform {
        if let Some(per_platform) = decl.instruction_templates.get(p) {
            return Some(per_platform.as_str());
        }
    }
    decl.instruction_template.as_deref()
}

/// Render an instruction template against the action's fields.
/// Tiny substitution engine — no Handlebars dep, just literal
/// `{{ key }}` and `{{ payload.X }}` lookups. Unknown keys
/// render as `(unset)` so operators see the gap rather than a
/// confusing literal.
pub(crate) fn render_with_template(action: &ActionRecord, body: &str) -> String {
    let mut out = String::with_capacity(body.len() + 64);
    let bytes = body.as_bytes();
    let mut cursor = 0;
    while cursor < bytes.len() {
        let Some(start) = body[cursor..].find("{{") else {
            out.push_str(&body[cursor..]);
            break;
        };
        out.push_str(&body[cursor..cursor + start]);
        let open = cursor + start;
        let Some(rel_end) = body[open + 2..].find("}}") else {
            // Unmatched `{{` — emit verbatim and stop.
            out.push_str(&body[open..]);
            break;
        };
        let close = open + 2 + rel_end;
        let key = body[open + 2..close].trim();
        let resolved = lookup_template_key(action, key);
        out.push_str(&resolved);
        cursor = close + 2;
    }
    out
}

fn lookup_template_key(action: &ActionRecord, key: &str) -> String {
    match key {
        "action_id" => action.action_id.clone(),
        "action_type" => action.action_type.clone(),
        "job_id" => action.job_id.clone(),
        "risk_level" => action.risk_level.clone(),
        "platform" => action.platform.clone().unwrap_or_else(|| "(unset)".into()),
        "item_id" => action.item_id.clone().unwrap_or_else(|| "(unset)".into()),
        "approved_by" => action
            .approved_by
            .clone()
            .unwrap_or_else(|| "(unset)".into()),
        "approved_at" => action
            .approved_at
            .clone()
            .unwrap_or_else(|| "(unset)".into()),
        other => {
            if let Some(rest) = other.strip_prefix("payload.") {
                return lookup_payload_field(&action.payload, rest);
            }
            "(unset)".to_string()
        }
    }
}

/// Lookup a dotted path inside the action's payload Value. Each
/// segment must resolve to a string-coercible value at the leaf;
/// missing segments return `(unset)`. Numbers + booleans render
/// via their canonical to_string.
fn lookup_payload_field(payload: &serde_json::Value, path: &str) -> String {
    let mut cur = payload;
    for seg in path.split('.') {
        match cur {
            serde_json::Value::Object(map) => match map.get(seg) {
                Some(v) => cur = v,
                None => return "(unset)".to_string(),
            },
            _ => return "(unset)".to_string(),
        }
    }
    match cur {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => "(unset)".to_string(),
        other => other.to_string(),
    }
}

/// Render `instructions.md` — the operator-facing markdown brief.
/// v1 is intentionally simple: action_type, platform, item, the
/// payload as JSON, and a "next step" footer pointing back at
/// `argo worker actions complete`. #732 layers on per-action-
/// type templates so e.g. `post_listing` produces a marketplace-
/// specific checklist instead of the generic dump.
pub(crate) fn render_instructions(action: &ActionRecord) -> String {
    let platform = action
        .platform
        .as_deref()
        .map(|p| format!(" on `{p}`"))
        .unwrap_or_default();
    let item = action
        .item_id
        .as_deref()
        .map(|i| format!(" (item `{i}`)"))
        .unwrap_or_default();
    let mut out = format!(
        "# Action: `{}`{platform}{item}\n\n\
         _action_id: `{}`_  \n\
         _job: `{}`_  \n\
         _risk: {}_  \n\n",
        action.action_type, action.action_id, action.job_id, action.risk_level,
    );
    if action.requires_approval {
        match (action.approved_by.as_deref(), action.approved_at.as_deref()) {
            (Some(by), Some(at)) => {
                out.push_str(&format!("**Approved** by `{by}` at `{at}`. Proceed.\n\n"));
            }
            _ => {
                out.push_str(
                    "**⚠️ Approval-gated action**. The OS expected approval before \
                              `executing` — check `argo worker actions status` for the trail.\n\n",
                );
            }
        }
    } else {
        out.push_str("Autonomous-safe action. No approval was required.\n\n");
    }
    out.push_str("## Payload\n\n```json\n");
    out.push_str(
        &serde_json::to_string_pretty(&action.payload).unwrap_or_else(|_| "{}".to_string()),
    );
    out.push_str("\n```\n\n");
    out.push_str(
        "## Next step\n\n\
         Carry the action out offline. When complete, run:\n\n\
         ```\n\
         argo worker actions complete <action_id> --note \"<short summary>\" \
         [--result-json '{\"key\": \"value\"}']\n\
         ```\n\n\
         If something blocks the action, run `argo worker actions fail <action_id> \
         --reason \"<why>\"` instead.\n",
    );
    out
}

/// Render `evidence.md` — the cited research backing this
/// action. Empty-state placeholder when the action carried no
/// evidence (legitimate for autonomous-safe actions where
/// research wasn't needed).
pub(crate) fn render_evidence(action: &ActionRecord) -> String {
    let mut out = format!(
        "# Evidence for action `{}`\n\n\
         _action_id: `{}`_  \n_job: `{}`_  \n\n",
        action.action_type, action.action_id, action.job_id,
    );
    if action.evidence.is_empty() {
        out.push_str("_(no evidence attached to this action)_\n");
        return out;
    }
    out.push_str(&format!(
        "{} source(s) cited at emission time:\n\n",
        action.evidence.len()
    ));
    for url in &action.evidence {
        out.push_str(&format!("- {url}\n"));
    }
    out
}
