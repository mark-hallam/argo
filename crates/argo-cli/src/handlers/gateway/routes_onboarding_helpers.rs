//! Common helpers for the three `web-gateway-endpoints-onboarding-batch`
//! handlers. Sibling of `routes_onboarding.rs`; split out to keep both
//! files under the 500-LOC budget. No public surface — every item is
//! `pub(super)` and consumed only by `routes_onboarding`.

use super::routes_auth_principal::principal_id_for_audit;
use argo_audit::{AuditKind, AuditSink};
use argo_credentials::{encrypt_value, MasterKey};
use argo_identity::tenant_provision::{provision_for_user, ProvisionRequest};
use argo_identity::user_store::get_user_by_email;
use argo_identity::UserAccount;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId, WorkspaceId};
use argo_vertical::LoadedVertical;
use regex::Regex;
use serde_json::{json, Value};
use std::path::Path;
use std::sync::{Arc, OnceLock};

// The former `cwd_write_tenant` shim lived here. It resolved the
// gateway-cwd tenant for the first-hire write so the worker would be
// visible to the (then cwd-based) read routes. #2676 migrated the reads
// to the `__Host-session` JWT tenant, and sprint WS1-2 migrated the
// first-hire write to the SAME canonical resolver
// (`resolve_tenant::resolve_tenant_for_request`), so this cwd shim is no
// longer needed and was removed to keep a single tenant-resolution home.

pub(super) const HUMAN_PRINCIPAL_PREFIX: &str = "human:";
const PRINCIPAL_HEADER: &str = "X-Argo-Principal";

/// Why [`resolve_workspace`] couldn't return a `(tenant, workspace)` pair.
pub(super) enum WorkspaceResolve {
    /// The user has not yet POSTed `/api/v1/onboarding/provision`.
    NotProvisioned,
    /// `X-Argo-Principal` named an email that isn't in `user_account`.
    UserMissing,
    /// SQL / serialization failure.
    Internal(String),
}

/// Resolve the caller's `(tenant_id, workspace_id)`. Re-uses
/// `provision_for_user`'s idempotent re-call path so existing tenants
/// surface unchanged; a fresh `created=true` outcome here means the
/// operator skipped `/provision`, so we map that to
/// [`WorkspaceResolve::NotProvisioned`] rather than implicitly
/// bootstrapping.
pub(super) async fn resolve_workspace(
    store: &Arc<SessionStore>,
    email: &str,
) -> Result<(TenantId, WorkspaceId), WorkspaceResolve> {
    let user = match get_user_by_email(store, email).await {
        Ok(Some(u)) => u,
        Ok(None) => return Err(WorkspaceResolve::UserMissing),
        Err(e) => return Err(WorkspaceResolve::Internal(format!("get_user: {e}"))),
    };
    let outcome = match provision_for_user(
        store,
        ProvisionRequest {
            user_id: user.id,
            user_email: &user.email,
            workspace_name: workspace_name_fallback(&user),
            company_name: None,
        },
    )
    .await
    {
        Ok(o) => o,
        Err(ArgoError::Schema(msg)) => return Err(WorkspaceResolve::Internal(msg)),
        Err(e) => return Err(WorkspaceResolve::Internal(format!("provision: {e}"))),
    };
    if outcome.created {
        return Err(WorkspaceResolve::NotProvisioned);
    }
    Ok((outcome.tenant_id, outcome.workspace_id))
}

fn workspace_name_fallback(user: &UserAccount) -> &str {
    user.email.split('@').next().unwrap_or("workspace")
}

/// Extract the `human:<email>` payload out of the bearer-gated request.
/// Returns `None` when the header is missing or doesn't carry a human
/// principal — the caller maps that to 401.
pub(super) fn principal_email(req: &str) -> Option<String> {
    let headers = req
        .split_once("\r\n\r\n")
        .map_or(req, |(headers, _body)| headers);
    let from_header = headers
        .lines()
        .skip(1)
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case(PRINCIPAL_HEADER)
                .then(|| value.trim().to_string())
        })
        .and_then(|raw| raw.strip_prefix(HUMAN_PRINCIPAL_PREFIX).map(str::to_string))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && s.contains('@'));
    if from_header.is_some() {
        return from_header;
    }
    // Split-topology fallback: when the SPA is served from a separate origin
    // (App Runner) and proxies to the gateway, its simple rewrite forwards the
    // `__Host-session` cookie but does NOT stamp the `X-Argo-Principal` header.
    // Derive the human principal from the verified session JWT the gateway
    // already validates for page access (same HS256 signature; the `email`
    // claim is the verified human identity) — no weaker than the header, since
    // both ultimately trace to the gateway's own signed magic-link JWT.
    super::session_jwt::extract_session_claims(req)
        .map(|c| c.email.trim().to_string())
        .filter(|s| !s.is_empty() && s.contains('@'))
}

/// Best-effort provider inference from the API key prefix; falls back
/// to `"custom"` when nothing matches. Caller may override with an
/// explicit `provider` field on the request body.
pub(super) fn infer_provider(api_key: &str) -> String {
    if api_key.starts_with("sk-ant-") {
        "anthropic".into()
    } else if api_key.starts_with("sk-proj-") || api_key.starts_with("sk-") {
        "openai".into()
    } else if api_key.starts_with("AIza") {
        "google".into()
    } else {
        "custom".into()
    }
}

/// Shape-only validation of an API key against a provider. v1 ships
/// without the live one-token validation call; the wire shape mirrors
/// the eventual real-validation contract so a follow-up brief can
/// swap the body without changing the response.
pub(super) struct KeyValidation {
    pub(super) ok: bool,
    pub(super) reason: String,
    pub(super) test_cost_micros: i64,
    pub(super) test_tokens: i64,
}

pub(super) fn validate_key_shape(provider: &str, api_key: &str) -> KeyValidation {
    let trimmed = api_key.trim();
    if trimmed.len() < 16 {
        return KeyValidation {
            ok: false,
            reason: "api_key must be at least 16 chars".into(),
            test_cost_micros: 0,
            test_tokens: 0,
        };
    }
    let prefix_ok = match provider {
        "anthropic" => trimmed.starts_with("sk-ant-"),
        "openai" => trimmed.starts_with("sk-") || trimmed.starts_with("sk-proj-"),
        "google" => trimmed.starts_with("AIza"),
        // azure / custom: no prefix enforcement.
        _ => true,
    };
    if !prefix_ok {
        return KeyValidation {
            ok: false,
            reason: format!("api_key shape does not match provider '{provider}'"),
            test_cost_micros: 0,
            test_tokens: 0,
        };
    }
    KeyValidation {
        ok: true,
        reason: String::new(),
        test_cost_micros: 1,
        test_tokens: 1,
    }
}

/// Reduce an API key to a display-safe form (`sk-ant-...abc1`) the
/// UI can render in the audit + status surfaces.
pub(super) fn mask_key(api_key: &str) -> String {
    let trimmed = api_key.trim();
    let len = trimmed.len();
    if len <= 8 {
        return "***".into();
    }
    let prefix_end = trimmed
        .find('-')
        .map_or(4, |i| (i + 1).min(8).min(len.saturating_sub(4)));
    let prefix = &trimmed[..prefix_end];
    let suffix = &trimmed[len.saturating_sub(4)..];
    format!("{prefix}...{suffix}")
}

pub(super) fn micros_to_usd(micros: i64) -> f64 {
    #[allow(clippy::cast_precision_loss)]
    let v = micros as f64;
    v / 1_000_000.0
}

/// Wrap a secret with the **per-tenant** master key derived from the
/// workspace root via HKDF-SHA256.
///
/// The previous implementation used a process-global `OnceLock<MasterKey>`
/// with a `MasterKey::generate()` ephemeral fallback. That had two
/// production bugs surfaced by eval-credentials-cross-tenant-isolation
/// (round-9, PR #2080 §3 P0):
///
/// 1. **Data loss**: if env resolution fell through to `generate()`,
///    the random ephemeral key was lost on restart and every wrapped
///    credential became permanently unreadable.
/// 2. **Cross-tenant blast radius**: one master key for every tenant
///    meant any leak compromised the whole estate at once.
///
/// The new contract: `MasterKey::resolve_for_tenant` always derives
/// from the workspace-root key (fail-closed; no ephemeral fallback),
/// and caches the per-tenant `Arc<MasterKey>` so repeat wraps in the
/// same process pay HKDF cost at most once per tenant.
pub(super) fn wrap_secret(plaintext: &str, tenant: TenantId) -> ArgoResult<String> {
    let key = MasterKey::resolve_for_tenant(tenant)?;
    encrypt_value(plaintext, &key)
}

/// Load one vertical pack from the on-disk catalog. Returns `None`
/// when the id isn't present.
pub(super) fn load_vertical(id: &str) -> Option<LoadedVertical> {
    let dir = crate::handlers::vertical::resolve_verticals_dir();
    let (registry, _errors) = argo_vertical::load_dir(&dir).ok()?;
    registry.get(id).cloned()
}

pub(super) fn count_workflow_files(loaded: &LoadedVertical) -> i64 {
    count_files_with_suffix(&loaded.root.join("workflows"), "workflow.toml")
}

pub(super) fn count_skill_files(loaded: &LoadedVertical) -> i64 {
    count_files_with_suffix(&loaded.root.join("skills"), "skill.md")
}

/// True when the manifest's `[memory]` section declares a schema file
/// the runtime will validate writes against. The pick-vertical receipt
/// renders ✓ when this is true; ✗ otherwise.
pub(super) fn memory_schema_attached(loaded: &LoadedVertical) -> bool {
    loaded
        .manifest
        .memory
        .as_ref()
        .and_then(|m| m.schema.as_ref())
        .is_some_and(|s| !s.trim().is_empty())
}

/// Count of `[escalation].human_approval_required` entries — the
/// "approval gates" the wizard sub-copy promises. 0 is a valid value
/// (vertical opts out of approvals); the receipt still renders ✓ on
/// install but shows the count alongside.
pub(super) fn count_approval_gates(loaded: &LoadedVertical) -> i64 {
    let len = loaded
        .manifest
        .escalation
        .as_ref()
        .map(|e| e.human_approval_required.len())
        .unwrap_or(0);
    i64::try_from(len).unwrap_or(i64::MAX)
}

fn count_files_with_suffix(dir: &Path, suffix: &str) -> i64 {
    let Ok(read) = std::fs::read_dir(dir) else {
        return 0;
    };
    let mut count: i64 = 0;
    for entry in read.flatten() {
        if entry
            .file_name()
            .to_string_lossy()
            .ends_with(&format!(".{suffix}"))
        {
            count += 1;
        }
    }
    count
}

/// Build a voice-preamble string that names the worker and folds the
/// first sentence of the operator's brief into the system prompt.
/// Replaces the pre-fix 35-char templated preamble
/// (`"I'm a <vertical> style worker named <name>."`) with one that
/// actually mentions what the worker is supposed to do.
///
/// Truncates the brief excerpt at ~200 chars on a word boundary so the
/// preamble stays compact in the agent prompt; the full brief still
/// lives on `persona.bio` for the UI.
pub(super) fn voice_preamble_from_description(name: &str, description: &str) -> String {
    let trimmed = description.trim();
    let first_sentence = trimmed
        .split(['.', '\n'])
        .find(|s| !s.trim().is_empty())
        .map_or(trimmed, str::trim);
    let excerpt = truncate_on_word_boundary(first_sentence, 200);
    format!("You are {name}, an autonomous SKYCOT teammate. {excerpt}")
}

fn truncate_on_word_boundary(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        return s.to_string();
    }
    let mut cut = 0;
    for (idx, _) in s.char_indices().take(max_chars) {
        cut = idx;
    }
    let head = &s[..=cut];
    let trimmed = head
        .rsplit_once(|c: char| c.is_whitespace())
        .map_or(head, |(left, _)| left);
    format!("{}…", trimmed.trim_end_matches(|c: char| c.is_whitespace() || c == ','))
}

/// Derive a worker display name from the operator's description.
///
/// Resolution order (first match wins):
/// 1. An **explicit naming pattern** — `named X` / `called X` /
///    `name is X` / `I am X` / `my <noun> is X` (case-insensitive).
///    This is preferred so a prompt like *"A marketing intern named
///    Maya …"* resolves to `Maya`, not the leading article `A`
///    (UX-AUDIT-T22-20: the first-capitalised-token heuristic named
///    every demo hire after the wizard's own example article).
/// 2. The **first capitalised token** that isn't an article or a
///    single letter.
/// 3. `"Pioneer"` so the row always has a non-empty name (the persona
///    validator rejects empties).
///
/// The Genesis-driven first-hire path reuses this extractor as the
/// fallback when the SPA doesn't POST an explicit `name`, so the fix
/// propagates into that flow too.
pub(super) fn name_from_description(desc: &str) -> String {
    let trimmed = desc.trim();
    explicit_name_from_description(trimmed)
        .or_else(|| first_capitalized_name(trimmed))
        .unwrap_or_else(|| "Pioneer".into())
}

/// Match an explicit naming clause and return the named token, or
/// `None` when no clause is present (or the captured token is an
/// article / single letter).
fn explicit_name_from_description(desc: &str) -> Option<String> {
    // Compiled once; fail-safe to `None` rather than `expect` (no
    // `unwrap`/`expect` is permitted outside `#[cfg(test)]`). A regex
    // compile failure simply disables the explicit path so the
    // capitalised-token fallback still runs.
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    let re = RE
        .get_or_init(|| {
            Regex::new(r"(?i)\b(?:named|called|name is|i am|my \w+ is)\s+([A-Za-z][A-Za-z'\-]*)")
                .ok()
        })
        .as_ref()?;
    // `captures_iter` walks matches left-to-right; the first one whose
    // token survives `clean_name_token` wins. This lets a clause like
    // *"I am a marketing intern named Maya"* skip the article `a`
    // captured by `I am` and land on `Maya` from `named`.
    re.captures_iter(desc)
        .filter_map(|caps| caps.get(1).and_then(|m| clean_name_token(m.as_str())))
        .next()
}

/// First whitespace/comma/period-delimited token that starts with an
/// uppercase letter and survives the article / single-letter filter.
fn first_capitalized_name(desc: &str) -> Option<String> {
    desc.split(|c: char| c.is_whitespace() || c == ',' || c == '.')
        .filter(|t| {
            t.chars()
                .next()
                .is_some_and(|c| c.is_alphabetic() && c.is_uppercase())
        })
        .find_map(clean_name_token)
}

/// Strip non-name characters, then reject single-letter tokens and
/// articles (`a`, `an`, `the`, `my`, `your`, `our`). Returns the
/// cleaned token with its first letter upper-cased.
fn clean_name_token(raw: &str) -> Option<String> {
    let cleaned: String = raw
        .chars()
        .filter(|c| c.is_alphabetic() || *c == '\'' || *c == '-')
        .collect();
    if cleaned.chars().count() < 2 {
        return None;
    }
    const STOPWORDS: [&str; 6] = ["a", "an", "the", "my", "your", "our"];
    if STOPWORDS.contains(&cleaned.to_ascii_lowercase().as_str()) {
        return None;
    }
    let mut chars = cleaned.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().chain(chars).collect::<String>())
}

pub(super) async fn emit_audit(
    store: &Arc<SessionStore>,
    tenant: TenantId,
    req: &str,
    endpoint: &str,
    payload: Value,
) {
    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant.to_string());
    let mut body = payload;
    if let Some(obj) = body.as_object_mut() {
        obj.insert("endpoint".into(), Value::String(endpoint.into()));
    }
    let principal = principal_id_for_audit(req);
    if let Err(e) = sink
        .write(None, AuditKind::HttpRequest, body, principal)
        .await
    {
        tracing::warn!(
            target: "gateway::onboarding",
            error = %e,
            endpoint = %endpoint,
            tenant = %tenant,
            "onboarding audit write failed"
        );
    }
}

pub(super) fn unauthorized() -> (&'static str, String) {
    error_envelope(
        "401 Unauthorized",
        "unauthorized",
        "X-Argo-Principal header must carry a verified `human:<email>` value",
    )
}

pub(super) fn bad_request(message: &str) -> (&'static str, String) {
    error_envelope("400 Bad Request", "bad_request", message)
}

pub(super) fn not_found(message: &str) -> (&'static str, String) {
    error_envelope("404 Not Found", "not_found", message)
}

pub(super) fn precondition_failed_provision() -> (&'static str, String) {
    error_envelope(
        "412 Precondition Failed",
        "not_provisioned",
        "Workspace has not been provisioned yet; POST /api/v1/onboarding/provision first",
    )
}

pub(super) fn server_error(message: &str) -> (&'static str, String) {
    error_envelope("500 Internal Server Error", "internal_error", message)
}

pub(super) fn provider_validation_unconfigured() -> (&'static str, String) {
    error_envelope(
        "503 Service Unavailable",
        "provider_validation_unconfigured",
        "Provider validation isn't configured in this workspace. You can skip this step - your workers will run with the built-in mock provider until you add a real key later in Settings.",
    )
}

pub(super) fn provider_key_storage_error() -> (&'static str, String) {
    server_error("Provider key could not be stored. Try again later.")
}

pub(super) fn is_workspace_root_key_error(err: &ArgoError) -> bool {
    let message = err.to_string();
    message.contains("no workspace root key")
        || message.contains("SKYCOT_WORKSPACE_ROOT_KEY_BASE64")
        || message.contains("ARGO_CREDENTIALS_MASTER_KEY")
}

fn error_envelope(status: &'static str, code: &str, message: &str) -> (&'static str, String) {
    (
        status,
        json!({
            "error": {
                "code": code,
                "message": message,
            }
        })
        .to_string(),
    )
}

/// Resolve the `WorkspaceResolve` enum to the matching HTTP response.
/// Centralised so the three handlers stay symmetric.
pub(super) fn map_workspace_error(err: WorkspaceResolve) -> (&'static str, String) {
    match err {
        WorkspaceResolve::NotProvisioned => precondition_failed_provision(),
        WorkspaceResolve::UserMissing => unauthorized(),
        WorkspaceResolve::Internal(msg) => server_error(&msg),
    }
}

#[cfg(test)]
#[path = "routes_onboarding_helpers_name_tests.rs"]
mod name_tests;
