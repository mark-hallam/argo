//! Transport construction. Resolves a `ProviderManifest` (loaded
//! from `<cwd>/manifests/providers/<id>.toml`, with a fallback
//! to `$ARGO_MANIFESTS_DIR/providers/<id>.toml` when the
//! workspace doesn't carry its own copy) to a live
//! `Box<dyn Transport>`.
use argo_provider::{
    AnthropicTransport, BedrockTransport, OpenAiCompatTransport, OpenAiResponsesTransport,
    ProviderFamily, ProviderManifest, Transport,
};
use argo_types::{ArgoError, ArgoResult};
use std::path::{Path, PathBuf};

/// Resolve a `ProviderManifest` to a live `Transport`. Anthropic /
/// OpenAI families read the API key from `manifest.auth.api_key_env`;
/// Bedrock loads AWS credentials via `aws_config::load_from_env()`
/// (honours the standard `AWS_*` env vars + `~/.aws` profile).
pub async fn build_transport(manifest: ProviderManifest) -> ArgoResult<Box<dyn Transport>> {
    if matches!(manifest.provider.family, ProviderFamily::Bedrock) {
        let cfg = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
        return Ok(Box::new(BedrockTransport::from_manifest(manifest, &cfg)?));
    }
    let api_key_env = manifest.auth.api_key_env.clone();
    let api_key = argo_config::cred::get(&api_key_env)
        .map_err(|_| ArgoError::Provider(format!("{api_key_env} not set")))?;
    let t: Box<dyn Transport> = match manifest.provider.family {
        ProviderFamily::Anthropic => {
            Box::new(AnthropicTransport::from_manifest(manifest, &api_key)?)
        }
        ProviderFamily::OpenaiCompatible => {
            Box::new(OpenAiCompatTransport::from_manifest(manifest, &api_key)?)
        }
        ProviderFamily::OpenaiResponses => {
            Box::new(OpenAiResponsesTransport::from_manifest(manifest, &api_key)?)
        }
        // Already handled above; reachable only if a new family is
        // added without wiring it through. Fail loud.
        ProviderFamily::Bedrock => unreachable!("bedrock handled above"),
    };
    Ok(t)
}

/// Resolve and parse a provider manifest. Search order:
///
/// 1. `<cwd>/manifests/providers/<provider-id>.toml` —
///    workspace-local override (per-tenant customisation, the
///    spec layout).
/// 2. `$ARGO_MANIFESTS_DIR/providers/<provider-id>.toml` —
///    operator-installed shared location, the same env-var
///    pattern as `ARGO_VERTICALS_DIR`. Lets a workspace that
///    doesn't carry its own `manifests/` (e.g. a fresh
///    `argo workspace init` directory) still resolve providers
///    against the canonical shipped manifests.
///
/// Returns the cwd-path error when neither location holds the
/// file, so the operator-facing diagnostic still names the path
/// the worker was looking at first. The `$ARGO_MANIFESTS_DIR`
/// fallback path is appended to the error so the operator sees
/// both attempted locations in one shot.
pub fn load_manifest(cwd: &Path, provider_id: &str) -> ArgoResult<ProviderManifest> {
    let cwd_path = cwd
        .join("manifests")
        .join("providers")
        .join(format!("{provider_id}.toml"));
    if cwd_path.is_file() {
        return parse_at(&cwd_path);
    }
    if let Some(env_dir) = std::env::var_os("ARGO_MANIFESTS_DIR") {
        let env_path = PathBuf::from(env_dir)
            .join("providers")
            .join(format!("{provider_id}.toml"));
        if env_path.is_file() {
            return parse_at(&env_path);
        }
        return Err(ArgoError::Provider(format!(
            "provider manifest `{provider_id}` not found — tried {} \
             AND {} (via $ARGO_MANIFESTS_DIR)",
            cwd_path.display(),
            env_path.display(),
        )));
    }
    Err(ArgoError::Provider(format!(
        "provider manifest `{provider_id}` not found at {} \
         (set $ARGO_MANIFESTS_DIR to point at a shared providers/ \
         directory, or create the file under the workspace)",
        cwd_path.display(),
    )))
}

fn parse_at(path: &Path) -> ArgoResult<ProviderManifest> {
    let body = std::fs::read_to_string(path)
        .map_err(|e| ArgoError::Provider(format!("read {}: {e}", path.display())))?;
    ProviderManifest::from_toml_str(&body)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_manifest(dir: &Path, provider_id: &str, body: &str) {
        let path = dir.join("providers");
        std::fs::create_dir_all(&path).expect("mkdir providers");
        std::fs::write(path.join(format!("{provider_id}.toml")), body).expect("write manifest");
    }

    fn fixture_anthropic() -> String {
        // Use the real shipped manifest as a fixture — the
        // schema has many required fields and re-deriving the
        // minimal valid shape inline is brittle. The file is a
        // few hundred bytes and version-controlled, so re-using
        // it is stable AND keeps the test honest.
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(std::path::Path::parent)
            .expect("workspace root")
            .join("manifests/providers/anthropic.toml");
        std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read fixture {}: {e}", path.display()))
    }

    #[test]
    fn loads_from_cwd_when_workspace_carries_manifest() {
        let cwd = tempfile::tempdir().expect("cwd tempdir");
        write_manifest(
            &cwd.path().join("manifests"),
            "anthropic",
            &fixture_anthropic(),
        );
        let m = load_manifest(cwd.path(), "anthropic").expect("loads from cwd");
        assert_eq!(m.provider.id, "anthropic");
    }

    #[test]
    fn falls_back_to_argo_manifests_dir_when_workspace_path_missing() {
        // Simulates a fresh workspace where the operator hasn't
        // copied manifests in: cwd has no providers, but the
        // env var points at the shared shipped location.
        let cwd = tempfile::tempdir().expect("cwd tempdir");
        let shared = tempfile::tempdir().expect("shared tempdir");
        write_manifest(shared.path(), "anthropic", &fixture_anthropic());
        // SAFETY: tests in this module are serial-on-write of
        // ARGO_MANIFESTS_DIR. The env-var read happens inside
        // load_manifest synchronously so this set/clear pair
        // doesn't race — but we keep the cleanup tight regardless.
        // Cargo test harness defaults to single-threaded for
        // env-touching tests via `--test-threads=1` is the
        // belt-and-braces fix; the cwd-only fallback test below
        // doesn't touch the var so order doesn't matter.
        let prev = std::env::var_os("ARGO_MANIFESTS_DIR");
        // SAFETY: tests run in a single test process; we
        // restore the prior value below before returning.
        unsafe {
            std::env::set_var("ARGO_MANIFESTS_DIR", shared.path());
        }
        let result = load_manifest(cwd.path(), "anthropic");
        // Restore env regardless of test outcome.
        unsafe {
            match prev {
                Some(v) => std::env::set_var("ARGO_MANIFESTS_DIR", v),
                None => std::env::remove_var("ARGO_MANIFESTS_DIR"),
            }
        }
        let m = result.expect("loads from ARGO_MANIFESTS_DIR fallback");
        assert_eq!(m.provider.id, "anthropic");
    }

    #[test]
    fn errors_clearly_when_neither_location_has_the_manifest() {
        let cwd = tempfile::tempdir().expect("cwd tempdir");
        // Make sure ARGO_MANIFESTS_DIR is unset for this test.
        let prev = std::env::var_os("ARGO_MANIFESTS_DIR");
        unsafe {
            std::env::remove_var("ARGO_MANIFESTS_DIR");
        }
        let err = load_manifest(cwd.path(), "anthropic").unwrap_err();
        unsafe {
            if let Some(v) = prev {
                std::env::set_var("ARGO_MANIFESTS_DIR", v);
            }
        }
        let msg = format!("{err}");
        assert!(
            msg.contains("anthropic")
                && msg.contains("not found")
                && msg.contains("ARGO_MANIFESTS_DIR"),
            "error must name the provider AND mention the env-var fallback; got: {msg}"
        );
    }

    #[test]
    fn cwd_wins_over_env_dir_when_both_exist() {
        // The spec contract: workspace overrides take precedence
        // over the shared shipped location. Locks that.
        let cwd = tempfile::tempdir().expect("cwd tempdir");
        let shared = tempfile::tempdir().expect("shared tempdir");
        // cwd has the canonical manifest...
        write_manifest(
            &cwd.path().join("manifests"),
            "anthropic",
            &fixture_anthropic(),
        );
        // ...and the shared dir has a DIFFERENT id under the same
        // file name. The cwd one should win. Mutate the real
        // fixture's `id` field so the file still parses but the
        // manifest is distinguishable.
        let shared_body =
            fixture_anthropic().replace("id = \"anthropic\"", "id = \"anthropic-but-overridden\"");
        write_manifest(shared.path(), "anthropic", &shared_body);
        let prev = std::env::var_os("ARGO_MANIFESTS_DIR");
        unsafe {
            std::env::set_var("ARGO_MANIFESTS_DIR", shared.path());
        }
        let result = load_manifest(cwd.path(), "anthropic");
        unsafe {
            match prev {
                Some(v) => std::env::set_var("ARGO_MANIFESTS_DIR", v),
                None => std::env::remove_var("ARGO_MANIFESTS_DIR"),
            }
        }
        let m = result.expect("loads from cwd");
        assert_eq!(
            m.provider.id, "anthropic",
            "cwd manifest must win over $ARGO_MANIFESTS_DIR fallback"
        );
    }
}
