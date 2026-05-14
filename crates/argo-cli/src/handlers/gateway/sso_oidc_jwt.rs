//! OIDC ID-token signature verification helpers.
use argo_config::SsoOidcTenant;
use jsonwebtoken::jwk::JwkSet;
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::sync::OnceLock;
use tokio::sync::Mutex;

const CLOCK_SKEW_SECS: i64 = 60;
const DEFAULT_JWKS_CACHE_TTL_SECS: u64 = 3600;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct IdTokenClaims {
    pub sub: String,
    pub email: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum VerifyError {
    Malformed,
    MissingKid,
    UnsupportedAlg,
    SignatureRejected,
    IssuerRejected,
    AudRejected,
    Expired,
    NonceRejected,
}

impl VerifyError {
    pub(super) const fn reason(self) -> &'static str {
        match self {
            Self::Malformed => "malformed",
            Self::MissingKid => "missing_kid",
            Self::UnsupportedAlg => "unsupported_alg",
            Self::SignatureRejected => "signature_rejected",
            Self::IssuerRejected => "issuer_rejected",
            Self::AudRejected => "aud_rejected",
            Self::Expired => "expired",
            Self::NonceRejected => "nonce_rejected",
        }
    }
}

#[derive(Clone)]
struct CachedJwks {
    fetched_at: i64,
    set: JwkSet,
}

#[derive(Debug, Deserialize)]
struct RawClaims {
    sub: Option<String>,
    email: Option<String>,
    iss: Option<String>,
    aud: Option<Audience>,
    exp: Option<i64>,
    iat: Option<i64>,
    nonce: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Audience {
    One(String),
    Many(Vec<String>),
}

impl Audience {
    fn contains(&self, expected: &str) -> bool {
        match self {
            Self::One(aud) => aud == expected,
            Self::Many(auds) => auds.iter().any(|aud| aud == expected),
        }
    }
}

type Cache = Mutex<BTreeMap<String, CachedJwks>>;

fn jwks_cache() -> &'static Cache {
    static CACHE: OnceLock<Cache> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(BTreeMap::new()))
}

#[cfg(test)]
pub(super) async fn seed_jwks(uri: &str, fetched_at: i64, set: JwkSet) {
    jwks_cache()
        .lock()
        .await
        .insert(uri.to_string(), CachedJwks { fetched_at, set });
}

pub(super) async fn verify_id_token_with_cache(
    token: &str,
    tenant: &SsoOidcTenant,
    now: i64,
    nonce_expected: &str,
) -> Result<IdTokenClaims, VerifyError> {
    let set = fetch_cached_jwks(tenant, now, false)
        .await
        .map_err(|_| VerifyError::SignatureRejected)?;
    match verify_and_parse_id_token(token, &set, tenant, now, nonce_expected) {
        Err(VerifyError::SignatureRejected | VerifyError::MissingKid) => {
            let refreshed = fetch_cached_jwks(tenant, now, true)
                .await
                .map_err(|_| VerifyError::SignatureRejected)?;
            verify_and_parse_id_token(token, &refreshed, tenant, now, nonce_expected)
        }
        other => other,
    }
}

async fn fetch_cached_jwks(
    tenant: &SsoOidcTenant,
    now: i64,
    force_refresh: bool,
) -> Result<JwkSet, String> {
    if tenant.jwks_uri.is_empty() {
        return Err("jwks_uri not configured".into());
    }
    let ttl = if tenant.jwks_cache_ttl_secs == 0 {
        DEFAULT_JWKS_CACHE_TTL_SECS
    } else {
        tenant.jwks_cache_ttl_secs
    };
    if !force_refresh {
        let cache = jwks_cache().lock().await;
        if let Some(cached) = cache.get(&tenant.jwks_uri) {
            if now.saturating_sub(cached.fetched_at) <= i64::try_from(ttl).unwrap_or(3600) {
                return Ok(cached.set.clone());
            }
        }
    }
    let set = fetch_jwks(&tenant.jwks_uri).await?;
    let mut cache = jwks_cache().lock().await;
    cache.insert(
        tenant.jwks_uri.clone(),
        CachedJwks {
            fetched_at: now,
            set: set.clone(),
        },
    );
    Ok(set)
}

async fn fetch_jwks(uri: &str) -> Result<JwkSet, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("build client: {e}"))?;
    let resp = client
        .get(uri)
        .send()
        .await
        .map_err(|e| format!("send: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("idp jwks status {}", resp.status()));
    }
    resp.json::<JwkSet>()
        .await
        .map_err(|e| format!("parse jwks: {e}"))
}

pub(super) fn verify_and_parse_id_token(
    token: &str,
    jwks: &JwkSet,
    tenant: &SsoOidcTenant,
    now: i64,
    nonce_expected: &str,
) -> Result<IdTokenClaims, VerifyError> {
    let header = decode_header(token).map_err(|_| VerifyError::Malformed)?;
    let kid = header.kid.as_deref().ok_or(VerifyError::MissingKid)?;
    let alg = match header.alg {
        Algorithm::RS256 | Algorithm::ES256 => header.alg,
        _ => return Err(VerifyError::UnsupportedAlg),
    };
    let jwk = jwks.find(kid).ok_or(VerifyError::MissingKid)?;
    let mut validation = Validation::new(alg);
    validation.validate_exp = false;
    validation.validate_aud = false;
    validation.set_required_spec_claims::<&str>(&[]);
    let data = decode::<RawClaims>(
        token,
        &DecodingKey::from_jwk(jwk).map_err(|_| VerifyError::SignatureRejected)?,
        &validation,
    )
    .map_err(|_| VerifyError::SignatureRejected)?;
    validate_claims(data.claims, tenant, now, nonce_expected)
}

fn validate_claims(
    claims: RawClaims,
    tenant: &SsoOidcTenant,
    now: i64,
    nonce_expected: &str,
) -> Result<IdTokenClaims, VerifyError> {
    if tenant.issuer.is_empty() || claims.iss.as_deref() != Some(tenant.issuer.as_str()) {
        return Err(VerifyError::IssuerRejected);
    }
    if !claims
        .aud
        .as_ref()
        .map(|aud| aud.contains(&tenant.client_id))
        .unwrap_or(false)
    {
        return Err(VerifyError::AudRejected);
    }
    if claims
        .exp
        .map(|exp| exp + CLOCK_SKEW_SECS < now)
        .unwrap_or(true)
        || claims
            .iat
            .map(|iat| iat - CLOCK_SKEW_SECS > now)
            .unwrap_or(true)
    {
        return Err(VerifyError::Expired);
    }
    if claims.nonce.as_deref() != Some(nonce_expected) {
        return Err(VerifyError::NonceRejected);
    }
    Ok(IdTokenClaims {
        sub: claims.sub.ok_or(VerifyError::Malformed)?,
        email: claims.email.unwrap_or_default(),
    })
}
