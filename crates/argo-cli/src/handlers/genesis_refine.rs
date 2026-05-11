//! `argo genesis refine <vertical-id>` handler.
use argo_genesis::archetype::classify_charter;
use argo_genesis::refine::{http::ReqwestCaller, AnthropicCaller, RefinementResponse};
use argo_genesis::{refine_plan_with_cap, write_vertical, GenesisPlan, RefineConfig};
use argo_types::{ArgoError, ArgoResult};
use argo_vertical::{ActionDeclaration, ExecutorDeclaration, LoadedVertical};
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone)]
pub(crate) struct RefineOptions {
    pub vertical_id: String,
    pub judge: bool,
    pub max_iterations: usize,
    pub dry_run: bool,
    pub max_vocab_size: usize,
}

#[derive(Debug, Serialize)]
pub(crate) struct RefineOutcome {
    pub vertical_id: String,
    pub dry_run: bool,
    pub judge: bool,
    pub max_iterations: usize,
    pub changed: bool,
    pub diff: String,
    pub output_dir: String,
    pub archive_dir: Option<String>,
    /// Action_types newly added to the manifest's vocabulary
    /// across all refinement iterations. Always present (empty
    /// when no growth) for JSON consumers; the human path only
    /// prints under `--show-vocab-evolution`.
    pub evolved_vocab: Vec<String>,
}

pub(crate) fn run(
    vertical_id: &str,
    _judge_flag: bool,
    no_judge: bool,
    max_iterations: usize,
    dry_run: bool,
    json: bool,
    show_vocab_evolution: bool,
) -> ArgoResult<()> {
    if max_iterations == 0 {
        return Err(ArgoError::Provider(
            "--max-iterations must be greater than zero".into(),
        ));
    }
    let config = RefineConfig::from_env().map_err(ArgoError::from)?;
    let max_vocab_size = config.max_vocab_size;
    let caller = ReqwestCaller::new(config).map_err(ArgoError::from)?;
    let opts = RefineOptions {
        vertical_id: vertical_id.to_string(),
        judge: !no_judge,
        max_iterations,
        dry_run,
        max_vocab_size,
    };
    let handle = tokio::runtime::Handle::try_current()
        .map_err(|e| ArgoError::Provider(format!("genesis refine needs a tokio runtime: {e}")))?;
    let outcome = tokio::task::block_in_place(|| handle.block_on(refine_existing(&opts, &caller)))?;
    print_outcome(&outcome, json, show_vocab_evolution);
    Ok(())
}

pub(crate) async fn refine_existing(
    opts: &RefineOptions,
    caller: &dyn AnthropicCaller,
) -> ArgoResult<RefineOutcome> {
    let verticals_dir = argo_vertical::default_dir();
    let (registry, _errors) = argo_vertical::load_dir(&verticals_dir)?;
    let loaded = registry.get(&opts.vertical_id).ok_or_else(|| {
        ArgoError::Provider(format!(
            "vertical '{}' not found in {}",
            opts.vertical_id,
            verticals_dir.display()
        ))
    })?;
    let original_plan = plan_from_loaded(loaded)?;
    let original_toml = manifest_toml(&original_plan)?;
    let mut refined = original_plan.clone();
    let mut evolved_vocab: Vec<String> = Vec::new();
    for _ in 0..opts.max_iterations {
        let (plan, evolved) = refine_once(refined, caller, opts.judge, opts.max_vocab_size).await?;
        refined = plan;
        evolved_vocab.extend(evolved);
    }
    let refined_toml = manifest_toml(&refined)?;
    let diff = render_diff(&original_toml, &refined_toml);
    let archive_dir = if opts.dry_run {
        None
    } else {
        let archive = argo_vertical::archive_existing_vertical(&loaded.root, &opts.vertical_id)?;
        write_vertical(&refined, &loaded.root).map_err(ArgoError::from)?;
        Some(archive)
    };
    Ok(RefineOutcome {
        vertical_id: opts.vertical_id.clone(),
        dry_run: opts.dry_run,
        judge: opts.judge,
        max_iterations: opts.max_iterations,
        changed: original_toml != refined_toml,
        diff,
        output_dir: loaded.root.display().to_string(),
        archive_dir: archive_dir.map(|p| p.display().to_string()),
        evolved_vocab,
    })
}

async fn refine_once(
    plan: GenesisPlan,
    caller: &dyn AnthropicCaller,
    judge: bool,
    max_vocab_size: usize,
) -> ArgoResult<(GenesisPlan, Vec<String>)> {
    if judge {
        return refine_plan_with_cap(plan, caller, max_vocab_size)
            .await
            .map_err(ArgoError::from);
    }
    let raw = match caller.call(build_refine_prompt(&plan)).await {
        Ok(raw) => raw,
        Err(_) => return Ok((plan, Vec::new())),
    };
    let response = match serde_json::from_str::<RefinementResponse>(&raw) {
        Ok(response) if response.additional_actions.len() <= 3 => response,
        _ => return Ok((plan, Vec::new())),
    };
    Ok(apply_without_judge(plan, response, max_vocab_size))
}

fn plan_from_loaded(loaded: &LoadedVertical) -> ArgoResult<GenesisPlan> {
    let manifest = loaded.manifest.clone();
    let charter = extract_charter(&manifest, &loaded.root)?;
    let archetype = classify_charter(&charter).map_err(ArgoError::from)?;
    let persona = extract_persona(&manifest);
    Ok(GenesisPlan {
        charter,
        persona,
        archetype,
        vertical_id: manifest.vertical.id.clone(),
        manifest,
    })
}

fn extract_charter(manifest: &argo_vertical::VerticalManifest, root: &Path) -> ArgoResult<String> {
    let prefix = "Generated by Genesis from charter: ";
    if let Some(charter) = manifest.vertical.description.strip_prefix(prefix) {
        return Ok(charter.trim().to_string());
    }
    let role = manifest
        .role
        .as_ref()
        .ok_or_else(|| ArgoError::Provider("vertical has no role.system_prompt".into()))?;
    let system_path = root.join(&role.system_prompt);
    let body = std::fs::read_to_string(&system_path)
        .map_err(|e| ArgoError::Io(format!("read {}: {e}", system_path.display())))?;
    extract_charter_from_system_prompt(&body).ok_or_else(|| {
        ArgoError::Provider(format!(
            "could not recover charter from {}",
            system_path.display()
        ))
    })
}

fn extract_charter_from_system_prompt(body: &str) -> Option<String> {
    let rest = body.split("## Charter (operator's words)").nth(1)?;
    let charter = rest.split("\n## ").next().unwrap_or(rest).trim();
    (!charter.is_empty()).then(|| charter.to_string())
}

fn extract_persona(manifest: &argo_vertical::VerticalManifest) -> String {
    manifest
        .vertical
        .name
        .rsplit_once('(')
        .and_then(|(_, tail)| tail.strip_suffix(')'))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| manifest.vertical.id.rsplit('-').next().map(str::to_string))
        .unwrap_or_else(|| "worker".into())
}

fn apply_without_judge(
    mut plan: GenesisPlan,
    response: RefinementResponse,
    max_vocab_size: usize,
) -> (GenesisPlan, Vec<String>) {
    let mut evolved = Vec::new();
    if let Some(actions) = plan.manifest.actions.as_mut() {
        for proposed in response.additional_actions {
            if actions.actions.contains_key(&proposed.action_type) {
                continue;
            }
            if actions.actions.len() >= max_vocab_size {
                continue;
            }
            let name = proposed.action_type.clone();
            actions.actions.insert(
                name.clone(),
                ActionDeclaration {
                    risk: proposed.risk,
                    requires_approval: proposed.requires_approval,
                    description: proposed.description,
                    executor: ExecutorDeclaration::default(),
                    ..ActionDeclaration::default()
                },
            );
            evolved.push(name);
        }
    }
    if !response.system_prompt_addendum.is_empty() {
        plan.manifest.vertical.description = format!(
            "{}\n\n## Refined addendum\n{}",
            plan.manifest.vertical.description, response.system_prompt_addendum
        );
    }
    (plan, evolved)
}

fn manifest_toml(plan: &GenesisPlan) -> ArgoResult<String> {
    toml::to_string_pretty(&plan.manifest)
        .map_err(|e| ArgoError::Schema(format!("serialise refined manifest: {e}")))
}

fn render_diff(before: &str, after: &str) -> String {
    if before == after {
        return "No manifest changes.".into();
    }
    let before_lines: Vec<_> = before.lines().collect();
    let after_lines: Vec<_> = after.lines().collect();
    let mut out = String::from("--- current vertical.toml\n+++ refined vertical.toml\n");
    let max = before_lines.len().max(after_lines.len());
    for idx in 0..max {
        match (before_lines.get(idx), after_lines.get(idx)) {
            (Some(a), Some(b)) if a == b => {}
            (Some(a), Some(b)) => {
                out.push_str(&format!("-{a}\n+{b}\n"));
            }
            (Some(a), None) => out.push_str(&format!("-{a}\n")),
            (None, Some(b)) => out.push_str(&format!("+{b}\n")),
            (None, None) => {}
        }
    }
    out
}

fn build_refine_prompt(plan: &GenesisPlan) -> String {
    let action_names: Vec<&str> = plan
        .manifest
        .actions
        .as_ref()
        .map(|a| a.actions.keys().map(String::as_str).collect())
        .unwrap_or_default();
    format!(
        "You're refining a draft Skycot vertical generated from a charter.\n\n\
         Charter: {charter}\n\n\
         Deterministic archetype: {archetype_label}\n\
         Deterministic actions: [{actions}]\n\n\
         Respond with a JSON object exactly matching this schema:\n\
         {{\n\
           \"archetype_confirmed\": <bool>,\n\
           \"archetype_suggested\": <string or null>,\n\
           \"additional_actions\": [\n\
             {{ \"action_type\": <snake_case>, \"risk\": \"low\"|\"medium\"|\"high\",\n\
                \"requires_approval\": <bool>, \"description\": <string> }}\n\
           ] (max 3 entries),\n\
           \"system_prompt_addendum\": <string up to 4096 chars>\n\
         }}\n\n\
         Do not propose actions that already exist. Do not propose actions \
         that touch external systems autonomously — sensitive verbs (send, \
         post, pay, publish, delete, share, message) MUST be \
         requires_approval=true.",
        charter = plan.charter,
        archetype_label = plan.archetype.label(),
        actions = action_names.join(", "),
    )
}

fn print_outcome(outcome: &RefineOutcome, json: bool, show_vocab_evolution: bool) {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({"genesis_refine": outcome}))
                .unwrap_or_else(|_| "{}".into())
        );
        return;
    }
    println!("Refined vertical: {}", outcome.vertical_id);
    println!("  dry_run: {}", outcome.dry_run);
    println!("  judge:   {}", outcome.judge);
    println!("  output:  {}", outcome.output_dir);
    if let Some(archive) = &outcome.archive_dir {
        println!("  archived previous version: {archive}");
    }
    if show_vocab_evolution {
        if outcome.evolved_vocab.is_empty() {
            println!("  evolved vocabulary: (none)");
        } else {
            println!("  evolved vocabulary:");
            for name in &outcome.evolved_vocab {
                println!("    - {name}");
            }
        }
    }
    println!("{}", outcome.diff);
}

#[cfg(test)]
mod tests {
    use super::*;
    use argo_genesis::AnthropicCaller;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct CountingCaller {
        calls: AtomicUsize,
        response: String,
    }

    #[async_trait::async_trait]
    impl AnthropicCaller for CountingCaller {
        async fn call(&self, _prompt: String) -> Result<String, argo_genesis::error::GenesisError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.response.clone())
        }
    }

    fn response(action: &str, requires_approval: bool) -> String {
        serde_json::json!({
            "archetype_confirmed": true,
            "archetype_suggested": null,
            "additional_actions": [{
                "action_type": action,
                "risk": "high",
                "requires_approval": requires_approval,
                "description": "Send a sensitive thing"
            }],
            "system_prompt_addendum": "Prefer short operator check-ins."
        })
        .to_string()
    }

    #[test]
    fn extracts_charter_from_system_prompt() {
        let body =
            "# X\n\n## Charter (operator's words)\n\nbe my marketing intern\n\n## How to behave\n";
        assert_eq!(
            extract_charter_from_system_prompt(body).as_deref(),
            Some("be my marketing intern")
        );
    }

    #[tokio::test]
    async fn no_judge_keeps_model_approval_flag() {
        let plan = argo_genesis::plan("be my marketing intern", "maya").unwrap();
        let caller = CountingCaller {
            calls: AtomicUsize::new(0),
            response: response("send_special_offer", false),
        };
        let (refined, _evolved) = refine_once(
            plan,
            &caller,
            false,
            RefineConfig::DEFAULT_MAX_VOCAB_SIZE,
        )
        .await
        .unwrap();
        let action = refined
            .manifest
            .actions
            .unwrap()
            .actions
            .get("send_special_offer")
            .unwrap()
            .clone();
        assert!(!action.requires_approval);
    }

    #[tokio::test]
    async fn max_iterations_invokes_caller_each_time() {
        let plan = argo_genesis::plan("be my marketing intern", "maya").unwrap();
        let caller = CountingCaller {
            calls: AtomicUsize::new(0),
            response: response("review_campaign", false),
        };
        let opts = RefineOptions {
            vertical_id: "unused".into(),
            judge: false,
            max_iterations: 2,
            dry_run: true,
            max_vocab_size: RefineConfig::DEFAULT_MAX_VOCAB_SIZE,
        };
        let mut refined = plan;
        for _ in 0..opts.max_iterations {
            let (plan, _evolved) =
                refine_once(refined, &caller, opts.judge, opts.max_vocab_size)
                    .await
                    .unwrap();
            refined = plan;
        }
        assert_eq!(caller.calls.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn render_diff_marks_changed_lines() {
        let diff = render_diff("a\nb\n", "a\nc\n");
        assert!(diff.contains("-b"));
        assert!(diff.contains("+c"));
    }
}
