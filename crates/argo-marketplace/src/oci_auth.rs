//! OCI Distribution Spec auth-challenge helpers.
//!
//! When a registry replies `401 Unauthorized` to a v2 request,
//! it includes a `WWW-Authenticate: Bearer realm="…",service="…"
//! ,scope="…"` header. The client is expected to:
//!
//! 1. Parse `realm`, `service`, `scope` out of the header.
//! 2. `GET <realm>?service=…&scope=…` with HTTP Basic auth (the
//!    user's registry credentials — for GHCR a Personal Access
//!    Token, for Docker Hub username + access token).
//! 3. Receive `{"token": "…"}`.
//! 4. Retry the original request with `Authorization: Bearer
//!    <token>`.
//!
//! This module owns steps 1–3; the `OciDistributionRegistry`
//! does the retry. Pure functions where possible so the parser
//! is unit-testable without a network.
use base64::Engine as _;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Resolved registry credentials after applying Argo's fallback chain.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ResolvedRegistryAuth {
    /// Token supplied by `--token` or credentials.toml. The OCI client
    /// uses it as the password side of Basic auth for bearer-challenge
    /// registries such as GHCR.
    pub token: Option<String>,
    /// Username/password pair from Docker config.
    pub basic: Option<(String, String)>,
}

impl ResolvedRegistryAuth {
    /// Anonymous registry access.
    #[must_use]
    pub fn anonymous() -> Self {
        Self::default()
    }

    /// Token-based registry access.
    #[must_use]
    pub fn token(token: impl Into<String>) -> Self {
        Self {
            token: Some(token.into()),
            basic: None,
        }
    }

    /// Basic-auth registry access.
    #[must_use]
    pub fn basic(user: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            token: None,
            basic: Some((user.into(), password.into())),
        }
    }
}

/// Resolve registry auth using `--token > credentials.toml >
/// Docker config > anonymous`.
#[must_use]
pub fn resolve_registry_auth(host: &str, explicit_token: Option<&str>) -> ResolvedRegistryAuth {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let docker = docker_config_path();
    resolve_registry_auth_from(host, explicit_token, &cwd, docker.as_deref())
}

/// Testable resolver with explicit workspace + Docker config paths.
#[must_use]
pub fn resolve_registry_auth_from(
    host: &str,
    explicit_token: Option<&str>,
    workspace: &Path,
    docker_config: Option<&Path>,
) -> ResolvedRegistryAuth {
    if let Some(token) = explicit_token.and_then(nonempty) {
        return ResolvedRegistryAuth::token(token);
    }
    if let Some(token) = credentials_token(host, workspace) {
        return ResolvedRegistryAuth::token(token);
    }
    docker_config
        .and_then(|path| docker_auth(host, path))
        .unwrap_or_else(ResolvedRegistryAuth::anonymous)
}

fn nonempty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn credentials_token(host: &str, workspace: &Path) -> Option<String> {
    let host_key = credential_token_key(host);
    argo_config::cred::get_from_workspace(workspace, &host_key)
        .or_else(|_| argo_config::cred::get_from_workspace(workspace, "ARGO_OCI_TOKEN"))
        .ok()
        .and_then(|v| nonempty(&v))
}

fn credential_token_key(host: &str) -> String {
    let mut key = String::from("ARGO_OCI_TOKEN_");
    for c in host.chars() {
        if c.is_ascii_alphanumeric() {
            key.push(c.to_ascii_uppercase());
        } else {
            key.push('_');
        }
    }
    key
}

fn docker_config_path() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("DOCKER_CONFIG").filter(|v| !v.is_empty()) {
        return Some(PathBuf::from(dir).join("config.json"));
    }
    std::env::var_os("HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .map(|home| home.join(".docker").join("config.json"))
}

#[derive(Debug, Deserialize)]
struct DockerConfig {
    #[serde(default)]
    auths: BTreeMap<String, DockerAuthEntry>,
}

#[derive(Debug, Deserialize)]
struct DockerAuthEntry {
    auth: Option<String>,
    username: Option<String>,
    password: Option<String>,
    identitytoken: Option<String>,
}

fn docker_auth(host: &str, path: &Path) -> Option<ResolvedRegistryAuth> {
    let body = std::fs::read_to_string(path).ok()?;
    let config: DockerConfig = serde_json::from_str(&body).ok()?;
    docker_auth_entry(host, &config.auths).and_then(auth_from_entry)
}

fn docker_auth_entry<'a>(
    host: &str,
    auths: &'a BTreeMap<String, DockerAuthEntry>,
) -> Option<&'a DockerAuthEntry> {
    for key in [
        host.to_string(),
        format!("https://{host}"),
        format!("http://{host}"),
    ] {
        if let Some(entry) = auths.get(&key) {
            return Some(entry);
        }
    }
    None
}

fn auth_from_entry(entry: &DockerAuthEntry) -> Option<ResolvedRegistryAuth> {
    if let Some(token) = entry.identitytoken.as_deref().and_then(nonempty) {
        return Some(ResolvedRegistryAuth::token(token));
    }
    if let (Some(user), Some(password)) = (
        entry.username.as_deref().and_then(nonempty),
        entry.password.as_deref().and_then(nonempty),
    ) {
        return Some(ResolvedRegistryAuth::basic(user, password));
    }
    let raw = entry.auth.as_deref().and_then(nonempty)?;
    let decoded = base64::engine::general_purpose::STANDARD.decode(raw).ok()?;
    let pair = String::from_utf8(decoded).ok()?;
    let (user, password) = pair.split_once(':')?;
    Some(ResolvedRegistryAuth::basic(user, password))
}

/// Parsed `WWW-Authenticate: Bearer …` parameters. Only the
/// fields Argo needs; extra params (e.g. `error_description`)
/// are ignored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BearerChallenge {
    /// Token endpoint URL — caller GETs this with `service` +
    /// `scope` query params + HTTP Basic auth.
    pub realm: String,
    /// Registry service name (e.g. `ghcr.io`,
    /// `registry.docker.io`).
    pub service: Option<String>,
    /// Permission scope the token should grant (e.g.
    /// `repository:foo/bar:pull,push`).
    pub scope: Option<String>,
}

impl BearerChallenge {
    /// Parse a `WWW-Authenticate` header value. Returns `None`
    /// when the value isn't a `Bearer …` challenge.
    #[must_use]
    pub fn parse(header_value: &str) -> Option<Self> {
        let trimmed = header_value.trim();
        let rest = trimmed.strip_prefix("Bearer ")?.trim();
        let mut realm = None;
        let mut service = None;
        let mut scope = None;
        for kv in split_params(rest) {
            let (k, v) = kv;
            match k {
                "realm" => realm = Some(v.to_string()),
                "service" => service = Some(v.to_string()),
                "scope" => scope = Some(v.to_string()),
                _ => {}
            }
        }
        let realm = realm?;
        Some(Self {
            realm,
            service,
            scope,
        })
    }

    /// Build the token-endpoint URL: `<realm>?service=…&scope=…`.
    #[must_use]
    pub fn token_url(&self) -> String {
        let mut url = self.realm.clone();
        let mut params: Vec<String> = Vec::new();
        if let Some(s) = self.service.as_deref() {
            params.push(format!("service={s}"));
        }
        if let Some(s) = self.scope.as_deref() {
            params.push(format!("scope={s}"));
        }
        if !params.is_empty() {
            url.push('?');
            url.push_str(&params.join("&"));
        }
        url
    }
}

/// Split `key1="v1",key2="v2",key3=v3` into `[(key, value), …]`.
/// Quoted values keep escaping minimal — backslash-escapes are
/// not handled (the registry spec doesn't use them in practice).
fn split_params(s: &str) -> Vec<(&str, &str)> {
    let mut out = Vec::new();
    let mut rest = s;
    while !rest.is_empty() {
        let Some(eq_idx) = rest.find('=') else {
            break;
        };
        let key = rest[..eq_idx].trim();
        let after_eq = &rest[eq_idx + 1..];
        let (value, advance) = if let Some(stripped) = after_eq.strip_prefix('"') {
            // Quoted value — find the closing quote.
            let close = stripped.find('"').unwrap_or(stripped.len());
            (&stripped[..close], eq_idx + 1 + 1 + close + 1) // +1 open quote, +1 close quote
        } else {
            // Bare value — until next comma or end.
            let end = after_eq.find(',').unwrap_or(after_eq.len());
            (after_eq[..end].trim(), eq_idx + 1 + end)
        };
        out.push((key, value));
        rest = if advance >= rest.len() {
            ""
        } else {
            rest[advance..].trim_start_matches([',', ' '])
        };
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ghcr_style_challenge() {
        let h = r#"Bearer realm="https://ghcr.io/token",service="ghcr.io",scope="repository:argo/x:pull,push""#;
        let c = BearerChallenge::parse(h).unwrap();
        assert_eq!(c.realm, "https://ghcr.io/token");
        assert_eq!(c.service.as_deref(), Some("ghcr.io"));
        assert_eq!(c.scope.as_deref(), Some("repository:argo/x:pull,push"));
    }

    #[test]
    fn parses_docker_hub_style_challenge() {
        let h = r#"Bearer realm="https://auth.docker.io/token",service="registry.docker.io""#;
        let c = BearerChallenge::parse(h).unwrap();
        assert_eq!(c.realm, "https://auth.docker.io/token");
        assert_eq!(c.service.as_deref(), Some("registry.docker.io"));
        assert!(c.scope.is_none());
    }

    #[test]
    fn rejects_non_bearer_challenges() {
        assert!(BearerChallenge::parse("Basic realm=\"foo\"").is_none());
    }

    #[test]
    fn token_url_appends_query_params() {
        let c = BearerChallenge {
            realm: "https://ghcr.io/token".into(),
            service: Some("ghcr.io".into()),
            scope: Some("repository:argo/x:pull".into()),
        };
        assert_eq!(
            c.token_url(),
            "https://ghcr.io/token?service=ghcr.io&scope=repository:argo/x:pull"
        );
    }

    #[test]
    fn token_url_handles_realm_only_challenge() {
        let c = BearerChallenge {
            realm: "https://example/token".into(),
            service: None,
            scope: None,
        };
        assert_eq!(c.token_url(), "https://example/token");
    }

    #[test]
    fn explicit_token_wins_over_credentials_and_docker_config() {
        let dir = tempfile::tempdir().unwrap();
        let docker = dir.path().join("config.json");
        std::fs::write(&docker, r#"{"auths":{"ghcr.io":{"auth":"dXNlcjpwYXNz"}}}"#).unwrap();
        argo_config::cred::set(dir.path(), "ARGO_OCI_TOKEN_GHCR_IO", "stored").unwrap();
        let auth =
            resolve_registry_auth_from("ghcr.io", Some("explicit"), dir.path(), Some(&docker));
        assert_eq!(auth, ResolvedRegistryAuth::token("explicit"));
    }

    #[test]
    fn credentials_toml_host_token_wins_over_global_token() {
        let dir = tempfile::tempdir().unwrap();
        argo_config::cred::set(dir.path(), "ARGO_OCI_TOKEN", "global").unwrap();
        argo_config::cred::set(dir.path(), "ARGO_OCI_TOKEN_TTL_SH", "host").unwrap();
        let auth = resolve_registry_auth_from("ttl.sh", None, dir.path(), None);
        assert_eq!(auth, ResolvedRegistryAuth::token("host"));
    }

    #[test]
    fn docker_config_auth_decodes_basic_pair() {
        let dir = tempfile::tempdir().unwrap();
        let docker = dir.path().join("config.json");
        std::fs::write(
            &docker,
            r#"{"auths":{"https://ghcr.io":{"auth":"dXNlcjpwYXNz"}}}"#,
        )
        .unwrap();
        let auth = resolve_registry_auth_from("ghcr.io", None, dir.path(), Some(&docker));
        assert_eq!(auth, ResolvedRegistryAuth::basic("user", "pass"));
    }

    #[test]
    fn docker_config_identity_token_maps_to_token_auth() {
        let dir = tempfile::tempdir().unwrap();
        let docker = dir.path().join("config.json");
        std::fs::write(
            &docker,
            r#"{"auths":{"registry.local:5000":{"identitytoken":"id-token"}}}"#,
        )
        .unwrap();
        let auth =
            resolve_registry_auth_from("registry.local:5000", None, dir.path(), Some(&docker));
        assert_eq!(auth, ResolvedRegistryAuth::token("id-token"));
    }

    #[test]
    fn missing_credentials_falls_back_to_anonymous() {
        let dir = tempfile::tempdir().unwrap();
        let auth = resolve_registry_auth_from("ttl.sh", None, dir.path(), None);
        assert_eq!(auth, ResolvedRegistryAuth::anonymous());
    }
}
