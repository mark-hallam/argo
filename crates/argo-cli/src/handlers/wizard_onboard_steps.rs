use super::wizard_onboard::{
    is_plausible_email, is_valid_worker_name, provider_err, Answers, KeyVerifier, PromptIo,
    DEFAULT_VERTICAL,
};
use argo_types::ArgoResult;
use std::path::Path;

pub fn existing_operator_block(config_path: &Path) -> ArgoResult<bool> {
    if !config_path.exists() {
        return Ok(false);
    }
    let body = std::fs::read_to_string(config_path)
        .map_err(|e| provider_err(format!("read {}: {e}", config_path.display())))?;
    let doc: toml::Value = body
        .parse()
        .map_err(|e| provider_err(format!("parse {}: {e}", config_path.display())))?;
    Ok(doc.get("operator").and_then(|v| v.as_table()).is_some())
}

pub(crate) fn step_detect<P: PromptIo>(io: &mut P) -> ArgoResult<()> {
    io.write_line("")?;
    io.write_line("step 1/5  detect — platform + workspace")?;
    io.write_line(&format!(
        "  ✓ platform: {}/{}",
        std::env::consts::OS,
        std::env::consts::ARCH
    ))?;
    io.write_line(&format!(
        "  ✓ argo CLI v{} (run `argo doctor` for the full health check)",
        env!("CARGO_PKG_VERSION")
    ))
}

pub(crate) fn step_provider_key<P: PromptIo, V: KeyVerifier>(
    io: &mut P,
    verifier: &V,
    answers: &Answers,
    interactive: bool,
) -> ArgoResult<String> {
    io.write_line("")?;
    io.write_line("step 2/5  provider — ANTHROPIC_API_KEY")?;
    let key = collect_required(
        io,
        interactive,
        answers.provider_api_key.as_deref(),
        "Anthropic API key (sk-ant-…)",
        "provider_api_key",
    )?;
    if answers.skip_key_test {
        io.write_line("  • skipping live key test (skip_key_test=true)")?;
        return Ok(key);
    }
    io.write_line("  • running 1-token verify call to api.anthropic.com (~$0.001)…")?;
    verifier
        .verify(&key)
        .map_err(|r| provider_err(format!("wizard onboard: provider key rejected: {r}")))?;
    io.write_line("  ✓ key accepted by Anthropic")?;
    Ok(key)
}

pub(crate) fn step_operator<P: PromptIo>(
    io: &mut P,
    answers: &Answers,
    interactive: bool,
) -> ArgoResult<(String, String)> {
    io.write_line("")?;
    io.write_line("step 3/5  operator identity")?;
    let name = collect_required(
        io,
        interactive,
        answers.operator_name.as_deref(),
        "operator name",
        "operator_name",
    )?;
    let email = loop {
        let c = collect_required(
            io,
            interactive,
            answers.operator_email.as_deref(),
            "operator email",
            "operator_email",
        )?;
        if is_plausible_email(&c) {
            break c;
        }
        if !interactive {
            return Err(provider_err(format!(
                "wizard onboard: '{c}' is not a plausible email (need user@domain.tld)"
            )));
        }
        io.write_line("    (need user@domain.tld — try again)")?;
    };
    Ok((name, email))
}

pub(crate) fn step_vertical<P: PromptIo>(
    io: &mut P,
    answers: &Answers,
    cwd: &Path,
    interactive: bool,
) -> ArgoResult<String> {
    io.write_line("")?;
    io.write_line("step 4/5  first vertical")?;
    let suggested = answers
        .vertical
        .clone()
        .unwrap_or_else(|| DEFAULT_VERTICAL.to_string());
    let chosen = if interactive {
        match io.read_line(&format!("vertical (default {suggested})"))? {
            Some(s) if !s.is_empty() => s,
            _ => suggested,
        }
    } else {
        suggested
    };
    validate_vertical_exists(&chosen, cwd)?;
    io.write_line(&format!("  ✓ vertical {chosen} resolved"))?;
    Ok(chosen)
}

pub(crate) fn step_worker<P: PromptIo>(
    io: &mut P,
    answers: &Answers,
    interactive: bool,
) -> ArgoResult<(String, String)> {
    io.write_line("")?;
    io.write_line("step 5/5  first worker")?;
    let name = loop {
        let c = collect_required(
            io,
            interactive,
            answers.worker_name.as_deref(),
            "worker name (lowercase, 3-31 chars)",
            "worker_name",
        )?;
        if is_valid_worker_name(&c) {
            break c;
        }
        if !interactive {
            return Err(provider_err(format!(
                "wizard onboard: worker name '{c}' fails /^[a-z][a-z0-9_-]{{2,30}}$/"
            )));
        }
        io.write_line("    (must match /^[a-z][a-z0-9_-]{2,30}$/)")?;
    };
    let persona = collect_required(
        io,
        interactive,
        answers.worker_persona.as_deref(),
        "persona summary (one line)",
        "worker_persona",
    )?;
    Ok((name, persona))
}

fn collect_required<P: PromptIo>(
    io: &mut P,
    interactive: bool,
    answer: Option<&str>,
    prompt: &str,
    field: &str,
) -> ArgoResult<String> {
    if let Some(s) = answer {
        let v = s.trim().to_string();
        if v.is_empty() {
            return Err(provider_err(format!(
                "wizard onboard: answers.{field} is empty"
            )));
        }
        return Ok(v);
    }
    if !interactive {
        return Err(provider_err(format!(
            "wizard onboard: --non-interactive requires answers.{field}"
        )));
    }
    loop {
        let c = io.read_line(prompt)?.unwrap_or_default();
        if !c.is_empty() {
            return Ok(c);
        }
        io.write_line("    (required — please answer)")?;
    }
}

fn validate_vertical_exists(id: &str, cwd: &Path) -> ArgoResult<()> {
    if cwd.join("verticals").join(id).is_dir() {
        return Ok(());
    }
    let (registry, _errors) = argo_vertical::load_dir(&argo_vertical::default_dir())
        .map_err(|e| provider_err(format!("wizard onboard: vertical load: {e}")))?;
    if registry.get(id).is_some() {
        return Ok(());
    }
    Err(provider_err(format!(
        "wizard onboard: vertical '{id}' not found — run `argo vertical list` to see what's available"
    )))
}
