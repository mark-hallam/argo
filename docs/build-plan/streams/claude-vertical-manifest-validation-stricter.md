# claude-vertical-manifest-validation-stricter — strict-mode manifest checks

**Owner**: claude. **Depends on**: nothing. **Phase A correctness narrow.**

## Why

`vertical.toml` schema validation today is **permissive**: missing `executor.kind`, dangling `action.X` references, and `executor.binding` typos are caught only at dispatch time (or worse, silently no-op). A malformed manifest can ship to a tenant, get executed, and the failure surface is a vague "executor 'X' is not yet implemented" log line many layers downstream.

The hard contract a manifest must satisfy is implicit and tribal:
- Every `action.X` must declare `executor.kind`
- Every `executor.kind` must have a corresponding `executor.binding` shape
- Every `binding.url` (api kind) must be a valid URL
- Every referenced `secret.X` must be declared in `secrets:` block
- No two actions share an `id`
- `provider.kind` must be one of the registered kinds

These checks should run at **parse time**, before the manifest reaches dispatch, with crisp error messages including the line number and the bad field path.

## Approach (concrete)

**Pure validation.** Adds a strict-mode pass over the parsed manifest. Zero behaviour change for valid manifests. Currently-invalid-but-tolerated manifests now fail fast with a precise error.

1. **NEW `crates/argo-vertical/src/manifest_validate.rs`** (~280 LOC):
   - `pub fn validate_strict(manifest: &VerticalManifest) -> ArgoResult<()>` — runs every check, accumulating errors via a `Vec<ValidationError>` then returning a single `ArgoError::InvalidInput(_)` summarising all of them (instead of failing on the first).
   - `pub struct ValidationError { path: String, message: String, line: Option<usize> }` — the structured form.
   - Individual check functions: `check_action_executor_kind_present`, `check_executor_binding_shape`, `check_api_binding_url_valid`, `check_secret_references_declared`, `check_action_id_uniqueness`, `check_provider_kind_registered`.
2. **Modify `crates/argo-vertical/src/manifest.rs`** (or wherever `VerticalManifest::parse` lives) — narrowly: at the end of `parse`, call `validate_strict(&manifest)?`. Single line change.
3. **NEW `crates/argo-vertical/src/manifest_validate_tests.rs`** (≤ 350 LOC) — 12+ unit tests:
   - Valid manifest passes
   - Missing `executor.kind` → InvalidInput with path
   - Unknown `executor.kind` → InvalidInput
   - `kind = "api"` missing `binding.url` → InvalidInput
   - `kind = "api"` `binding.url` not a URL → InvalidInput
   - Reference to `{{secret.X}}` where `X` not in secrets block → InvalidInput
   - Duplicate `action.id` → InvalidInput
   - Unknown `provider.kind` → InvalidInput
   - Multiple errors in one manifest → all reported in one error message

## Files you may touch

- `crates/argo-vertical/src/manifest_validate.rs` (NEW, ≤ 350 LOC)
- `crates/argo-vertical/src/manifest_validate_tests.rs` (NEW, ≤ 400 LOC)
- `crates/argo-vertical/src/manifest.rs` — narrowly: 1 line at end of `parse` calling `validate_strict`
- `crates/argo-vertical/src/lib.rs` — narrowly: `pub mod manifest_validate;`
- Existing manifest fixtures under `crates/argo-vertical/tests/fixtures/` — narrowly: any fixture currently relying on a permissive parse may need a small fix

## Files you must NOT touch

- The `VerticalManifest` struct itself (no schema change — only stricter checks on existing fields)
- Any executor implementation (api, mcp, builtin) — they are downstream
- `argo-types` — no new types

## Acceptance criteria

- [ ] `validate_strict` returns `Ok(())` for every fixture currently in the repo
- [ ] Every check has a unit test covering the failure case + a test covering the happy case
- [ ] Multiple errors in one manifest are reported in a single `ArgoError::InvalidInput(_)` message
- [ ] Error messages include the `path` (e.g. `action.foo.executor.kind`) so callers can locate the bad field
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-vertical` clean
- [ ] Every modified file ≤ 500 LOC
- [ ] `gh pr view --json mergeable` returns MERGEABLE post-push

## Branch + PR

- Branch: `claude-vertical-manifest-validation-stricter`
- PR title: `vertical: strict-mode manifest validation (claude-vertical-manifest-validation-stricter)`

## Notes for the implementer

- Read `crates/argo-vertical/src/manifest.rs` end-to-end first. The parse path's exact shape determines where `validate_strict` slots in.
- The `path` field in `ValidationError` should follow the TOML-table convention: `action.foo.executor.kind`, not `action[0].executor.kind`. Mirror the user's mental model of the TOML file.
- Line numbers are nice-to-have. If `toml::Value` doesn't preserve them cheaply, leave `line: None` for now and file a follow-up.
- `check_provider_kind_registered` needs a list of registered kinds. Source it from the `provider` registry the dispatch code already uses — don't hard-code.
- This brief is **scoped narrowly to validation**. If you find a bug in dispatch caused by a previously-tolerated invalid manifest, file a follow-up — DO NOT widen scope.
