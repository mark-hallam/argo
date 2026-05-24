# fix-cli-gateway-cors-doc-continuation-lints

**Owner**: codex.
**Depends on**: none.
**Authored against**: origin/main `e55c2a1dc` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-cli-gateway-cors-doc-continuation-lints`.

P1 gate-unblock. PR #2172 fixed the `argo-skill` workspace-clippy
cluster, then the conductor pre-merge gate on the PR head surfaced the
next pre-existing workspace clippy failure: a `doc_lazy_continuation`
cluster in the CORS header docblock.

## Scope

Fix the three `doc_lazy_continuation` warnings at
`crates/argo-cli/src/handlers/gateway/cors.rs:37-39` without changing
runtime behavior. This should be a doc-comment reflow only. The
intended meaning is that `X-CSRF-Token` must remain in the allow-list
or signed-in SPA POSTs fail closed from cross-origin frontends.

Do not broaden this into CORS behavior changes.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/cors.rs` - doc-comment reflow only.

## Files you must NOT touch

- `crates/argo-cli/src/handlers/gateway/**` other than `cors.rs`.
- `crates/argo-skill/**` - PR #2172 owns the prior lint cluster.
- CORS config, request handling, or tests unless the doc-only fix unexpectedly changes generated docs.
- `docs/build-plan/BUILD_STATUS.md` except the standard PR-number row update.
- Anything not on the may-touch list above.

## Acceptance criteria

- [ ] The three `doc_lazy_continuation` warnings at `cors.rs:37-39` are gone.
- [ ] `ALLOWED_HEADERS` string is byte-for-byte unchanged.
- [ ] No runtime CORS logic changes.
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` passes or surfaces only pre-existing out-of-scope failures documented in the PR body.
- [ ] `bash scripts/workspace-clippy-fast-fail.sh` reaches the next gate state; if a new unrelated lint appears, document it as a boundary note.
- [ ] `bash scripts/check-brief-freshness.sh fix-cli-gateway-cors-doc-continuation-lints` passes.
- [ ] Every modified file remains under 500 LOC.

## Branch + PR

- Branch: `codex-fix-cli-gateway-cors-doc-continuation-lints`
- PR title: `fix(cli-gateway): reflow CORS doc comment for clippy (fix-cli-gateway-cors-doc-continuation-lints)`

## Notes for the implementer

- Prefer indentation or sentence reflow over deleting context. The doc explains why `X-CSRF-Token` is load-bearing.
- This is intentionally small; do not use it to revisit the CORS allow-list.
