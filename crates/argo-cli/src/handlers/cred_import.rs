//! `argo cred import`.

use argo_types::{ArgoError, ArgoResult};
use std::path::PathBuf;

pub fn run(shell: &str) -> ArgoResult<()> {
    validate_shell(shell)?;
    let count = argo_config::cred::import_current_env(&workspace_dir()?)?;
    println!("imported {count} credential(s)");
    Ok(())
}

fn validate_shell(shell: &str) -> ArgoResult<()> {
    match shell {
        "bash" | "zsh" => Ok(()),
        other => Err(ArgoError::Provider(format!(
            "unsupported shell '{other}': expected bash or zsh"
        ))),
    }
}

fn workspace_dir() -> ArgoResult<PathBuf> {
    std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))
}

#[cfg(test)]
mod tests {
    use super::validate_shell;

    #[test]
    fn import_accepts_bash_and_zsh_only() {
        assert!(validate_shell("bash").is_ok());
        assert!(validate_shell("zsh").is_ok());
        assert!(validate_shell("fish").is_err());
    }
}
