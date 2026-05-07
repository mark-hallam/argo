//! `argo connector list`.

use super::connector_authorize::{provider_err, workspace_dir, AuthKind, CONNECTORS};
use argo_types::ArgoResult;
use serde::Serialize;
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use toml::Value;

#[derive(Clone, Debug, Serialize)]
pub(crate) struct ConnectorInfo {
    pub kind: String,
    pub auth_type: String,
    pub installed: bool,
    pub methods: usize,
    pub method_names: Vec<String>,
    pub required_credentials: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manifest_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manifest: Option<String>,
}

#[derive(Clone, Debug)]
struct ManifestMeta {
    path: PathBuf,
    body: String,
    methods: Vec<String>,
    required: Vec<String>,
    auth_type: Option<String>,
}

pub async fn run(installed_only: bool, json_out: bool) -> ArgoResult<()> {
    let workspace = workspace_dir()?;
    let rows = inventory(&workspace, installed_only)?;
    if json_out {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "ok": true,
                "connector_list": {"connectors": rows}
            }))
            .map_err(|e| provider_err(format!("serialize JSON: {e}")))?
        );
    } else {
        print_table(&rows);
    }
    Ok(())
}

pub(crate) fn inventory(workspace: &Path, installed_only: bool) -> ArgoResult<Vec<ConnectorInfo>> {
    let manifests = load_manifests(workspace)?;
    let mut kinds = known_kinds(workspace);
    kinds.extend(manifests.keys().cloned());
    let mut out = Vec::new();
    for kind in kinds {
        let manifest = manifests.get(&kind);
        if installed_only && manifest.is_none() {
            continue;
        }
        out.push(info_for(workspace, &kind, manifest));
    }
    Ok(out)
}

pub(crate) fn find_connector(workspace: &Path, kind: &str) -> ArgoResult<ConnectorInfo> {
    inventory(workspace, false)?
        .into_iter()
        .find(|row| row.kind == kind)
        .ok_or_else(|| provider_err(format!("unknown connector '{kind}'")))
}

fn info_for(workspace: &Path, kind: &str, manifest: Option<&ManifestMeta>) -> ConnectorInfo {
    let manifest_methods = manifest.map(|m| m.methods.clone()).unwrap_or_default();
    let method_names = if manifest_methods.is_empty() {
        crate_methods(workspace, kind)
    } else {
        manifest_methods
    };
    let mut required = spec_credentials(kind);
    if let Some(meta) = manifest {
        required.extend(meta.required.iter().cloned());
    }
    required.sort();
    required.dedup();
    ConnectorInfo {
        kind: kind.to_string(),
        auth_type: auth_type(kind, manifest),
        installed: manifest.is_some(),
        methods: method_names.len(),
        method_names,
        required_credentials: required,
        manifest_path: manifest.map(|m| m.path.display().to_string()),
        manifest: manifest.map(|m| m.body.clone()),
    }
}

fn print_table(rows: &[ConnectorInfo]) {
    println!("| kind | auth_type | installed | methods |");
    println!("|---|---|---:|---:|");
    for row in rows {
        println!(
            "| {} | {} | {} | {} |",
            row.kind, row.auth_type, row.installed, row.methods
        );
    }
}

fn known_kinds(workspace: &Path) -> BTreeSet<String> {
    let mut kinds: BTreeSet<String> = CONNECTORS.iter().map(|s| s.kind.to_string()).collect();
    let crates = workspace.join("crates");
    if let Ok(entries) = std::fs::read_dir(crates) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if let Some(kind) = name.strip_prefix("argo-mcp-connector-") {
                kinds.insert(kind.to_string());
            }
        }
    }
    kinds
}

fn load_manifests(workspace: &Path) -> ArgoResult<BTreeMap<String, ManifestMeta>> {
    let dir = workspace.join("manifests").join("connectors");
    let mut out = BTreeMap::new();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Ok(out);
    };
    for entry in entries {
        let path = entry
            .map_err(|e| provider_err(format!("read {}: {e}", dir.display())))?
            .path();
        if path.extension().and_then(|s| s.to_str()) != Some("toml") {
            continue;
        }
        let body = std::fs::read_to_string(&path)
            .map_err(|e| provider_err(format!("read {}: {e}", path.display())))?;
        let value: Value = toml::from_str(&body)
            .map_err(|e| provider_err(format!("parse {}: {e}", path.display())))?;
        let kind = manifest_id(&value).unwrap_or_else(|| {
            path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string()
        });
        out.insert(kind, manifest_meta(path, body, &value));
    }
    Ok(out)
}

fn manifest_meta(path: PathBuf, body: String, value: &Value) -> ManifestMeta {
    ManifestMeta {
        path,
        body,
        methods: manifest_methods(value),
        required: manifest_required(value),
        auth_type: manifest_auth(value),
    }
}

fn manifest_id(value: &Value) -> Option<String> {
    value
        .get("id")
        .and_then(Value::as_str)
        .or_else(|| value.get("connector")?.get("id")?.as_str())
        .map(str::to_string)
}

fn manifest_methods(value: &Value) -> Vec<String> {
    let mut names = BTreeSet::new();
    if let Some(tools) = value.get("tools") {
        collect_tool_names(tools, &mut names);
    }
    names.into_iter().collect()
}

fn collect_tool_names(value: &Value, names: &mut BTreeSet<String>) {
    match value {
        Value::Array(items) => {
            for item in items {
                match item {
                    Value::String(s) => {
                        names.insert(s.clone());
                    }
                    Value::Table(t) => {
                        if let Some(name) = t.get("name").and_then(Value::as_str) {
                            names.insert(name.to_string());
                        }
                    }
                    _ => {}
                }
            }
        }
        Value::Table(table) => {
            if let Some(Value::Array(items)) = table.get("names") {
                for item in items {
                    if let Some(name) = item.as_str() {
                        names.insert(name.to_string());
                    }
                }
            }
        }
        _ => {}
    }
}

fn manifest_required(value: &Value) -> Vec<String> {
    let mut required = BTreeSet::new();
    for key in ["required_env", "env"] {
        collect_string_array(value.get(key), &mut required);
    }
    if let Some(token) = value.get("token_env").and_then(Value::as_str) {
        required.insert(token.to_string());
    }
    if let Some(Value::Table(env)) = value.get("env") {
        for value in env.values() {
            if let Some(name) = value.as_str() {
                required.insert(name.to_string());
            }
        }
    }
    required.into_iter().collect()
}

fn collect_string_array(value: Option<&Value>, out: &mut BTreeSet<String>) {
    if let Some(Value::Array(items)) = value {
        for item in items {
            if let Some(s) = item.as_str() {
                out.insert(s.to_string());
            }
        }
    }
}

fn manifest_auth(value: &Value) -> Option<String> {
    if value.get("oauth_provider").is_some() || value.get("oauth_scopes").is_some() {
        Some("oauth".into())
    } else if value.get("auth").is_some()
        || value.get("token_env").is_some()
        || !manifest_required(value).is_empty()
    {
        Some("token".into())
    } else {
        None
    }
}

fn auth_type(kind: &str, manifest: Option<&ManifestMeta>) -> String {
    CONNECTORS
        .iter()
        .find(|spec| spec.kind == kind)
        .map(|spec| auth_label(spec.auth).to_string())
        .or_else(|| manifest.and_then(|m| m.auth_type.clone()))
        .unwrap_or_else(|| "none".into())
}

fn spec_credentials(kind: &str) -> Vec<String> {
    let Some(spec) = CONNECTORS.iter().find(|spec| spec.kind == kind) else {
        return Vec::new();
    };
    let mut keys = vec![spec.token_key.to_string()];
    if let Some(key) = spec.base_url_key {
        keys.push(key.to_string());
    }
    if let Some(oauth) = spec.oauth {
        keys.push(oauth.client_id_key.to_string());
        keys.push(oauth.client_secret_key.to_string());
        if let Some(key) = oauth.refresh_key {
            keys.push(key.to_string());
        }
        if let Some(key) = oauth.instance_url_key {
            keys.push(key.to_string());
        }
    }
    keys
}

fn crate_methods(workspace: &Path, kind: &str) -> Vec<String> {
    let path = workspace
        .join("crates")
        .join(format!("argo-mcp-connector-{kind}"))
        .join("src")
        .join("schemas.rs");
    let Ok(body) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    let prefix = format!("{kind}.");
    body.split('"')
        .filter(|part| part.starts_with(&prefix))
        .map(str::to_string)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn auth_label(auth: AuthKind) -> &'static str {
    match auth {
        AuthKind::OAuth => "oauth",
        AuthKind::Token => "token",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_shapes_parse_tool_names() {
        let value: Value = toml::from_str(
            r#"
            id = "gmail"
            [[tools]]
            name = "gmail.search"
            [[tools]]
            name = "gmail.send"
            "#,
        )
        .unwrap();
        assert_eq!(manifest_methods(&value), vec!["gmail.search", "gmail.send"]);
    }

    #[test]
    fn installed_only_filters_to_manifest_rows() {
        let dir = tempfile::tempdir().unwrap();
        let manifest_dir = dir.path().join("manifests/connectors");
        std::fs::create_dir_all(&manifest_dir).unwrap();
        std::fs::write(
            manifest_dir.join("mini.toml"),
            "id = \"mini\"\ntools = [\"mini.ping\"]",
        )
        .unwrap();
        let rows = inventory(dir.path(), true).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].kind, "mini");
        assert!(rows[0].installed);
    }

    #[test]
    fn crate_methods_reads_schema_names() {
        let dir = tempfile::tempdir().unwrap();
        let schema = dir
            .path()
            .join("crates/argo-mcp-connector-demo/src/schemas.rs");
        std::fs::create_dir_all(schema.parent().unwrap()).unwrap();
        std::fs::write(&schema, r#"name: "demo.one"; read_tool("demo.two")"#).unwrap();
        assert_eq!(
            crate_methods(dir.path(), "demo"),
            vec!["demo.one", "demo.two"]
        );
    }
}
