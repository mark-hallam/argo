//! `argo vertical publish <id> --registry <url> [--tag <ver>]`
//! (Stream `claude-marketplace-vertical-publish`).
//!
//! Packs `verticals/<id>/` into a deterministic tar via
//! [`argo_marketplace::build_vertical_bundle`], then pushes to an
//! OCI registry via [`argo_marketplace::OciDistributionRegistry`].
//! The deterministic tar means re-publishing the same on-disk
//! state yields the same digest, so receivers can verify the
//! pull byte-for-byte (handler in `vertical_pull.rs`).
//!
//! The handler defaults `--tag` to the vertical's manifest
//! version when omitted so the operator can run a one-liner.
use argo_marketplace::{
    build_vertical_bundle, OciDistributionRegistry, OciReference, OciRegistry,
};
use argo_types::{ArgoError, ArgoResult};
use argo_vertical::load_dir;
use std::path::PathBuf;

/// Top-level entry for `argo vertical publish`.
///
/// `registry_url` examples:
/// - `https://ghcr.io` (org-namespaced refs land at
///   `ghcr.io/<org>/<id>:<tag>`)
/// - `http://localhost:5000` (a local `registry:2` test container)
///
/// `repo_org` is the org/namespace prefix on the registry side
/// (e.g. `argo-agents`). When omitted, the vertical id is used
/// as the bare repo (`<host>/<id>:<tag>`).
pub async fn run(
    id: &str,
    registry_url: &str,
    tag: Option<&str>,
    repo_org: Option<&str>,
    auth_token: Option<&str>,
    json: bool,
) -> ArgoResult<()> {
    let dir = argo_vertical::default_dir();
    let (registry, _errors) = load_dir(&dir)?;
    let loaded = registry.get(id).ok_or_else(|| {
        ArgoError::Provider(format!(
            "vertical '{id}' not found in {}",
            dir.display()
        ))
    })?;
    let resolved_tag = tag
        .map(str::to_string)
        .unwrap_or_else(|| loaded.manifest.vertical.version.clone());
    let host = host_from_url(registry_url);
    let repo = match repo_org {
        Some(org) if !org.is_empty() => format!("{org}/{id}"),
        _ => id.to_string(),
    };
    let reference = OciReference {
        host: host.clone(),
        repo,
        tag: resolved_tag.clone(),
    };

    let tmp = tempfile::tempdir()
        .map_err(|e| ArgoError::Io(format!("create tempdir for publish: {e}")))?;
    let tar_path: PathBuf = tmp.path().join(format!("{id}.tar"));
    let manifest =
        build_vertical_bundle(&loaded.root, id, &resolved_tag, &tar_path).await?;
    let tar_bytes = tokio::fs::read(&tar_path)
        .await
        .map_err(|e| ArgoError::Io(format!("read built tar: {e}")))?;

    let oci = OciDistributionRegistry::new(registry_url, auth_token.map(str::to_string));
    oci.push(&reference, &tar_bytes, &manifest).await?;

    if json {
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({
                "ok": true,
                "id": id,
                "reference": reference.to_canonical(),
                "registry_url": registry_url,
                "tag": resolved_tag,
                "digest": manifest.digest,
                "entries": manifest.entries.len(),
                "bytes": tar_bytes.len(),
            }))
            .unwrap_or_default()
        );
    } else {
        println!("published {} ({} entries)", reference.to_canonical(), manifest.entries.len());
        println!("  digest:   {}", manifest.digest);
        println!("  bytes:    {}", tar_bytes.len());
        println!("  registry: {registry_url}");
    }
    Ok(())
}

fn host_from_url(url: &str) -> String {
    let without_scheme = url
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    without_scheme.trim_end_matches('/').to_string()
}

#[cfg(test)]
mod tests {
    use super::host_from_url;

    #[test]
    fn host_from_url_strips_scheme_and_trailing_slash() {
        assert_eq!(host_from_url("https://ghcr.io"), "ghcr.io");
        assert_eq!(host_from_url("http://localhost:5000/"), "localhost:5000");
        assert_eq!(host_from_url("registry.local:5000"), "registry.local:5000");
    }
}
