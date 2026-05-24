# mission-model-latest-runtime-wire

**Owner**: codex.
**Depends on**: none.
**Authored against**: origin/main `9aded3f9a` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-model-latest-runtime-wire`.

P2 provider-runtime follow-up. `argo-provider` already ships the
manifest parser, `resolve_latest`, and `LatestModelCache`, but current
repo grep shows the cache is only exercised by crate-local tests. The
manifest-backed transports still risk shipping `"latest"` straight
through unless a caller manually resolved it first.

## Scope

Wire the existing latest-model cache into the manifest-backed provider
transport path inside `argo-provider` so runtime requests resolve
`model = "latest"` before provider HTTP bodies are built. Keep the work
inside `argo-provider`: no gateway UI, no new background daemon, no
workspace-wide provider rearchitecture. If the refresh seam needs an
injected helper for tests, keep it crate-local and shared across the
manifest-backed transport families.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: `crates/argo-provider/src/transport.rs`

## Files you may touch

- `crates/argo-provider/src/transport.rs`
- `crates/argo-provider/src/anthropic/mod.rs`
- `crates/argo-provider/src/openai_compat/mod.rs`
- `crates/argo-provider/src/openai_responses/mod.rs`
- `crates/argo-provider/src/transport_tests.rs`
- `crates/argo-provider/src/anthropic/tests.rs`
- `crates/argo-provider/src/openai_compat/tests.rs`
- `crates/argo-provider/src/openai_responses/tests.rs`

## Files you must NOT touch

- `crates/argo-agent-runner/**`
- `crates/argo-cli/**`
- `docs/build-plan/BUILD_STATUS.md` except the standard PR-number row update.
- Anything not on the may-touch list above.

## Acceptance criteria

- [ ] At least the manifest-backed Anthropic, OpenAI-compat, and OpenAI-responses transports resolve `"latest"` through `LatestModelCache` before request serialization.
- [ ] Cold-cache behavior remains deterministic: unresolved `"latest"` falls back to the manifest's static model rather than panicking.
- [ ] Non-`"latest"` model ids remain byte-for-byte unchanged.
- [ ] Tests demonstrate both a prewarmed-cache success path and the cold-cache fallback path.
- [ ] `cargo test -p argo-provider` passes.
- [ ] `cargo clippy -p argo-provider --all-targets -- -D warnings` passes.
- [ ] Every modified file remains under 500 LOC.

## Branch + PR

- Branch: `codex-mission-model-latest-runtime-wire`
- PR title: `provider: wire model=latest through runtime cache (mission-model-latest-runtime-wire)`

## Notes for the implementer

- Reuse the shared cache type; do not duplicate `"latest"` resolution logic per transport.
- If one transport family needs a slightly different hook shape, refactor only as far as needed to keep the cache wiring common and testable.
