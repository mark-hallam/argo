//! `argo vertical pull <oci-ref> [--rename <id>]`
//! (Stream `claude-marketplace-vertical-publish`).
//!
//! Inverse of `vertical_publish`. Resolves the OCI reference,
//! pulls the tar + manifest, verifies the tar's sha256 matches
//! the manifest's declared digest BEFORE writing any file, then
//! unpacks under `verticals/<id>/`.
//!
//! Conflict resolution:
//! - Refuses if a vertical with the same id already exists locally
//!   (operator must delete first or use `--rename <new-id>`).
//! - `--rename <new-id>` lets the operator pull a registry artifact
//!   under a different local id (useful when forking or
//!   side-by-side comparing two versions).
use argo_marketplace::{
    resolve_registry_auth, unpack_vertical_bundle, OciDistributionRegistry, OciReference,
    OciRegistry,
};
use argo_types::{ArgoError, ArgoResult};

/// Top-level entry for `argo vertical pull`.
///
/// `reference` examples:
/// - `ghcr.io/argo-agents/marketplace-seller:1.0.0`
/// - `localhost:5000/marketplace-seller:0.1`
///
/// `registry_url` is the HTTPS base. When omitted, falls back
/// to `https://<host>` derived from the reference.
pub async fn run(
    reference: &str,
    registry_url: Option<&str>,
    rename: Option<&str>,
    auth_token: Option<&str>,
    json: bool,
) -> ArgoResult<()> {
    let parsed = OciReference::parse(reference)?;
    let url = registry_url
        .map(str::to_string)
        .unwrap_or_else(|| derive_registry_url(&parsed.host));
    let auth = resolve_registry_auth(&parsed.host, auth_token);
    let oci = OciDistributionRegistry::with_resolved_auth(&url, auth);
    let (tar_bytes, manifest) = oci.pull(&parsed).await?;

    let local_id = rename
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| repo_basename(&parsed.repo));
    validate_local_id(&local_id)?;

    let dir = argo_vertical::default_dir();
    let target_root = dir.join(&local_id);
    if target_root.exists() {
        return Err(ArgoError::Provider(format!(
            "vertical '{local_id}' already exists at {} \
             — uninstall it first, or pull with --rename <new-id>",
            target_root.display()
        )));
    }

    unpack_vertical_bundle(&tar_bytes, &manifest, &target_root).await?;

    // Re-validate the unpacked tree so the operator finds out
    // immediately if the registry artifact was malformed.
    let manifest_path = target_root.join("vertical.toml");
    if !manifest_path.exists() {
        // Roll back so a half-extracted vertical doesn't litter the
        // discovery dir.
        let _ = tokio::fs::remove_dir_all(&target_root).await;
        return Err(ArgoError::Provider(format!(
            "pulled artifact at {} is missing vertical.toml — refusing to register",
            reference
        )));
    }

    if json {
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({
                "ok": true,
                "reference": parsed.to_canonical(),
                "id": local_id,
                "root": target_root.display().to_string(),
                "digest": manifest.digest,
                "entries": manifest.entries.len(),
                "bytes": tar_bytes.len(),
                "renamed": rename.is_some(),
            }))
            .unwrap_or_default()
        );
    } else {
        println!(
            "pulled {} → {} ({} entries)",
            parsed.to_canonical(),
            target_root.display(),
            manifest.entries.len()
        );
        println!("  digest: {}", manifest.digest);
        println!("  bytes:  {}", tar_bytes.len());
    }
    Ok(())
}

fn derive_registry_url(host: &str) -> String {
    if host.starts_with("localhost") || host.starts_with("127.0.0.1") {
        format!("http://{host}")
    } else {
        format!("https://{host}")
    }
}

fn repo_basename(repo: &str) -> String {
    repo.rsplit('/').next().unwrap_or(repo).to_string()
}

/// Vertical ids are constrained to `[a-z0-9-]+` to match the
/// `argo_vertical` loader's discovery rules. Reject pulls that
/// would create an unloadable tree.
fn validate_local_id(id: &str) -> ArgoResult<()> {
    if id.is_empty() {
        return Err(ArgoError::Schema("vertical id is empty".into()));
    }
    let ok = id
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if !ok {
        return Err(ArgoError::Schema(format!(
            "vertical id '{id}' must match [a-z0-9-]+ — pass --rename with a valid id"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{derive_registry_url, repo_basename, validate_local_id};

    #[test]
    fn derive_registry_url_picks_http_for_localhost() {
        assert_eq!(
            derive_registry_url("localhost:5000"),
            "http://localhost:5000"
        );
        assert_eq!(
            derive_registry_url("127.0.0.1:5000"),
            "http://127.0.0.1:5000"
        );
        assert_eq!(derive_registry_url("ghcr.io"), "https://ghcr.io");
    }

    #[test]
    fn repo_basename_strips_org_prefix() {
        assert_eq!(repo_basename("argo-agents/seller"), "seller");
        assert_eq!(repo_basename("seller"), "seller");
        assert_eq!(repo_basename("a/b/c/seller"), "seller");
    }

    #[test]
    fn validate_local_id_accepts_dashed_lowercase() {
        validate_local_id("marketplace-seller").unwrap();
        validate_local_id("seller01").unwrap();
    }

    #[test]
    fn validate_local_id_rejects_uppercase_and_underscore() {
        assert!(validate_local_id("Seller").is_err());
        assert!(validate_local_id("market_place").is_err());
        assert!(validate_local_id("").is_err());
    }
}
