# V1.x — Real OCI push for vertical publish

**Owner**: codex. **Depends on**: claude-marketplace-vertical-publish
(#821) merged.

## Scope

Today `argo vertical publish` produces an OCI bundle but
pushes to a mocked registry index (per memory notes:
"real OCI push deferred to v1.x"). Replace the mock with a
real OCI Distribution Spec push — works against any
OCI-compliant registry (ghcr.io, Docker Hub, ttl.sh,
self-hosted Zot/Harbor).

- `argo vertical publish <id> --registry <ref>` accepts a
  real OCI registry reference (e.g.
  `ghcr.io/mark-hallam/argo-verticals/maya:v1`)
- Auth via Docker config (`~/.docker/config.json`) +
  per-registry token from credentials.toml
- Push: build manifest → push blobs (charter, manifest.toml,
  skills, eval seeds) as separate layers → finalize manifest
- `argo vertical pull <ref>` becomes the inverse

Use `oci-distribution` Rust crate — well-maintained and
already vetted by other Rust ecosystem projects.

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- `crates/argo-vertical/src/oci_push.rs` (NEW or replace
  existing mock)
- `crates/argo-vertical/src/oci_pull.rs` (NEW or replace mock)
- `crates/argo-vertical/Cargo.toml` — narrowly: add
  `oci-distribution` dep
- `crates/argo-cli/src/handlers/vertical_publish.rs` —
  narrowly: route to real-OCI path when `--registry` set,
  fallback to mock when unset (back-compat)
- `crates/argo-cli/tests/vertical_publish_oci_smoke.rs` (NEW —
  gated on `ARGO_OCI_TEST_REGISTRY` env)

## Acceptance criteria
- [ ] Push to ttl.sh (no-auth ephemeral registry) works
- [ ] Push to ghcr.io with token auth works
- [ ] Pull round-trips correctly (bundle hash matches)
- [ ] Manifest layers correctly typed
  (application/vnd.skycot.vertical.v1+toml, etc.)
- [ ] Auth fallback chain: --token > credentials.toml >
      docker config > anonymous
- [ ] Mock path still works when no `--registry` (back-compat)
- [ ] 6 unit tests + 1 integration test (ttl.sh round-trip)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean

## Branch + PR
- Branch: `codex-vertical-publish-real-oci`
- PR title: `vertical: real OCI push/pull via oci-distribution (codex-vertical-publish-real-oci)`
