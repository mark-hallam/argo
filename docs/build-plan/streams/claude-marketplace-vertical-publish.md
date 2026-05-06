# Claude — Vertical publishing to OCI marketplace

**Owner**: claude. **Depends on**: G-genesis (#790) merged
(verticals shipped + loadable). Optional: argo-marketplace
crate already exists per the workspace.

## Scope

A vertical is a directory tree. To share verticals across
tenants — the v1.x story — you need a way to publish them
to an OCI registry as a versioned artifact + pull them on
the receiving side.

This brief adds:
- `argo vertical publish <id> --registry <url> [--tag <ver>]`
  — packs `verticals/<id>/` into an OCI manifest using the
  existing argo-marketplace primitives, signs it (sigstore
  if available, falls back to manifest hash), pushes.
- `argo vertical pull <oci-ref>` — fetches + verifies +
  unpacks under `verticals/<id>/`. Refuses if a vertical
  with the same id already exists locally (operator must
  delete first or use `--rename`).

## Files you may touch
- `crates/argo-cli/src/cli_vertical_cmds.rs` — narrowly:
  add publish + pull subcommands
- `crates/argo-cli/src/handlers/vertical_publish.rs` (NEW)
- `crates/argo-cli/src/handlers/vertical_pull.rs` (NEW)
- `crates/argo-marketplace/src/oci.rs` — extend if a
  thin helper is missing (the crate already has push /
  pull primitives per the workspace memory)
- `crates/argo-marketplace/Cargo.toml` — narrowly: add a
  dep ONLY if genuinely needed

## Files you must NOT touch
- Any vertical's manifest
- argo-vertical (read-only consumer)
- Any other CLI subcommand

## Acceptance criteria
- [ ] `argo vertical publish marketplace-seller --registry
      ghcr.io/example` works against a local OCI registry
      (test container, e.g. `registry:2`)
- [ ] `argo vertical pull ghcr.io/example/marketplace-seller:1.0.0`
      round-trips (publish → pull → loadable)
- [ ] Hash verification on pull rejects tampered manifests
      with a clear error (NOT a crash)
- [ ] `--rename <new-id>` flag for pull-side conflict
      resolution
- [ ] Integration test using a docker-compose'd registry
      (gated on `OCI_REGISTRY_INTEGRATION=1`)
- [ ] Unit tests for the pack / unpack / hash-verify
      logic that don't need a registry
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean
- [ ] Every authored file < 500 LOC

## Branch + PR
- Branch: `claude-marketplace-vertical-publish`
- PR title: `vertical: publish + pull subcommands for OCI marketplace (claude-marketplace-vertical-publish)`

## Notes
- The existing argo-marketplace crate (per workspace
  memory: "real OCI push" was on the v1.x todo) may have
  partial scaffolding. Read its code first to see what's
  already there; reuse aggressively.
- For sigstore signing, fall back to a workspace-local
  ed25519 key if sigstore isn't reachable. Document the
  fallback in `argo doctor`'s output.
- This is the brief that makes "skill library two-tier"
  (H-4b) actually USEFUL — the library tier ships via
  this publish/pull flow.
