//! API-key roster seeder for the canonical v1 demo.
//!
//! `/settings/api-keys` reads the first-class `api_key` store
//! (`GET /api/v1/api-keys` → `routes_api_keys::list_response`, migration
//! v86). A demo tenant with zero keys renders the page's empty state —
//! cosmetically wrong for a workspace the demo presents as an operating
//! AI workforce with live integrations.
//!
//! This module seeds 2-3 **metadata-only** key rows ("Production CI",
//! "Staging dashboard", "Analytics read-only") so the roster paints real
//! cards: a name, a masked prefix, scopes, created/last-used metadata.
//!
//! NO THEATER / secret discipline: these are NOT working credentials.
//! Each row's `token_hash` is a random throwaway digest minted at seed
//! time — there is no corresponding raw token anywhere (it was never
//! generated), so nothing can authenticate as a seeded key, and no
//! plaintext secret is stored. The masked `prefix` is illustrative
//! (`sk_live_••••...••••`-style), not derived from any real secret. The
//! seed deliberately does NOT write a `service_token_binding` row (that
//! is what would make a key authenticate); only the create *endpoint*
//! does that, with a real minted token returned once to the operator.
//!
//! Idempotent: if the tenant already has any `api_key` row the whole
//! block is skipped, so re-seeding never duplicates the roster.

use argo_store::api_key::{self, ApiKeyInsert};
use argo_store::SessionStore;
use argo_types::TenantId;
use std::sync::Arc;

use crate::SeedResult;

/// One seeded key's display metadata. `scopes` is a JSON array literal;
/// `days_since_use` drives a believable `last_used_at`.
struct DemoKey {
    name: &'static str,
    scopes_json: &'static str,
    all_cohorts: bool,
    /// Days before "now" the key was created.
    created_days_ago: i64,
    /// Days before "now" the key was last used, or `None` if never used.
    used_days_ago: Option<i64>,
}

const DEMO_KEYS: &[DemoKey] = &[
    DemoKey {
        name: "Production CI",
        scopes_json: "[\"read:workers\",\"run:workflows\",\"read:audit\"]",
        all_cohorts: true,
        created_days_ago: 47,
        used_days_ago: Some(0),
    },
    DemoKey {
        name: "Staging dashboard",
        scopes_json: "[\"read:workers\",\"read:workflows\",\"read:skills\"]",
        all_cohorts: true,
        created_days_ago: 21,
        used_days_ago: Some(2),
    },
    DemoKey {
        name: "Analytics read-only",
        scopes_json: "[\"read:audit\",\"read:workflows\"]",
        all_cohorts: false,
        created_days_ago: 9,
        used_days_ago: None,
    },
];

/// A throwaway 64-hex-char `token_hash` with NO corresponding raw token.
/// Two concatenated UUID-v7 simple forms (32 hex chars each) give a
/// SHA-256-shaped, per-row-unique value so the `token_hash UNIQUE`
/// constraint is satisfied without any real secret ever existing — there
/// is no token that hashes to this, by construction.
fn throwaway_hash() -> String {
    format!(
        "{}{}",
        uuid::Uuid::now_v7().simple(),
        uuid::Uuid::now_v7().simple()
    )
}

/// Insert the canonical demo API-key roster (metadata only) under
/// `tenant_id`. Returns the number of rows inserted (`3` on first run,
/// `0` on re-seed). Idempotent: a tenant that already has any key is
/// skipped wholesale.
pub(crate) async fn seed_api_keys(
    store: &Arc<SessionStore>,
    tenant_id: TenantId,
) -> SeedResult<usize> {
    let pool = store.pool();
    let tenant = tenant_id.to_string();
    if !api_key::list_for_tenant(pool, &tenant).await?.is_empty() {
        return Ok(0);
    }
    let now = chrono::Utc::now();
    let mut inserted = 0usize;
    for key in DEMO_KEYS {
        let id = format!("key_{}", uuid::Uuid::now_v7());
        let hash = throwaway_hash();
        // Illustrative masked preview — NOT derived from any secret. The
        // head/tail come from the unique throwaway hash so each row's
        // preview differs, the way a real masked prefix would.
        let head = &hash[..4];
        let tail = &hash[hash.len() - 4..];
        let prefix = format!("sk_live_{head}...{tail}");
        let created_at = (now - chrono::Duration::days(key.created_days_ago)).to_rfc3339();
        let last_used = key
            .used_days_ago
            .map(|d| (now - chrono::Duration::days(d)).to_rfc3339());
        api_key::insert(
            pool,
            &ApiKeyInsert {
                id: &id,
                tenant_id: &tenant,
                name: key.name,
                token_hash: &hash,
                prefix: &prefix,
                scopes_json: key.scopes_json,
                cohort_ids_json: "[]",
                all_cohorts: key.all_cohorts,
                created_at: &created_at,
                expires_at: None,
            },
        )
        .await?;
        if let Some(used_at) = last_used {
            // Stamp a believable last-used time on the freshly-seeded row.
            api_key::touch_last_used(pool, &hash, &used_at).await?;
        }
        inserted += 1;
    }
    Ok(inserted)
}

#[cfg(test)]
#[path = "seed_api_keys_tests.rs"]
mod tests;
