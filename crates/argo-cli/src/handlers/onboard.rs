//! `argo onboard` — first-experience walkthrough for non-developer
//! operators. 7 stages run end-to-end: welcome → workspace →
//! credentials → first connector → first worker → first action →
//! done. Stages 3-7 may Skip without aborting; stages 1-2 are
//! mandatory.
//!
//! Interactive mode (TTY + no `--non-interactive`) prompts the
//! operator at each stage. `--non-interactive` mode reads
//! `<workspace>/.argo/onboarding.toml` instead — designed for CI /
//! scripted bootstrap.
use crate::home::{ensure_dir, resolve_home};
use argo_types::{ArgoError, ArgoResult};
use std::io::IsTerminal;
use std::path::PathBuf;

pub async fn run(non_interactive: bool, json: bool, config: Option<String>) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let interactive = !non_interactive && std::io::stdin().is_terminal();
    let config = if non_interactive {
        let path = config
            .map(PathBuf::from)
            .unwrap_or_else(|| cwd.join(".argo/onboarding.toml"));
        super::onboard_stages::OnboardConfig::load(&path)?
    } else {
        super::onboard_stages::OnboardConfig::default()
    };
    let stages = super::onboard_stages::run_all(&cwd, &home, interactive, &config).await?;
    super::onboard_render::print(&stages, json)
}
