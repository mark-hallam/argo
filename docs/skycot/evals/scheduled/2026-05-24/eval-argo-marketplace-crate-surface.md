# eval-argo-marketplace-crate-surface

Verdict: PASS

`argo-marketplace` is a live marketplace substrate, not dormant code. The crate
root exports workspace bundle, vertical bundle, registry, OCI auth, and OCI
distribution surfaces from `crates/argo-marketplace/src/lib.rs`, and
`crates/argo-marketplace/tests/surface_contract.rs` now pins that crate-root
inventory plus the externally visible behavior those exports support.

Pinned contracts:

- Workspace bundle API: `BUNDLE_PATHS`, `build_bundle`, `manifests_match`,
  `BundleEntry`, and `BundleManifest` are pinned with a deterministic fixture
  that excludes unknown top-level directories and round-trips through
  `MemoryRegistry`.
- Vertical bundle API: `build_vertical_bundle` and `unpack_vertical_bundle` are
  pinned with an arbitrary-path vertical fixture, deterministic re-bundle check,
  and tamper rejection before extraction.
- Registry API: `OciReference`, `OciRegistry`, `MemoryRegistry`, and
  `StubRegistry` are pinned for canonical reference formatting, in-memory
  push/pull semantics, and stub no-network behavior.
- OCI auth/distribution API: `resolve_registry_auth`, `BearerChallenge`,
  `ResolvedRegistryAuth`, `OciDistributionRegistry`, `OciDescriptor`,
  `OciImageManifest`, `CONFIG_MEDIA_TYPE`, `LAYER_MEDIA_TYPE`, and
  `MANIFEST_MEDIA_TYPE` are pinned without contacting a live registry.

Static consumer evidence:

- `crates/argo-cli/Cargo.toml:46` depends on `argo-marketplace`.
- `crates/argo-cli/src/handlers/workspace_marketplace.rs:6-29` uses
  `build_bundle`, `OciReference`, `OciDistributionRegistry`, and `OciRegistry`
  for `argo workspace push`; lines `43-65` use the same registry surface for
  pull; lines `71-111` use it for fork; lines `115-178` use `build_bundle` for
  local diff.
- `crates/argo-cli/src/handlers/workspace_marketplace.rs:221-281` records pushed
  artifacts and constructs the live registry client from CLI credential state.
- `crates/argo-cli/src/handlers/vertical_publish.rs:13-75` uses
  `build_vertical_bundle`, `resolve_registry_auth`, `MemoryRegistry`,
  `OciDistributionRegistry`, `OciReference`, and `OciRegistry` for vertical
  publication.
- `crates/argo-cli/src/handlers/vertical_pull.rs:15-60` uses
  `resolve_registry_auth`, `OciDistributionRegistry`, `OciReference`,
  `OciRegistry`, and `unpack_vertical_bundle` for vertical pull and verified
  extraction.
- `crates/argo-cli/src/cli_vertical_cmds.rs:257-319` exposes the user-facing
  `argo vertical publish` and `argo vertical pull` commands backed by this
  substrate.

Live-vs-dormant classification:

- Live substrate: workspace marketplace push/pull/fork/diff and vertical
  publish/pull all call into `argo-marketplace` from `argo-cli`.
- Test/pin-facing substrate: `OciDescriptor`, `OciImageManifest`, and
  `manifests_match` are public DTO/helper surfaces with direct contract
  coverage but no current production callsite outside the crate.
- No live OCI registry is contacted by the new test; existing mock-registry
  integration tests remain the network boundary.

Gaps found: none requiring a follow-up brief.

Verification:

- `bash scripts/check-brief-freshness.sh eval-argo-marketplace-crate-surface`
  reported only the authored-SHA distance warning; structural checks were clean.
- `cargo fmt -p argo-marketplace`
- `CARGO_TARGET_DIR=/tmp/eval-argo-marketplace-crate-surface-target cargo test -p argo-marketplace --tests`
