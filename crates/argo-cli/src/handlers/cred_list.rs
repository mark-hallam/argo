//! `argo cred list`.

use argo_types::{ArgoError, ArgoResult};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::PathBuf;

pub fn run(reveal: bool, json: bool) -> ArgoResult<()> {
    let credentials = argo_config::cred::load(&workspace_dir()?)?;
    if json {
        return print_json(&credentials, reveal);
    }
    for (key, value) in credentials {
        if reveal {
            println!("{key}={value}");
        } else {
            println!("{key}={}", mask(&value));
        }
    }
    Ok(())
}

#[derive(Debug, Serialize)]
struct CredentialListEntry {
    key: String,
    value: String,
}

#[derive(Debug, Serialize)]
struct CredentialListPayload {
    ok: bool,
    count: usize,
    credentials: Vec<CredentialListEntry>,
}

fn print_json(credentials: &BTreeMap<String, String>, reveal: bool) -> ArgoResult<()> {
    let payload = credential_payload(credentials, reveal);
    let body = serde_json::to_string_pretty(&payload)
        .map_err(|e| ArgoError::Provider(format!("serialize credential list: {e}")))?;
    println!("{body}");
    Ok(())
}

fn credential_payload(
    credentials: &BTreeMap<String, String>,
    reveal: bool,
) -> CredentialListPayload {
    let entries = credentials
        .iter()
        .map(|(key, value)| CredentialListEntry {
            key: key.clone(),
            value: if reveal { value.clone() } else { mask(value) },
        })
        .collect::<Vec<_>>();
    CredentialListPayload {
        ok: true,
        count: entries.len(),
        credentials: entries,
    }
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
    use super::{credential_payload, mask};
    use std::collections::BTreeMap;

    #[test]
    fn mask_hides_non_empty_values() {
        assert_eq!(mask("secret"), "********");
    }

    #[test]
    fn json_payload_masks_values_by_default() {
        let credentials = BTreeMap::from([
            ("ANTHROPIC_API_KEY".to_string(), "sk-secret".to_string()),
            ("EMPTY".to_string(), String::new()),
        ]);
        let payload = credential_payload(&credentials, false);
        let body = serde_json::to_value(payload).expect("serialize payload");
        assert_eq!(body["ok"], true);
        assert_eq!(body["count"], 2);
        assert_eq!(body["credentials"][0]["key"], "ANTHROPIC_API_KEY");
        assert_eq!(body["credentials"][0]["value"], "********");
        assert_eq!(body["credentials"][1]["value"], "<empty>");
    }

    #[test]
    fn json_payload_reveals_values_when_requested() {
        let credentials = BTreeMap::from([("OPENAI_API_KEY".to_string(), "sk-real".to_string())]);
        let payload = credential_payload(&credentials, true);
        let body = serde_json::to_value(payload).expect("serialize payload");
        assert_eq!(body["credentials"][0]["value"], "sk-real");
    }
}
