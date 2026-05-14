//! 7-stage onboarding pipeline. Each stage is a function that
//! returns an `OnboardStage` (name + status + detail). Stages
//! 1-2 are mandatory, 3-7 are skippable.
use argo_types::{ArgoError, ArgoResult};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OnboardStatus {
    Ok,
    Skip,
    Fail,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OnboardStage {
    pub name: String,
    pub status: OnboardStatus,
    pub detail: String,
}

impl OnboardStage {
    pub fn ok(name: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: OnboardStatus::Ok,
            detail: detail.into(),
        }
    }
    pub fn skip(name: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: OnboardStatus::Skip,
            detail: detail.into(),
        }
    }
    pub fn fail(name: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: OnboardStatus::Fail,
            detail: detail.into(),
        }
    }
}

/// Driver config for `--non-interactive`. In interactive mode an
/// empty default is used and prompts gather input live.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct OnboardConfig {
    #[serde(default)]
    pub credentials: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    pub connector: ConnectorChoice,
    #[serde(default)]
    pub charter: Option<CharterSpec>,
    #[serde(default)]
    pub action: Option<ActionSpec>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ConnectorChoice {
    #[serde(default)]
    pub authorize: bool,
    #[serde(default)]
    pub kind: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CharterSpec {
    pub text: String,
    pub persona: String,
    pub output: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ActionSpec {
    pub sample: String,
}

impl OnboardConfig {
    pub fn load(path: &Path) -> ArgoResult<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let body = std::fs::read_to_string(path)
            .map_err(|e| ArgoError::Io(format!("read {}: {e}", path.display())))?;
        toml::from_str(&body)
            .map_err(|e| ArgoError::Provider(format!("parse {}: {e}", path.display())))
    }
}

pub async fn run_all(
    cwd: &Path,
    _home: &Path,
    interactive: bool,
    config: &OnboardConfig,
) -> ArgoResult<Vec<OnboardStage>> {
    Ok(vec![
        welcome_stage(),
        workspace_stage(cwd),
        credentials_stage(cwd, &config.credentials, interactive),
        connector_stage(&config.connector, interactive),
        first_worker_stage(cwd, &config.charter, interactive),
        first_action_stage(&config.action, interactive),
        done_stage(),
    ])
}

pub fn welcome_stage() -> OnboardStage {
    let version = env!("CARGO_PKG_VERSION");
    OnboardStage::ok(
        "Welcome",
        format!("argo CLI v{version} — let's get you onboarded"),
    )
}

pub fn workspace_stage(cwd: &Path) -> OnboardStage {
    let toml = cwd.join("argo.toml");
    if toml.exists() {
        return OnboardStage::ok(
            "Workspace",
            format!("{} already initialized", cwd.display()),
        );
    }
    if let Err(e) = bootstrap_workspace(cwd) {
        return OnboardStage::fail("Workspace", format!("bootstrap: {e}"));
    }
    OnboardStage::ok("Workspace", format!("initialized {}", toml.display()))
}

fn bootstrap_workspace(cwd: &Path) -> ArgoResult<()> {
    let toml_path = cwd.join("argo.toml");
    let body = "[core]\nprofile = \"default\"\n\n[tenant]\ndefault_id = \"default\"\nmode = \"single\"\n\n[forge]\nenabled = false\n";
    std::fs::write(&toml_path, body)
        .map_err(|e| ArgoError::Io(format!("write argo.toml: {e}")))?;
    for dir in ["skills", "personas", "cron", "rubrics", ".argo/manifests"] {
        let p = cwd.join(dir);
        std::fs::create_dir_all(&p)
            .map_err(|e| ArgoError::Io(format!("mkdir {}: {e}", p.display())))?;
    }
    Ok(())
}

pub fn credentials_stage(
    cwd: &Path,
    creds: &std::collections::BTreeMap<String, String>,
    interactive: bool,
) -> OnboardStage {
    if creds.is_empty() {
        let detail = if interactive {
            "no credentials provided in this run; set later with `argo cred set <KEY>`"
        } else {
            "no credentials in onboarding.toml; CLI-only mode"
        };
        return OnboardStage::skip("Credentials", detail);
    }
    let mut written = Vec::new();
    for (key, value) in creds {
        if let Err(e) = argo_config::cred::set(cwd, key, value) {
            return OnboardStage::fail("Credentials", format!("set {key}: {e}"));
        }
        written.push(key.clone());
    }
    OnboardStage::ok(
        "Credentials",
        format!("stored {} credential(s) at .argo/credentials.toml", written.len()),
    )
}

pub fn connector_stage(choice: &ConnectorChoice, interactive: bool) -> OnboardStage {
    if !choice.authorize {
        let detail = if interactive {
            "skip — authorize one later with `argo connector authorize <kind>`"
        } else {
            "skip — non-interactive cannot drive OAuth flows"
        };
        return OnboardStage::skip("First connector", detail);
    }
    let kind = choice.kind.as_deref().unwrap_or("gmail");
    OnboardStage::ok(
        "First connector",
        format!("planned: argo connector authorize {kind} (run separately to complete OAuth)"),
    )
}

pub fn first_worker_stage(
    cwd: &Path,
    charter: &Option<CharterSpec>,
    interactive: bool,
) -> OnboardStage {
    let Some(spec) = charter else {
        let detail = if interactive {
            "skip — run `argo genesis interactive` to create your first worker"
        } else {
            "skip — onboarding.toml had no [charter]"
        };
        return OnboardStage::skip("First worker", detail);
    };
    match argo_genesis::plan(&spec.text, &spec.persona) {
        Ok(plan) => {
            let output = PathBuf::from(&spec.output);
            let abs = if output.is_absolute() {
                output
            } else {
                cwd.join(&output)
            };
            if let Err(e) = argo_genesis::write_vertical(&plan, &abs) {
                return OnboardStage::fail("First worker", format!("write_vertical: {e}"));
            }
            OnboardStage::ok(
                "First worker",
                format!("vertical written to {}", abs.display()),
            )
        }
        Err(e) => OnboardStage::fail("First worker", format!("genesis plan: {e}")),
    }
}

pub fn first_action_stage(action: &Option<ActionSpec>, interactive: bool) -> OnboardStage {
    let Some(spec) = action else {
        let detail = if interactive {
            "skip — give your worker a task with `argo worker spawn` then `argo worker action`"
        } else {
            "skip — onboarding.toml had no [action]"
        };
        return OnboardStage::skip("First action", detail);
    };
    OnboardStage::ok(
        "First action",
        format!(
            "sample task recorded: {} (approve via your configured channel)",
            truncate(&spec.sample, 60)
        ),
    )
}

pub fn done_stage() -> OnboardStage {
    OnboardStage::ok(
        "Done",
        "onboarding complete — see `argo doctor` and `docs/skycot/RUNBOOK.md`",
    )
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(n).collect();
        out.push('…');
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn welcome_stage_always_ok() {
        let s = welcome_stage();
        assert_eq!(s.status, OnboardStatus::Ok);
        assert!(s.detail.contains("argo CLI"));
    }

    #[test]
    fn workspace_stage_creates_argo_toml() {
        let cwd = tempfile::tempdir().unwrap();
        let s = workspace_stage(cwd.path());
        assert_eq!(s.status, OnboardStatus::Ok);
        assert!(cwd.path().join("argo.toml").exists());
        assert!(cwd.path().join(".argo/manifests").is_dir());
    }

    #[test]
    fn workspace_stage_idempotent_when_already_initialized() {
        let cwd = tempfile::tempdir().unwrap();
        std::fs::write(cwd.path().join("argo.toml"), "[core]\n").unwrap();
        let s = workspace_stage(cwd.path());
        assert_eq!(s.status, OnboardStatus::Ok);
        assert!(s.detail.contains("already initialized"));
    }

    #[test]
    fn credentials_stage_skips_when_empty() {
        let cwd = tempfile::tempdir().unwrap();
        let creds = std::collections::BTreeMap::new();
        let s = credentials_stage(cwd.path(), &creds, false);
        assert_eq!(s.status, OnboardStatus::Skip);
    }

    #[test]
    fn credentials_stage_writes_credentials_file_at_0600() {
        let cwd = tempfile::tempdir().unwrap();
        let mut creds = std::collections::BTreeMap::new();
        creds.insert("ANTHROPIC_API_KEY".to_string(), "sk-test".to_string());
        let s = credentials_stage(cwd.path(), &creds, false);
        assert_eq!(s.status, OnboardStatus::Ok);
        let path = cwd.path().join(".argo/credentials.toml");
        assert!(path.exists());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600, "credentials.toml must be mode 0600");
        }
    }

    #[test]
    fn connector_stage_skips_when_authorize_false() {
        let s = connector_stage(&ConnectorChoice::default(), false);
        assert_eq!(s.status, OnboardStatus::Skip);
    }

    #[test]
    fn first_worker_stage_skips_when_charter_missing() {
        let cwd = tempfile::tempdir().unwrap();
        let s = first_worker_stage(cwd.path(), &None, false);
        assert_eq!(s.status, OnboardStatus::Skip);
    }

    #[test]
    fn first_worker_stage_writes_vertical_when_charter_present() {
        let cwd = tempfile::tempdir().unwrap();
        let spec = CharterSpec {
            text: "Help draft customer welcome emails".to_string(),
            persona: "Maya".to_string(),
            output: "verticals/maya".to_string(),
        };
        let s = first_worker_stage(cwd.path(), &Some(spec), false);
        assert_eq!(s.status, OnboardStatus::Ok, "detail = {}", s.detail);
        assert!(cwd.path().join("verticals/maya/vertical.toml").exists());
    }

    #[test]
    fn first_action_stage_skips_when_no_sample() {
        let s = first_action_stage(&None, false);
        assert_eq!(s.status, OnboardStatus::Skip);
    }

    #[test]
    fn done_stage_always_ok() {
        let s = done_stage();
        assert_eq!(s.status, OnboardStatus::Ok);
    }

    #[tokio::test]
    async fn run_all_returns_seven_stages() {
        let cwd = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let cfg = OnboardConfig::default();
        let stages = run_all(cwd.path(), home.path(), false, &cfg).await.unwrap();
        assert_eq!(stages.len(), 7);
    }

    #[test]
    fn onboard_config_loads_from_toml() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("onboarding.toml");
        let body = r#"
[credentials]
ANTHROPIC_API_KEY = "sk-test"

[connector]
authorize = false

[charter]
text = "Help with onboarding emails"
persona = "Maya"
output = "verticals/maya"
"#;
        std::fs::write(&path, body).unwrap();
        let cfg = OnboardConfig::load(&path).unwrap();
        assert_eq!(cfg.credentials.get("ANTHROPIC_API_KEY").unwrap(), "sk-test");
        assert_eq!(cfg.charter.unwrap().persona, "Maya");
    }
}
