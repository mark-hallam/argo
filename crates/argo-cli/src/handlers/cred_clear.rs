//! `argo cred clear` and `argo cred clear-all`.

use argo_types::{ArgoError, ArgoResult};
use std::path::PathBuf;

pub fn clear(key: &str) -> ArgoResult<()> {
    let removed = argo_config::cred::clear(&workspace_dir()?, key)?;
    if removed {
        println!("cleared {key}");
    } else {
        println!("{key} was not set");
    }
    Ok(())
}

pub fn clear_all() -> ArgoResult<()> {
    let count = argo_config::cred::clear_all(&workspace_dir()?)?;
    println!("cleared {count} credential(s)");
    Ok(())
}

fn workspace_dir() -> ArgoResult<PathBuf> {
    std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))
}
