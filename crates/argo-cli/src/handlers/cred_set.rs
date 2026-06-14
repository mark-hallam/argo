//! `argo cred set`.

use argo_types::{ArgoError, ArgoResult};
use std::path::PathBuf;

pub fn run(key: &str, value: Option<&str>) -> ArgoResult<()> {
    let workspace = workspace_dir()?;
    let value = match value {
        Some(value) => value.to_string(),
        // Echo-suppressed read shared with `argo connector authorize` — masks the
        // secret on the console on unix AND Windows (win-cli-console-echo-suppression-secrets).
        None => super::secret_input::read_hidden_value(key)?,
    };
    argo_config::cred::set(&workspace, key, &value)?;
    println!("stored {key}");
    Ok(())
}

fn workspace_dir() -> ArgoResult<PathBuf> {
    std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))
}
