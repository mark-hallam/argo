# Mission load-bearing wire — replace hardcoded `MANIFEST_INDEX` with OCI catalog scan

**Owner**: claude. **Depends on**: existing `MarketplaceSkill` + `search()` surface (verified at `crates/argo-skill/src/marketplace.rs:82`); hardcoded `MANIFEST_INDEX` const (verified at `marketplace.rs:211`); `oci-distribution` crate likely already in workspace (verify in first 5min via `cargo tree -p argo-skill | grep -i oci`).
**Authored against**: origin/main `e44f6e8a` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-marketplace-oci-registry-scan`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 4 #71 — "Replace hardcoded `MANIFEST_INDEX` with real OCI registry catalog scan." Today `marketplace::search` walks a static slice of 5-10 baked-in skills. Mission promise: `argo skill catalog` returns LIVE results from a configured OCI registry. The hardcoded list becomes a fallback for offline / dev mode only.

## Scope

1. **NEW**: `MarketplaceSource` trait in `argo-skill::marketplace`:
   ```rust
   #[async_trait::async_trait]
   pub trait MarketplaceSource: Send + Sync {
       async fn list_all(&self) -> Result<Vec<MarketplaceSkill>, MarketplaceError>;
   }
   ```
2. **Two impls**:
   - `StaticIndex` — wraps the existing `MANIFEST_INDEX`. Renames the existing internal const but keeps the data table byte-identical for back-compat.
   - `OciRegistryScan` — `new(registry_url: String, namespace: String)`. `list_all()` calls the OCI `_catalog` endpoint (HTTP GET `<registry>/v2/_catalog`) + per-repo `<registry>/v2/<repo>/tags/list`. Hydrates each into `MarketplaceSkill { id, name, description, trust_grade, ... }` from the manifest annotations (`org.opencontainers.image.title`, `.description`, `org.skycot.trust_grade`).

3. **`MarketplaceClient` composition** (NEW): owns `Vec<Box<dyn MarketplaceSource>>`. `search()` becomes a method that calls `list_all` on each source, merges, dedupes by `id`, and applies the existing filter logic (string match, trust grade floor).

4. **Configuration**: read `~/.argo/marketplace.toml` (NEW, optional) for registry list:
   ```toml
   [[registry]]
   url = "https://skycot.cr"
   namespace = "skills"
   ```
   If file missing → use only `StaticIndex` (back-compat preserved). One copy-paste example in rustdoc.

5. **CLI wire**: `argo skill catalog` (existing handler — narrowly scoped change: instantiate `MarketplaceClient` with configured sources + default-static, then call `client.search(...)`). NO change to flags or output format.

6. **Offline resilience**: if `OciRegistryScan::list_all` errors (network/auth/404), log `tracing::warn!` with the source URL, EXCLUDE that source from the merge, continue with remaining sources. Empty merged result is normal in airgapped mode.

This is NOT in scope: write-side (publishing); cosign verification (`mission-skill-publish-verified-ingest` precursor); pull-and-cache (separate brief); auth flows (anonymous-only v1); pagination beyond first page (TODO note).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-skill/src/marketplace.rs` — narrowly scoped: rename internal `MANIFEST_INDEX` → `STATIC_INDEX_DATA`, ADD `MarketplaceSource` trait + `StaticIndex` impl + `MarketplaceClient`. Verify it stays < 500 LOC; spill to sibling if needed.
- `crates/argo-skill/src/marketplace_oci.rs` (NEW, ≤ 250 LOC) — `OciRegistryScan` impl + HTTP client (use `reqwest` since already a workspace dep — verify).
- `crates/argo-skill/src/marketplace_tests.rs` — narrowly scoped: ADD 5 NEW unit tests: trait-impl-static-roundtrip; client-merges-two-sources-dedupes-by-id; client-falls-back-when-source-errors; toml-parse-empty-file; toml-parse-multi-registry. Use a mock HTTP server (`mockito` or `wiremock`, whichever's in workspace).
- `crates/argo-skill/Cargo.toml` — narrowly scoped: add `reqwest` (verify) + `mockito` (dev-dep, verify).
- `crates/argo-cli/src/handlers/skill_catalog.rs` — narrowly scoped: instantiate `MarketplaceClient` instead of calling `search()` directly. ≤ 30 LOC delta.

## Files you must NOT touch

- `crates/argo-skill/src/store.rs` — store API unchanged
- Any non-skill/non-cli crate
- `~/.argo/` files — operators populate `marketplace.toml` themselves; brief just adds parse path

## Acceptance criteria

- [ ] `MarketplaceSource` trait + `StaticIndex` + `OciRegistryScan` + `MarketplaceClient` exist
- [ ] `STATIC_INDEX_DATA` table byte-identical to old `MANIFEST_INDEX` (back-compat)
- [ ] `argo skill catalog` still works with NO `marketplace.toml` (static-only path)
- [ ] `argo skill catalog` with `marketplace.toml` present queries each registry's `/v2/_catalog`
- [ ] Network error on one source → warn + continue, doesn't fail the command
- [ ] 5 new unit tests + mock HTTP server for OCI path
- [ ] No new public API outside the listed exports
- [ ] `cargo build -p argo-skill -p argo-cli` clean
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-skill -p argo-cli` passes (skip pre-existing flakes)
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-marketplace-oci-registry-scan`
- PR title: `skill: MarketplaceSource trait + OCI registry catalog scan (mission-marketplace-oci-registry-scan)`

## Notes for the implementer

- The static-index back-compat is **load-bearing**: existing tests assert exact catalog contents (e.g. `argo skill catalog` lists "kpi-tracker", "receptionist-router", etc.). Preserve byte-identical via the rename + a wrapper, NOT a rewrite.
- For the OCI HTTP shape: `GET /v2/_catalog?n=100` returns `{"repositories": [...]}`. Per-repo manifest fetch is `GET /v2/<repo>/manifests/<tag>` with `Accept: application/vnd.oci.image.manifest.v1+json`. Annotations sit at `manifest.annotations.<key>`. Document any deviations in PR body.
- Don't try to do auth/cosign in this brief — `mission-skill-publish-verified-ingest` covers that. v1 is anonymous-pull only.
- If `reqwest` isn't already a workspace dep, BLOCK and request a precursor brief to add it cleanly.
- The toml config schema is intentionally minimal — multi-registry support deferred to follow-up.
