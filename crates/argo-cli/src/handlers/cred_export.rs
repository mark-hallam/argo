//! `argo cred export`.

use argo_types::{ArgoError, ArgoResult};
use std::path::PathBuf;

pub fn run(shell: &str) -> ArgoResult<()> {
    for line in argo_config::cred::export_lines(&workspace_dir()?, shell)? {
        println!("{line}");
    }
    Ok(())
}

fn workspace_dir() -> ArgoResult<PathBuf> {
    std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))
}
