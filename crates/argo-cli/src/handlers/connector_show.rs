//! `argo connector show`.

use super::connector_authorize::{provider_err, workspace_dir};
use super::connector_list::{find_connector, ConnectorInfo};
use argo_types::ArgoResult;
use serde_json::json;

pub async fn run(kind: &str, json_out: bool) -> ArgoResult<()> {
    let workspace = workspace_dir()?;
    let info = find_connector(&workspace, kind)?;
    if json_out {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "ok": true,
                "connector_show": info
            }))
            .map_err(|e| provider_err(format!("serialize JSON: {e}")))?
        );
    } else {
        print!("{}", render_detail(&info));
    }
    Ok(())
}

fn render_detail(info: &ConnectorInfo) -> String {
    let mut out = String::new();
    out.push_str(&format!("# connector {}\n\n", info.kind));
    out.push_str(&format!("auth_type: {}\n", info.auth_type));
    out.push_str(&format!("installed: {}\n", info.installed));
    out.push_str(&format!("methods: {}\n", info.methods));
    if !info.required_credentials.is_empty() {
        out.push_str("required credentials:\n");
        for key in &info.required_credentials {
            out.push_str(&format!("- {key}\n"));
        }
    }
    if !info.method_names.is_empty() {
        out.push_str("method list:\n");
        for name in &info.method_names {
            out.push_str(&format!("- {name}\n"));
        }
    }
    out.push_str("manifest:\n");
    if let Some(path) = &info.manifest_path {
        out.push_str(&format!("path: {path}\n"));
    }
    match &info.manifest {
        Some(body) => {
            out.push_str("```toml\n");
            out.push_str(body.trim_end());
            out.push('\n');
            out.push_str("```\n");
        }
        None => out.push_str("(not installed)\n"),
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_detail_includes_manifest_and_methods() {
        let info = ConnectorInfo {
            kind: "gmail".into(),
            auth_type: "oauth".into(),
            installed: true,
            methods: 1,
            method_names: vec!["gmail.search".into()],
            required_credentials: vec!["GMAIL_ACCESS_TOKEN".into()],
            manifest_path: Some("manifests/connectors/gmail.toml".into()),
            manifest: Some("id = \"gmail\"".into()),
        };
        let out = render_detail(&info);
        assert!(out.contains("methods: 1"));
        assert!(out.contains("- gmail.search"));
        assert!(out.contains("```toml"));
    }
}
