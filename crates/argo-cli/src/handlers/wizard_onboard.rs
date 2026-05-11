//! `argo wizard onboard` — end-to-end first-operator setup
//! (Mission `mission-operator-onboarding-wizard`). Five steps:
//! detect → provider key → operator identity → first vertical →
//! first worker. Replaces the legacy `setup` + hand-edit + `new`
//! sequence. Telemetry-free: only an explicit 1-token Anthropic
//! call (skippable for airgapped operators via the answers file).
use argo_types::{ArgoError, ArgoResult};
use serde::Deserialize;
use std::io::{BufRead, IsTerminal, Write};
use std::path::{Path, PathBuf};

pub const DEFAULT_VERTICAL: &str = "marketplace-seller-browser";

fn provider_err(s: impl Into<String>) -> ArgoError {
    ArgoError::Provider(s.into())
}

pub fn is_valid_worker_name(s: &str) -> bool {
    if !(3..=31).contains(&s.len()) {
        return false;
    }
    let mut it = s.chars();
    if !matches!(it.next(), Some(c) if c.is_ascii_lowercase()) {
        return false;
    }
    it.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

pub fn is_plausible_email(s: &str) -> bool {
    let parts: Vec<&str> = s.split('@').collect();
    if parts.len() != 2 {
        return false;
    }
    let (l, d) = (parts[0], parts[1]);
    !l.is_empty() && d.contains('.') && !d.starts_with('.') && !d.ends_with('.')
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Answers {
    pub provider_api_key: Option<String>,
    pub operator_name: Option<String>,
    pub operator_email: Option<String>,
    pub vertical: Option<String>,
    pub worker_name: Option<String>,
    pub worker_persona: Option<String>,
    #[serde(default)]
    pub skip_key_test: bool,
    #[serde(default)]
    pub overwrite_existing: bool,
}

impl Answers {
    pub fn load(path: &Path) -> ArgoResult<Self> {
        let body = std::fs::read_to_string(path)
            .map_err(|e| provider_err(format!("read {}: {e}", path.display())))?;
        toml::from_str(&body).map_err(|e| provider_err(format!("parse {}: {e}", path.display())))
    }
}

pub trait PromptIo {
    fn read_line(&mut self, prompt: &str) -> ArgoResult<Option<String>>;
    fn write_line(&mut self, line: &str) -> ArgoResult<()>;
}

pub struct StdinPrompt<R: BufRead, W: Write> {
    reader: R,
    writer: W,
}

impl<R: BufRead, W: Write> StdinPrompt<R, W> {
    pub fn new(reader: R, writer: W) -> Self {
        Self { reader, writer }
    }
}

impl<R: BufRead, W: Write> PromptIo for StdinPrompt<R, W> {
    fn read_line(&mut self, prompt: &str) -> ArgoResult<Option<String>> {
        write!(self.writer, "{prompt} > ").map_err(|e| provider_err(format!("wizard: {e}")))?;
        self.writer
            .flush()
            .map_err(|e| provider_err(format!("wizard flush: {e}")))?;
        let mut buf = String::new();
        let n = self
            .reader
            .read_line(&mut buf)
            .map_err(|e| provider_err(format!("wizard read: {e}")))?;
        if n == 0 {
            return Ok(None);
        }
        while matches!(buf.chars().last(), Some('\n') | Some('\r')) {
            buf.pop();
        }
        Ok(Some(buf.trim().to_string()))
    }
    fn write_line(&mut self, line: &str) -> ArgoResult<()> {
        writeln!(self.writer, "{line}").map_err(|e| provider_err(format!("wizard write: {e}")))
    }
}

pub trait KeyVerifier {
    fn verify(&self, key: &str) -> Result<(), String>;
}

pub struct HttpKeyVerifier;

impl KeyVerifier for HttpKeyVerifier {
    fn verify(&self, key: &str) -> Result<(), String> {
        if key.trim().is_empty() {
            return Err("empty key".into());
        }
        let body = serde_json::json!({
            "model": "claude-haiku-4-5-20251001",
            "max_tokens": 1,
            "messages": [{"role": "user", "content": "ping"}],
        });
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| format!("http client: {e}"))?;
        let resp = client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .map_err(|e| format!("send: {e}"))?;
        if resp.status().is_success() {
            return Ok(());
        }
        let status = resp.status();
        let txt: String = resp.text().unwrap_or_default().chars().take(200).collect();
        Err(format!("http {status}: {txt}"))
    }
}

#[derive(Debug, Clone)]
pub struct StepOutcome {
    #[allow(dead_code)] // surfaced for test assertions only
    pub operator_name: String,
    #[allow(dead_code)] // surfaced for test assertions only
    pub operator_email: String,
    pub api_key: String,
    pub vertical: String,
    pub worker_name: String,
    pub persona: String,
}

pub fn run_steps<P: PromptIo, V: KeyVerifier>(
    io: &mut P,
    verifier: &V,
    cwd: &Path,
    config_path: &Path,
    answers: &Answers,
    interactive: bool,
) -> ArgoResult<Option<StepOutcome>> {
    io.write_line(&format!(
        "argo wizard onboard v{} — five steps to your first worker.",
        env!("CARGO_PKG_VERSION")
    ))?;
    if existing_operator_block(config_path)? {
        io.write_line(&format!(
            "{} already has an [operator] block.",
            config_path.display()
        ))?;
        let yes = if interactive {
            let ans = io
                .read_line("overwrite existing operator block? [y/N]")?
                .unwrap_or_default()
                .to_lowercase();
            matches!(ans.as_str(), "y" | "yes")
        } else {
            answers.overwrite_existing
        };
        if !yes {
            io.write_line("aborted — existing [operator] block preserved.")?;
            return Ok(None);
        }
    }
    step_detect(io)?;
    let api_key = step_provider_key(io, verifier, answers, interactive)?;
    let (operator_name, operator_email) = step_operator(io, answers, interactive)?;
    write_operator_block(config_path, &operator_name, &operator_email)?;
    let vertical = step_vertical(io, answers, cwd, interactive)?;
    let (worker_name, persona) = step_worker(io, answers, interactive)?;
    io.write_line("")?;
    io.write_line("✓ all five steps complete.")?;
    io.write_line(&format!("  • argo.toml: {}", config_path.display()))?;
    io.write_line(&format!("  • operator:  {operator_name} <{operator_email}>"))?;
    io.write_line(&format!("  • vertical:  {vertical}"))?;
    io.write_line(&format!("  • worker:    {worker_name} — {persona}"))?;
    io.write_line("")?;
    io.write_line("next steps:")?;
    io.write_line(&format!("  argo chat {worker_name}        — open a chat session"))?;
    io.write_line("  argo handoff list             — see active handoffs")?;
    io.write_line("  argo doctor                   — workspace health check")?;
    Ok(Some(StepOutcome {
        operator_name,
        operator_email,
        api_key,
        vertical,
        worker_name,
        persona,
    }))
}

fn existing_operator_block(config_path: &Path) -> ArgoResult<bool> {
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

fn step_detect<P: PromptIo>(io: &mut P) -> ArgoResult<()> {
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

fn step_provider_key<P: PromptIo, V: KeyVerifier>(
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

fn step_operator<P: PromptIo>(
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

fn step_vertical<P: PromptIo>(
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

fn step_worker<P: PromptIo>(
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

pub fn write_operator_block(config_path: &Path, name: &str, email: &str) -> ArgoResult<()> {
    let mut doc = load_argo_toml_table(config_path)?;
    let mut op = toml::value::Table::new();
    op.insert("name".into(), toml::Value::String(name.into()));
    op.insert("email".into(), toml::Value::String(email.into()));
    op.insert("provider".into(), toml::Value::String("anthropic".into()));
    doc.insert("operator".into(), toml::Value::Table(op));
    if let Some(parent) = config_path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)
            .map_err(|e| provider_err(format!("mkdir {}: {e}", parent.display())))?;
    }
    let body = toml::to_string_pretty(&toml::Value::Table(doc))
        .map_err(|e| provider_err(format!("serialize: {e}")))?;
    std::fs::write(config_path, body)
        .map_err(|e| provider_err(format!("write {}: {e}", config_path.display())))
}

fn load_argo_toml_table(config_path: &Path) -> ArgoResult<toml::value::Table> {
    if !config_path.exists() {
        let mut t = toml::value::Table::new();
        let mut core = toml::value::Table::new();
        core.insert("profile".into(), toml::Value::String("default".into()));
        t.insert("core".into(), toml::Value::Table(core));
        return Ok(t);
    }
    let body = std::fs::read_to_string(config_path)
        .map_err(|e| provider_err(format!("read {}: {e}", config_path.display())))?;
    Ok(body
        .parse::<toml::Value>()
        .map_err(|e| provider_err(format!("parse {}: {e}", config_path.display())))?
        .as_table()
        .cloned()
        .unwrap_or_default())
}

pub fn scaffold_first_worker(
    cwd: &Path,
    worker_name: &str,
    persona: &str,
    vertical: &str,
) -> ArgoResult<PathBuf> {
    let dir = cwd.join("workers").join(worker_name);
    std::fs::create_dir_all(&dir)
        .map_err(|e| provider_err(format!("mkdir {}: {e}", dir.display())))?;
    let body = format!(
        "[worker]\nname = \"{worker_name}\"\nvertical = \"{vertical}\"\npersona = \"\"\"\n{persona}\n\"\"\"\n"
    );
    let path = dir.join("worker.toml");
    std::fs::write(&path, body)
        .map_err(|e| provider_err(format!("write {}: {e}", path.display())))?;
    Ok(path)
}

pub async fn run(
    non_interactive: bool,
    config: Option<String>,
    answers_path: Option<String>,
) -> ArgoResult<()> {
    let cwd = std::env::current_dir().map_err(|e| provider_err(format!("wizard: cwd: {e}")))?;
    let config_path = config.map_or_else(|| cwd.join("argo.toml"), PathBuf::from);
    let answers = match (non_interactive, answers_path.as_deref()) {
        (true, Some(p)) => Answers::load(Path::new(p))?,
        (true, None) => {
            return Err(provider_err(
                "wizard onboard: --non-interactive requires --answers <path>",
            ));
        }
        (false, Some(p)) => Answers::load(Path::new(p))?,
        (false, None) => Answers::default(),
    };
    let interactive = !non_interactive && std::io::stdin().is_terminal();
    let stdin = std::io::stdin();
    let mut io = StdinPrompt::new(stdin.lock(), std::io::stdout());
    let verifier = HttpKeyVerifier;
    if let Some(outcome) = run_steps(&mut io, &verifier, &cwd, &config_path, &answers, interactive)?
    {
        argo_config::cred::set(&cwd, "ANTHROPIC_API_KEY", &outcome.api_key)?;
        scaffold_first_worker(&cwd, &outcome.worker_name, &outcome.persona, &outcome.vertical)?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "wizard_onboard_tests.rs"]
mod tests;
