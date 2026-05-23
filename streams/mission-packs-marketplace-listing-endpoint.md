# mission-packs-marketplace-listing-endpoint

**Priority**: P2 (claim-3 + claim-4 marketplace surface — operator-visible pack catalog)
**Owner**: claude
**Type**: Gateway route + handler + integration test
**Size**: ~200-400 LOC across one new handler + one route registration + integration test
**Authored against**: origin/main `01bbe9b58` on 2026-05-24
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-packs-marketplace-listing-endpoint`

## Why

The evaluator audit `eval-vertical-pack-registry-coverage` (2026-05-23) F-1 row:

> 🟡 P2: `mission-packs-marketplace-listing-endpoint` — surface a pack index under `GET /api/v1/packs`.

`packs/` is the marketplace half of claim-4 ("Skills shared and learned. Library + learning loop, cross-tenant sharing with provenance + trust"). Today there are 10 packs in `worker.toml` schema with **zero gateway surface** — operators cannot browse, search, or list them through the SPA. The hire wizard's connector catalog ships at `web/lib/connectors/registry.ts`, but the equivalent pack catalog has no live source.

This brief adds the missing `GET /api/v1/packs` endpoint: a paginated list of available packs (cached + repo) with pack metadata (id, version, vertical_id, author, description, last_updated). The endpoint is read-only and is the seam the SPA's `/marketplace/packs` page will consume in a follow-up brief.

## What

### Step 1 — handler

New file `crates/argo-cli/src/handlers/gateway/routes_packs.rs` (~120-200 LOC):

```rust
//! GET /api/v1/packs — list available packs from the marketplace.
//!
//! Source of truth: PackLoader (~/.argo/packs cache) + repo `packs/` dir
//! merged into a single catalog. Tenant-scoped: only packs the
//! caller's workspace can install (today: all packs, since the marketplace
//! is global; future: workspace-scoped private packs filter in here).

#[derive(Serialize)]
pub struct PackListResponse {
    pub packs: Vec<PackEntry>,
    pub total: usize,
}

#[derive(Serialize)]
pub struct PackEntry {
    pub id: String,
    pub version: String,
    pub vertical_id: Option<String>,
    pub author: Option<String>,
    pub description: Option<String>,
    pub installed: bool,         // is it in ~/.argo/packs?
    pub source: PackSource,      // Cache | Repo
}

#[derive(Serialize)]
pub enum PackSource { Cache, Repo }

pub async fn list_packs(
    State(deps): State<AgentDeps>,
    principal: Principal,
) -> Result<Json<PackListResponse>, ApiError> {
    let _tenant = principal.tenant_id();
    let loader = PackLoader::new_from_deps(&deps);

    let cache = loader.list()?;
    let repo = read_repo_packs_dir(&deps.repo_root)?;
    let merged = merge_catalog(cache, repo);

    Ok(Json(PackListResponse { total: merged.len(), packs: merged }))
}
```

### Step 2 — route registration

Wire `list_packs` into the gateway router. The router lives at `crates/argo-cli/src/handlers/gateway/mod.rs` (or similar — confirm at claim time). Add:

```rust
.route("/api/v1/packs", get(routes_packs::list_packs))
```

Per MISSION.md invariant #3 (capability-based security with first-class RBAC): the `Principal` extractor handles auth. Tenant-scoping today is identity-only because the marketplace is global; the handler must call `principal.tenant_id()` even if the value isn't used in the filter — so the next reviewer can see the seam.

### Step 3 — integration test

New file `crates/argo-cli/tests/gateway_routes_packs_test.rs` (~80-150 LOC, ~3 tests):

1. **Happy path** — boot a test gateway with 2 packs in `repo_root/packs/` and 1 pack installed in `~/.argo/packs`. GET `/api/v1/packs`. Assert response is `200`, `total=3`, returns one entry per pack with correct `source` field.
2. **Auth** — GET `/api/v1/packs` without `X-Argo-Principal`; assert `401`.
3. **Empty catalog** — boot gateway with no packs; assert `200` + `{packs:[], total:0}` (not an error).

## Acceptance

- [ ] `crates/argo-cli/src/handlers/gateway/routes_packs.rs` exists and exposes `list_packs`.
- [ ] `GET /api/v1/packs` is reachable through the live gateway.
- [ ] All 3 integration tests pass.
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean.
- [ ] Response shape documented in `docs/skycot/api/packs.md` (or extend an existing API doc).
- [ ] No production code in `crates/argo-vertical-packs/src/*.rs` is touched — the handler is a consumer of `PackLoader`, not a contributor.
- [ ] Handler logs `principal.tenant_id()` at debug level for traceability (does NOT filter by it today, but the seam is visible).

## May-touch

- `crates/argo-cli/src/handlers/gateway/routes_packs.rs` (NEW)
- `crates/argo-cli/src/handlers/gateway/mod.rs` (add route registration)
- `crates/argo-cli/tests/gateway_routes_packs_test.rs` (NEW)
- `docs/skycot/api/packs.md` (NEW or extend)

**Out of may-touch**: `crates/argo-vertical-packs/src/*.rs` (use the public `PackLoader` API only), other gateway routes, the SPA. The SPA consumer is a follow-up brief.

## Parallel surfaces

- `rg 'list_packs|packs_endpoint|GET.*packs' crates/argo-cli/src/` returns no production hits — net-new route.
- `crates/argo-cli/src/handlers/packs.rs` (CLI handler, not gateway) already exists at line 128 of `packs.rs` for the `argo packs` CLI command. This brief is the **gateway** sibling — different layer, different function name. Do not consolidate.
- `crates/argo-cli/src/handlers/gateway/routes_connectors.rs` (or similar — confirm at claim time) is the connectors-list endpoint. This brief's shape mirrors it 1:1 (read-only list, paginated, tenant-aware seam).

## Sensible deviations

- If `PackLoader` doesn't have a public `list_all` method that returns the cache+repo merged catalog, the handler builds the merge inline (no new pub method in `argo-vertical-packs`). Keep the adapter brief separate.
- If the gateway uses Axum vs. Actix vs. another framework, follow the existing route registration idiom — don't introduce a parallel pattern.
- If `Principal::tenant_id()` is not yet available on the Principal extractor (legacy gap), the handler still authenticates via the extractor but logs the absence as `tenant_unknown`. The follow-up tenant-scoping brief picks it up.
- If pagination is needed at >50 packs, accept `?limit=` + `?offset=` query params with sane defaults (limit=50, offset=0). The catalog is small today; pagination is YAGNI but the seam is one line.
- If repo `packs/` discovery requires walking the filesystem and that's slow at boot, cache the catalog in-memory on first call and invalidate on the next `PackLoader::install` write. Document the staleness window in the handler doc-comment.
