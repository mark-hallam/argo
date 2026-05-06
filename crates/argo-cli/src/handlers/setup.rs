//! `argo setup` guided onboarding and CI provider checks.
use crate::home::{ensure_dir, resolve_home};
use argo_types::{ArgoError, ArgoResult};
use std::io::IsTerminal;

pub async fn run(
    check: bool,
    require: Vec<String>,
    non_interactive: bool,
    json: bool,
) -> ArgoResult<()> {
    if check {
        return run_check(&require);
    }
    if !require.is_empty() {
        return Err(ArgoError::Provider(
            "--require has no effect without --check (it's a CI / launch-checklist flag)".into(),
        ));
    }
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let interactive = !non_interactive && std::io::stdin().is_terminal();
    let steps = super::setup_steps::collect_steps(&cwd, &home, interactive).await?;
    super::setup_render::print(&steps, json)
}

fn detect_provider_keys() -> Vec<&'static str> {
    let candidates = [
        "ANTHROPIC_API_KEY",
        "OPENAI_API_KEY",
        "GEMINI_API_KEY",
        "AWS_ACCESS_KEY_ID",
        "ELEVENLABS_API_KEY",
    ];
    candidates
        .iter()
        .filter(|k| {
            std::env::var(*k)
                .map(|v| !v.trim().is_empty())
                .unwrap_or(false)
        })
        .copied()
        .collect()
}

/// `argo setup --check` - non-interactive provider-key probe.
fn run_check(require: &[String]) -> ArgoResult<()> {
    if !require.is_empty() {
        return run_check_required(require);
    }
    let detected = detect_provider_keys();
    if !detected.is_empty() {
        println!(
            "[ok] provider key{} loadable from env: {}",
            if detected.len() == 1 { "" } else { "s" },
            detected.join(", ")
        );
        return Ok(());
    }
    eprintln!("[FAIL] no provider API keys detected in the process environment.");
    eprintln!();
    eprintln!("Resolution order (per docs/skycot/provider_keys.md):");
    eprintln!("  1. Process environment: ANTHROPIC_API_KEY / OPENAI_API_KEY / GEMINI_API_KEY / ...");
    eprintln!("  2. Future secrets.toml support is tracked separately.");
    eprintln!();
    eprintln!("Set at least one in your shell rc:");
    eprintln!("  export ANTHROPIC_API_KEY=sk-ant-...");
    eprintln!("  export OPENAI_API_KEY=sk-...");
    eprintln!();
    eprintln!("Then re-run `argo setup --check` to confirm.");
    Err(ArgoError::Provider(
        "no provider API keys detected - see stderr for fix".into(),
    ))
}

/// `argo setup --check --require <var>...` - strict-pair mode.
fn run_check_required(required: &[String]) -> ArgoResult<()> {
    let missing: Vec<&String> = required
        .iter()
        .filter(|var| {
            std::env::var(var.as_str())
                .map(|v| v.trim().is_empty())
                .unwrap_or(true)
        })
        .collect();
    if missing.is_empty() {
        println!(
            "[ok] all required keys loadable from env: {}",
            required.join(", ")
        );
        return Ok(());
    }
    eprintln!("[FAIL] required provider keys missing or empty:");
    for var in &missing {
        eprintln!("  - {var}");
    }
    eprintln!();
    eprintln!("All --require vars must resolve to a non-empty value.");
    eprintln!("See docs/skycot/provider_keys.md for launch-checklist guidance.");
    Err(ArgoError::Provider(format!(
        "{} required key{} missing - see stderr",
        missing.len(),
        if missing.len() == 1 { "" } else { "s" }
    )))
}

#[cfg(test)]
mod tests {
    #[test]
    fn provider_var_set_is_stable() {
        for n in ["ANTHROPIC_API_KEY", "OPENAI_API_KEY", "GEMINI_API_KEY"] {
            assert!(
                include_str!("setup.rs").contains(n),
                "expected setup handler to mention `{n}`"
            );
        }
    }
}
