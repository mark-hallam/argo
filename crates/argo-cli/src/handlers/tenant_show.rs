//! Tenant detail support for `argo tenant show`.

use super::tenant_archive::{open_store, parse_tenant, print_json};
use argo_identity::{ClassificationRules, DataClass};
use argo_store::tenant_query::show_tenant;
use argo_types::{ArgoError, ArgoResult};
use chrono::Utc;
use serde_json::{json, Value};
use std::path::Path;

pub async fn run(id: &str, json_out: bool) -> ArgoResult<()> {
    let tenant = parse_tenant(id)?;
    let (store, cwd) = open_store().await?;
    let mut detail = show_tenant(&store, tenant, Utc::now())
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no tenant '{id}'")))?;
    enrich_detail(&mut detail, &cwd);
    if json_out {
        print_json(&json!({"ok": true, "tenant": detail}))
    } else {
        print_text(&detail);
        Ok(())
    }
}

fn enrich_detail(detail: &mut Value, cwd: &Path) {
    enrich_channel_manifests(detail, cwd);
    detail["classification_rules"] = Value::Array(classification_rules(cwd));
}

fn enrich_channel_manifests(detail: &mut Value, cwd: &Path) {
    let Some(channels) = detail["channels"].as_array_mut() else {
        return;
    };
    for channel in channels {
        let Some(id) = channel["channel"].as_str().map(str::to_string) else {
            continue;
        };
        let path = cwd
            .join("manifests")
            .join("channels")
            .join(format!("{id}.toml"));
        let present = path.exists();
        channel["manifest_present"] = Value::Bool(present);
        channel["manifest_path"] = if present {
            Value::String(path.display().to_string())
        } else {
            Value::Null
        };
    }
}

fn classification_rules(cwd: &Path) -> Vec<Value> {
    let rules = ClassificationRules::from_workspace(cwd);
    DataClass::all()
        .into_iter()
        .map(|class| {
            let rule = rules.rule_for(class);
            json!({
                "class": class.as_str(),
                "retention_days": rule.retention_days,
                "redact_in_export": rule.redact_in_export,
            })
        })
        .collect()
}

fn print_text(detail: &Value) {
    let summary = &detail["summary"];
    println!("tenant {}", summary["id"].as_str().unwrap_or("-"));
    println!("  name:       {}", detail["name"].as_str().unwrap_or("-"));
    println!("  owner:      {}", detail["owner_email"].as_str().unwrap_or("-"));
    println!("  root:       {}", detail["root_path"].as_str().unwrap_or("-"));
    println!("  workers:    {}", summary["worker_count"].as_i64().unwrap_or_default());
    println!("  sessions:   {}", detail["session_count"].as_i64().unwrap_or_default());
    println!("  archived:   {}", summary["archived_at_rfc3339"].as_str().unwrap_or("-"));
    println!("  spend day:  {}", summary["spend_today"].as_str().unwrap_or("$0.0000"));
    println!("  spend mo.:  {}", summary["spend_this_month"].as_str().unwrap_or("$0.0000"));
    print_channels(detail);
    print_activity(detail);
}

fn print_channels(detail: &Value) {
    let Some(channels) = detail["channels"].as_array() else {
        return;
    };
    if channels.is_empty() {
        println!("  channels:   -");
        return;
    }
    println!("  channels:");
    for channel in channels {
        println!(
            "    {:<16} workers={} rosters={} manifest={}",
            channel["channel"].as_str().unwrap_or("-"),
            channel["worker_refs"].as_i64().unwrap_or_default(),
            channel["roster_refs"].as_i64().unwrap_or_default(),
            channel["manifest_present"].as_bool().unwrap_or(false)
        );
    }
}

fn print_activity(detail: &Value) {
    let Some(rows) = detail["recent_activity"].as_array() else {
        return;
    };
    if rows.is_empty() {
        println!("  activity:   -");
        return;
    }
    println!("  activity:");
    for row in rows {
        println!(
            "    {:<18} {}",
            row["kind"].as_str().unwrap_or("-"),
            row["created_at_rfc3339"].as_str().unwrap_or("-")
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn classification_rules_include_defaults() {
        let dir = tempdir().unwrap();
        let rows = classification_rules(dir.path());
        assert!(rows.iter().any(|row| row["class"] == "pii"));
        assert!(rows.iter().any(|row| row["class"] == "untagged"));
    }

    #[test]
    fn enrich_channel_manifests_sets_presence() {
        let dir = tempdir().unwrap();
        let manifest = dir.path().join("manifests/channels/email.toml");
        std::fs::create_dir_all(manifest.parent().unwrap()).unwrap();
        std::fs::write(&manifest, "[channel]\nid = \"email\"\n").unwrap();
        let mut detail = json!({"channels":[{"channel":"email","manifest_present":false}]});

        enrich_channel_manifests(&mut detail, dir.path());

        assert_eq!(detail["channels"][0]["manifest_present"], true);
        assert!(detail["channels"][0]["manifest_path"].as_str().unwrap().ends_with("email.toml"));
    }
}
