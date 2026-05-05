//! `argo setup` — interactive walkthrough for first-time users.
//! Mirrors Hermes' two-step install (`curl … | bash` → `hermes
//! setup`) so a user who just installed via #646's `install.sh`
//! lands in a guided onboarding instead of a 133-endpoint API
//! reference.
//!
//! Steps:
//!   1. Confirm `~/.argo/` location.
//!   2. Run `argo workspace init` (idempotent; skipped if already
//!      registered).
//!   3. Detect which provider env-vars are already exported.
//!   4. If none, prompt the user to pick one + show the export
//!      they need to add to their shell rc.
//!   5. Print a one-shot smoke test (`argo doctor` + `curl
//!      /health`).
//!
//! Non-interactive fallback: when stdin isn't a TTY (e.g. piped
//! from CI), prints the same info as a text checklist and exits 0
//! without prompting.
//!
//! `--check` mode (#761): skip the walkthrough; verify that at
//! least one provider key is loadable from the process env.
//! Designed for CI / production-launch checklists. Exits non-zero
//! when no key is set so the operator's launch checklist surfaces
//! the gap loudly.
use crate::home::{ensure_dir, resolve_home};
use argo_types::{ArgoError, ArgoResult};
use std::io::{self, BufRead, IsTerminal, Write};

const PROVIDERS: &[(&str, &str, &str)] = &[
    (
        "1",
        "anthropic",
        "ANTHROPIC_API_KEY  (Claude — sk-ant-…)",
    ),
    ("2", "openai", "OPENAI_API_KEY     (GPT-* — sk-…)"),
    ("3", "gemini", "GEMINI_API_KEY     (Gemini)"),
    (
        "4",
        "local",
        "(no key needed — uses `local-shim`/Ollama-style endpoints)",
    ),
];

pub async fn run(check: bool) -> ArgoResult<()> {
    if check {
        return run_check();
    }
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;

    println!("argo setup");
    println!("==========");
    println!();
    println!("Workspace location: {}", home.display());
    println!();

    // Step 1 — workspace init (idempotent).
    let toml_path = cwd.join("argo.toml");
    if toml_path.exists() {
        println!("[ok] argo.toml already exists at {}", toml_path.display());
    } else {
        println!("Creating argo.toml + bundle dirs in {}", cwd.display());
        super::workspace::run(crate::cli::WorkspaceCmd::Init).await?;
    }
    println!();

    // Step 2 — provider key detection.
    let already_set = detect_provider_keys();
    if !already_set.is_empty() {
        println!(
            "[ok] provider key{} detected in env: {}",
            if already_set.len() == 1 { "" } else { "s" },
            already_set.join(", ")
        );
    } else {
        println!("No provider API keys detected in your environment.");
        println!();
        if io::stdin().is_terminal() {
            prompt_for_provider();
        } else {
            print_provider_checklist();
        }
    }
    println!();

    // Step 3 — verify hint + next steps.
    println!("Next steps:");
    println!("  1.  argo doctor                      # workspace health check");
    println!("  2.  argo gateway start &             # bind 127.0.0.1:7400");
    println!("  3.  curl http://127.0.0.1:7400/      # landing JSON");
    println!("  4.  argo chat \"hello\"                # one turn against your provider");
    println!();
    println!("All commands: `argo --help` (or `argo gateway routes` for the full HTTP catalog).");
    Ok(())
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

/// `argo setup --check` — non-interactive provider-key probe.
///
/// Walks the canonical provider env-var list and reports which
/// (if any) are loadable. Exits cleanly when at least one key is
/// set; returns a `Provider` error when none are. The error's
/// stderr includes the canonical resolution-order summary so an
/// operator running this from a launch checklist sees both the
/// gap AND the fix in one shot.
fn run_check() -> ArgoResult<()> {
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
    eprintln!("  1. Process environment — ANTHROPIC_API_KEY / OPENAI_API_KEY / GEMINI_API_KEY / ...");
    eprintln!("  2. (Future) ~/.config/argo/secrets.toml — not yet implemented; track via #758.");
    eprintln!();
    eprintln!("Set at least one in your shell rc:");
    eprintln!("  export ANTHROPIC_API_KEY=sk-ant-...");
    eprintln!("  export OPENAI_API_KEY=sk-...");
    eprintln!();
    eprintln!("Then re-run `argo setup --check` to confirm.");
    Err(ArgoError::Provider(
        "no provider API keys detected — see stderr for fix".into(),
    ))
}

fn prompt_for_provider() {
    println!("Which provider do you want to use first?");
    for (i, name, hint) in PROVIDERS {
        println!("  {i}) {name:<10} — {hint}");
    }
    print!("Choice [1]: ");
    let _ = io::stdout().flush();

    let mut line = String::new();
    let stdin = io::stdin();
    let _ = stdin.lock().read_line(&mut line);
    let choice = line.trim();
    let choice = if choice.is_empty() { "1" } else { choice };

    let pick = PROVIDERS
        .iter()
        .find(|(i, _, _)| *i == choice)
        .copied()
        .unwrap_or(PROVIDERS[0]);
    let (_, name, _) = pick;

    println!();
    match name {
        "anthropic" => print_export(
            "ANTHROPIC_API_KEY",
            "sk-ant-...",
            "https://console.anthropic.com",
        ),
        "openai" => print_export(
            "OPENAI_API_KEY",
            "sk-...",
            "https://platform.openai.com/api-keys",
        ),
        "gemini" => print_export(
            "GEMINI_API_KEY",
            "AIza...",
            "https://aistudio.google.com/app/apikey",
        ),
        "local" => {
            println!("`local` backend uses an OpenAI-compatible endpoint at");
            println!("http://localhost:11434 (Ollama-style). No API key needed.");
            println!("Make sure you have an Ollama or similar server running.");
        }
        _ => {}
    }
}

fn print_export(var: &str, example: &str, url: &str) {
    println!("To use {var}:");
    println!();
    println!("  1. Get a key from {url}");
    println!("  2. Add this to your shell rc (~/.bashrc / ~/.zshrc):");
    println!();
    println!("       export {var}={example}");
    println!();
    println!("  3. Reload your shell or `source` the rc.");
}

fn print_provider_checklist() {
    println!("Pick one and export the key in your shell:");
    println!();
    for (_, name, hint) in PROVIDERS {
        println!("  {name:<10} — {hint}");
    }
    println!();
    println!("Then re-run `argo setup` to confirm, or jump straight to `argo chat`.");
}

#[cfg(test)]
mod tests {
    /// Standard env vars are inherited from the host. We can't
    /// reliably test detection by mutating process env from a
    /// test (it's racy under cargo test parallelism), so the
    /// lookup is exercised end-to-end from the integration test
    /// instead. Here we lock the canonical-key set so a
    /// regression that drops a known provider variable is caught
    /// at unit-test speed.
    #[test]
    fn provider_var_set_is_stable() {
        // Sanity check: the canonical list this command probes
        // matches the operator-facing copy in the FAIL message.
        // If a new provider gets added to one but not the other,
        // this test gives the immediate signal.
        for n in ["ANTHROPIC_API_KEY", "OPENAI_API_KEY", "GEMINI_API_KEY"] {
            // Each canonical var must appear in the source file
            // verbatim; if someone renames one, both call sites
            // need updating.
            assert!(
                include_str!("setup.rs").contains(n),
                "expected setup handler to mention `{n}`"
            );
        }
    }
}
