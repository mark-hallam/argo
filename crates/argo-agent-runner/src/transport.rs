//! Transport construction. Resolves a `ProviderManifest` (loaded
//! from `<cwd>/manifests/providers/<id>.toml`, with a fallback
//! to `$ARGO_MANIFESTS_DIR/providers/<id>.toml` when the
//! workspace doesn't carry its own copy) to a live
//! `Box<dyn Transport>`.
use crate::transport_gate::TransportGate;
use argo_provider::{
    AnthropicTransport, BedrockTransport, ClaudeCodeCliTransport, DedupCache,
    OpenAiCompatTransport, OpenAiResponsesTransport, ProviderFamily, ProviderManifest,
    Transport, TransportKind,
};
use argo_types::{ArgoError, ArgoResult, TenantId};
use std::path::{Path, PathBuf};

/// Resolve a `ProviderManifest` to a live `Transport`. Anthropic /
/// OpenAI families read the API key from `manifest.auth.api_key_env`;
/// Bedrock loads AWS credentials via `aws_config::load_from_env()`
/// (honours the standard `AWS_*` env vars + `~/.aws` profile).
/// Subprocess-kind manifests (e.g. `claude-code-cli.toml`) skip the
/// API-key resolution path entirely and dispatch to the subprocess
/// driver. When `ARGO_PROVIDER_DEDUP_TTL_SECS` is a positive integer,
/// the result is additionally wrapped with [`DedupCache`] so identical
/// non-streaming requests in the TTL window collapse to one wire call.
///
/// This is the **process-global** entrypoint: the API key comes from
/// the platform env / unscoped credential store. For per-tenant BYOK
/// (Bring Your Own Key, DEC-053 Enterprise tier) resolution where a
/// tenant's own key must WIN over the platform key, use
/// [`build_transport_for_tenant`].
///
/// `gate` is the **required** hard-`[budget.ceiling]` spend context
/// ([`TransportGate`]). It is mandatory by design: the returned transport
/// is gated *by construction*, so no call site can obtain a bare,
/// un-metered transport (the WS5 open-wallet close). Pass
/// [`TransportGate::InternalTrusted`] for operator-run CLI tooling,
/// [`TransportGate::Anonymous`] for pre-auth gateway paths, or
/// [`TransportGate::Session`] for an authenticated `(tenant, session)`.
pub async fn build_transport(
    manifest: ProviderManifest,
    gate: TransportGate<'_>,
) -> ArgoResult<Box<dyn Transport>> {
    Ok(gate.wrap(build_transport_inner(manifest).await?))
}

/// Raw transport construction (no spend gate). Private so the gate cannot
/// be bypassed — every public path runs through [`build_transport`].
async fn build_transport_inner(manifest: ProviderManifest) -> ArgoResult<Box<dyn Transport>> {
    // Subprocess-kind manifests use no API key — dispatch first so the
    // missing `manifest.auth.api_key_env` value (it's only a placeholder
    // for these manifests) doesn't trigger the `not set` error below.
    if matches!(manifest.provider.transport_kind, TransportKind::Subprocess) {
        return Ok(maybe_wrap_dedup(Box::new(
            ClaudeCodeCliTransport::from_manifest(manifest)?,
        )));
    }
    if matches!(manifest.provider.family, ProviderFamily::Bedrock) {
        return build_bedrock(manifest).await;
    }
    let api_key_env = manifest.auth.api_key_env.clone();
    // tenant-unscoped: provider API key resolved at transport layer (process-wide, not per-tenant)
    let api_key = argo_config::cred::get_unscoped(&api_key_env)
        .map_err(|_| ArgoError::Provider(format!("{api_key_env} not set")))?;
    Ok(maybe_wrap_dedup(transport_for_family(manifest, &api_key)?))
}

/// Per-tenant BYOK variant of [`build_transport`] (DEC-053 Enterprise).
///
/// API-key families resolve the key via [`resolve_provider_api_key`]:
/// the tenant's own key (stored under `[credentials.tenant.<tid>]`)
/// WINS over the platform/process key. `allow_platform_fallback`
/// controls what happens when the tenant has NO key of their own:
///
/// * `true` (standard tier) — fall back to the platform/process key,
///   exactly the [`build_transport`] behaviour. Backward-compatible:
///   a tenant who never set a BYOK key is served the platform key.
/// * `false` (enterprise BYOK default — OFF) — **fail closed** with an
///   honest error rather than silently billing the platform key. A
///   silent fallback would un-do the zero-COGS margin model.
///
/// Note the fail-closed contract is about a *missing* tenant key. A
/// tenant whose own key is present-but-invalid is still built with that
/// key, so the provider returns an honest auth error at call time —
/// there is deliberately no retry-with-platform path here.
///
/// Subprocess (claude-code-cli OAuth) and Bedrock (AWS credentials)
/// manifests carry no per-tenant API key and behave as in
/// [`build_transport`].
pub async fn build_transport_for_tenant(
    manifest: ProviderManifest,
    workspace: &Path,
    tenant_id: &TenantId,
    allow_platform_fallback: bool,
    gate: TransportGate<'_>,
) -> ArgoResult<Box<dyn Transport>> {
    let inner =
        build_transport_for_tenant_inner(manifest, workspace, tenant_id, allow_platform_fallback)
            .await?;
    Ok(gate.wrap(inner))
}

/// Raw per-tenant transport construction (no spend gate). Private so the
/// gate cannot be bypassed — every public path runs through
/// [`build_transport_for_tenant`].
async fn build_transport_for_tenant_inner(
    manifest: ProviderManifest,
    workspace: &Path,
    tenant_id: &TenantId,
    allow_platform_fallback: bool,
) -> ArgoResult<Box<dyn Transport>> {
    if matches!(manifest.provider.transport_kind, TransportKind::Subprocess) {
        return Ok(maybe_wrap_dedup(Box::new(
            ClaudeCodeCliTransport::from_manifest(manifest)?,
        )));
    }
    if matches!(manifest.provider.family, ProviderFamily::Bedrock) {
        return build_bedrock(manifest).await;
    }
    let api_key_env = manifest.auth.api_key_env.clone();
    let api_key =
        resolve_provider_api_key(workspace, tenant_id, &api_key_env, allow_platform_fallback)?;
    Ok(maybe_wrap_dedup(transport_for_family(manifest, &api_key)?))
}

/// Resolve the provider API key for `tenant_id`, applying the BYOK
/// precedence: the tenant's own stored key WINS over the platform key.
///
/// Resolution order:
/// 1. The tenant's OWN key in `[credentials.tenant.<tid>]` — read via
///    [`argo_config::cred::load_tenant`], which consults ONLY that
///    tenant's section (never the process env, never the legacy
///    root-level section), so one tenant's key can never resolve for
///    another tenant.
/// 2. No per-tenant key → the platform/process key, but ONLY when
///    `allow_platform_fallback` is `true`.
/// 3. Otherwise **fail closed** with an honest error.
///
/// The returned `String` is the exact key handed to the transport's
/// `from_manifest`; the error messages name only the env-var KEY and
/// the tenant id — never a secret value.
///
/// # Errors
/// Returns [`ArgoError::Provider`] when no per-tenant key is set and
/// either fallback is disabled, or fallback is enabled but the platform
/// key is also absent.
pub fn resolve_provider_api_key(
    workspace: &Path,
    tenant_id: &TenantId,
    api_key_env: &str,
    allow_platform_fallback: bool,
) -> ArgoResult<String> {
    // 1. Tenant's OWN key — tenant section only, no env / legacy fallback.
    let tenant_creds = argo_config::cred::load_tenant(workspace, tenant_id).unwrap_or_default();
    if let Some(key) = tenant_creds.get(api_key_env) {
        let trimmed = key.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }
    // 2. No per-tenant key: platform key only when fallback is allowed.
    if allow_platform_fallback {
        // tenant-unscoped: standard-tier fallback to the platform key when the
        // tenant has set no BYOK key of their own.
        return argo_config::cred::get_unscoped(api_key_env)
            .map_err(|_| ArgoError::Provider(format!("{api_key_env} not set")));
    }
    // 3. Fail closed — enterprise BYOK tenant with no key + fallback OFF.
    Err(ArgoError::Provider(format!(
        "no per-tenant {api_key_env} configured for tenant {tenant_id} \
         and platform-key fallback is disabled for this tenant"
    )))
}

/// Construct the AWS-credential Bedrock transport. Shared by both
/// [`build_transport`] and [`build_transport_for_tenant`] — Bedrock
/// authenticates via the AWS credential chain, not a per-tenant key.
async fn build_bedrock(manifest: ProviderManifest) -> ArgoResult<Box<dyn Transport>> {
    let cfg = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    Ok(maybe_wrap_dedup(Box::new(BedrockTransport::from_manifest(
        manifest, &cfg,
    )?)))
}

/// Build the family-specific API-key transport from a resolved key.
/// Bedrock is handled by [`build_bedrock`] before this is reached.
fn transport_for_family(
    manifest: ProviderManifest,
    api_key: &str,
) -> ArgoResult<Box<dyn Transport>> {
    let t: Box<dyn Transport> = match manifest.provider.family {
        ProviderFamily::Anthropic => Box::new(AnthropicTransport::from_manifest(manifest, api_key)?),
        ProviderFamily::OpenaiCompatible => {
            Box::new(OpenAiCompatTransport::from_manifest(manifest, api_key)?)
        }
        ProviderFamily::OpenaiResponses => {
            Box::new(OpenAiResponsesTransport::from_manifest(manifest, api_key)?)
        }
        // Already handled by the caller; reachable only if a new family
        // is added without wiring it through. Fail loud.
        ProviderFamily::Bedrock => unreachable!("bedrock handled by caller"),
    };
    Ok(t)
}

/// Conditionally wrap `inner` with [`DedupCache`] when
/// `ARGO_PROVIDER_DEDUP_TTL_SECS` parses to a positive integer.
fn maybe_wrap_dedup(inner: Box<dyn Transport>) -> Box<dyn Transport> {
    match std::env::var("ARGO_PROVIDER_DEDUP_TTL_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
    {
        Some(secs) if secs > 0 => {
            Box::new(DedupCache::new(inner, std::time::Duration::from_secs(secs)))
        }
        _ => inner,
    }
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
        .map_err(|e| ArgoError::Io(format!("read {}: {e}", path.display())))?;
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

    // ---- BYOK per-tenant key resolution (DEC-053 Enterprise) ----
    //
    // These pin the security contract for `resolve_provider_api_key`,
    // the seam whose return value is handed verbatim to the transport's
    // `from_manifest`. Each test uses a UNIQUE `api_key_env` name so the
    // process-env reads never race the parallel test harness.

    #[test]
    fn tenant_key_wins_over_platform_env() {
        let ws = tempfile::tempdir().expect("ws tempdir");
        let tenant = TenantId::new();
        let env_key = "BYOK_TEST_TENANT_WINS";
        // Platform key present in process env...
        // SAFETY: unique-per-test env name; restored below.
        unsafe { std::env::set_var(env_key, "PLATFORM-KEY") };
        // ...but the tenant set their OWN key, which must win.
        argo_config::cred::set_tenant(ws.path(), &tenant, env_key, "TENANT-OWN-KEY")
            .expect("set tenant key");
        let resolved = resolve_provider_api_key(ws.path(), &tenant, env_key, true)
            .expect("resolves tenant key");
        unsafe { std::env::remove_var(env_key) };
        assert_eq!(
            resolved, "TENANT-OWN-KEY",
            "the tenant's own key must WIN over the platform env key"
        );
    }

    #[test]
    fn no_tenant_key_falls_back_to_platform_when_allowed() {
        let ws = tempfile::tempdir().expect("ws tempdir");
        let tenant = TenantId::new();
        let env_key = "BYOK_TEST_PLATFORM_FALLBACK";
        // SAFETY: unique-per-test env name; restored below.
        unsafe { std::env::set_var(env_key, "PLATFORM-KEY") };
        let resolved = resolve_provider_api_key(ws.path(), &tenant, env_key, true)
            .expect("falls back to platform key");
        unsafe { std::env::remove_var(env_key) };
        assert_eq!(
            resolved, "PLATFORM-KEY",
            "a tenant with no BYOK key gets the platform key when fallback is allowed"
        );
    }

    #[test]
    fn fail_closed_when_no_tenant_key_and_fallback_off() {
        let ws = tempfile::tempdir().expect("ws tempdir");
        let tenant = TenantId::new();
        // fallback=false path never reads the process env, so we don't
        // touch it — the absence of a tenant key alone must fail closed.
        let err = resolve_provider_api_key(ws.path(), &tenant, "BYOK_TEST_FAIL_CLOSED", false)
            .expect_err("must fail closed");
        let msg = format!("{err}");
        assert!(
            msg.contains("BYOK_TEST_FAIL_CLOSED") && msg.contains("fallback is disabled"),
            "fail-closed error must name the env key + the disabled fallback; got: {msg}"
        );
    }

    #[test]
    fn cross_tenant_isolation_no_key_leak() {
        let ws = tempfile::tempdir().expect("ws tempdir");
        let tenant_a = TenantId::new();
        let tenant_b = TenantId::new();
        let env_key = "BYOK_TEST_CROSS_TENANT";
        let secret_a = "SECRET-FOR-TENANT-A-9f3";
        argo_config::cred::set_tenant(ws.path(), &tenant_a, env_key, secret_a)
            .expect("set tenant A key");
        // Tenant A resolves their own key.
        let a = resolve_provider_api_key(ws.path(), &tenant_a, env_key, false)
            .expect("tenant A resolves own key");
        assert_eq!(a, secret_a);
        // Tenant B (who set nothing) must NOT see tenant A's key, even
        // though it lives in the same credentials file — fail closed.
        let err = resolve_provider_api_key(ws.path(), &tenant_b, env_key, false)
            .expect_err("tenant B must not inherit tenant A's key");
        let msg = format!("{err}");
        assert!(
            !msg.contains(secret_a),
            "secret must NEVER appear in the error surfaced to another tenant; got: {msg}"
        );
    }
}
