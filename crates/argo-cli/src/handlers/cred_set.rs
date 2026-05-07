//! `argo cred set`.

use argo_types::{ArgoError, ArgoResult};
use std::io::{self, Write};
use std::path::PathBuf;

pub fn run(key: &str, value: Option<&str>) -> ArgoResult<()> {
    let workspace = workspace_dir()?;
    let value = match value {
        Some(value) => value.to_string(),
        None => read_hidden_value(key)?,
    };
    argo_config::cred::set(&workspace, key, &value)?;
    println!("stored {key}");
    Ok(())
}

fn workspace_dir() -> ArgoResult<PathBuf> {
    std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))
}

fn read_hidden_value(key: &str) -> ArgoResult<String> {
    eprint!("value for {key}: ");
    io::stderr()
        .flush()
        .map_err(|e| ArgoError::Provider(format!("stderr flush: {e}")))?;
    let echo_disabled = disable_echo();
    let mut value = String::new();
    let read_result = io::stdin()
        .read_line(&mut value)
        .map_err(|e| ArgoError::Provider(format!("read credential from stdin: {e}")));
    if echo_disabled {
        let _ = enable_echo();
        eprintln!();
    }
    read_result?;
    Ok(value.trim_end_matches(['\r', '\n']).to_string())
}

#[cfg(unix)]
fn disable_echo() -> bool {
    std::process::Command::new("stty")
        .arg("-echo")
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn disable_echo() -> bool {
    false
}

#[cfg(unix)]
fn enable_echo() -> bool {
    std::process::Command::new("stty")
        .arg("echo")
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn enable_echo() -> bool {
    false
}
