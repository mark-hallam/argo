//! `argo cred list`.

use argo_types::{ArgoError, ArgoResult};
use std::path::PathBuf;

pub fn run(reveal: bool) -> ArgoResult<()> {
    let credentials = argo_config::cred::load(&workspace_dir()?)?;
    for (key, value) in credentials {
        if reveal {
            println!("{key}={value}");
        } else {
            println!("{key}={}", mask(&value));
        }
    }
    Ok(())
}

fn workspace_dir() -> ArgoResult<PathBuf> {
    std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))
}

fn mask(value: &str) -> String {
    if value.is_empty() {
        "<empty>".to_string()
    } else {
        "********".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::mask;

    #[test]
    fn mask_hides_non_empty_values() {
        assert_eq!(mask("secret"), "********");
    }
}
