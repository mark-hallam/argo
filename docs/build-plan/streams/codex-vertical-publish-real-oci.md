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

## Files you may touch (amended tick149 — actual layout)
- BLANKET CLI ALLOWLIST applies
- `crates/argo-marketplace/src/oci_distribution.rs` —
  narrowly: replace mock push/pull paths with real
  Distribution Spec calls
- `crates/argo-marketplace/src/oci_auth.rs` — narrowly:
  add credentials.toml token + ~/.docker/config.json
  parsing for fallback chain
- `crates/argo-marketplace/src/bundle.rs` — narrowly:
  serialize manifest layers with the correct
  application/vnd.skycot.vertical.v1+toml media types
- `crates/argo-marketplace/src/registry.rs` — narrowly:
  expose new push/pull entry points
- `crates/argo-marketplace/Cargo.toml` — narrowly: add
  `oci-distribution` dep if not already present
- `crates/argo-cli/src/handlers/vertical_publish.rs` —
  narrowly: route to real-OCI path when `--registry` set,
  fallback to mock when unset (back-compat)
- `crates/argo-cli/src/handlers/vertical_pull.rs` —
  narrowly: same — real-OCI when `--registry` set
- `crates/argo-cli/src/cli_vertical_cmds.rs` — narrowly:
  surface `--registry` flag if not already exposed
- `crates/argo-cli/tests/vertical_publish_oci_smoke.rs` (NEW —
  gated on `ARGO_OCI_TEST_REGISTRY` env)

## Acceptance criteria
- [ ] Push to ttl.sh (no-auth ephemeral registry) works
- [ ] **(Operator-validated post-merge, not in CI):** Push to ghcr.io
      with token auth works. Codex tabs in the swarm don't have
      write:packages scope on their GH tokens, so this can't be
      verified at PR time. The CODE PATH for ghcr.io must work — but
      the CI evidence is ttl.sh only. Boundary-note this in the PR.
- [ ] Pull round-trips correctly (bundle hash matches) — verified
      against ttl.sh
- [ ] Manifest layers correctly typed
      (application/vnd.skycot.vertical.v1+toml, etc.)
- [ ] Auth fallback chain: --token > credentials.toml >
      docker config > anonymous (verified via unit tests of the
      auth resolver)
- [ ] Mock path still works when no `--registry` (back-compat)
- [ ] 6 unit tests + 1 integration test (ttl.sh round-trip)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean

## Branch + PR
- Branch: `codex-vertical-publish-real-oci`
- PR title: `vertical: real OCI push/pull via oci-distribution (codex-vertical-publish-real-oci)`
