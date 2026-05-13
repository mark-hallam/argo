# claude-delete-argo-gateway-crate — delete orphan `argo-gateway` crate (677 LOC)

**Owner**: claude. **Depends on**: nothing (pure delete).
**Authored against**: origin/main `bf8236e3` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-delete-argo-gateway-crate`.

> **PURE DELETE 2026-05-14 by conductor (CEO-authorized Phase 1 cleanup).** Audit cluster 5 found `crates/argo-gateway/` (677 LOC: `lib.rs`, `adapter.rs`, `router.rs`, `adapters/{slack,discord,telegram,mod}.rs`) is **orphan** — no Cargo.toml in the workspace depends on it. The CLI's gateway functionality lives in `crates/argo-cli/src/handlers/gateway/` (a different concept — HTTP server, not chat-platform adapter). Inbound chat webhooks consume `argo-channels` directly. The `argo-gateway` abstraction was forward-declared per `§21` design doc; the codebase routed around it.

## Scope

Delete `crates/argo-gateway/` entirely + remove its workspace entry.

If any rare external reference exists (docs example, CI fixture, brief), surface in PR body — keep the crate if so, with a 1-line note explaining the lone consumer. But the audit confirmed zero Cargo dependency.

**Not in scope**: rewriting the HTTP-server gateway in `handlers/gateway/` (that's the canonical surface, untouched); rebuilding chat-platform adapters (those live in `argo-channels` + the connector crates).

## Files you may touch

- `crates/argo-gateway/` — `git rm -r`.
- `Cargo.toml` (workspace root) — remove `crates/argo-gateway` from `[workspace.members]`.
- `Cargo.lock` — regenerate via `cargo check`.
- Any doc that references `argo-gateway` as a crate (NOT `argo gateway` the CLI subcommand — that's `handlers/gateway/`): grep `rg 'argo-gateway' docs/` and update to point at the actual canonical surface.

## Files you must NOT touch

- `crates/argo-cli/src/handlers/gateway/` — the canonical gateway implementation; locked.
- `argo-channels` and its `Adapter`-shaped functionality — locked.

## Acceptance criteria

- [ ] `crates/argo-gateway/` removed.
- [ ] Workspace `Cargo.toml` no longer lists `crates/argo-gateway`.
- [ ] `cargo build --workspace` succeeds.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] `cargo test --workspace` passes.
- [ ] Any docs that referenced `argo-gateway` as a Rust crate now point at `handlers/gateway/` or `argo-channels`, OR carry an explicit "deleted, see ARCHITECTURE.md" note.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-delete-argo-gateway-crate`
- PR title: `cleanup: delete orphan argo-gateway crate (677 LOC) (claude-delete-argo-gateway-crate)`

## Notes for the implementer

- Pure delete. ~1 hour worker time.
- If you find a hidden consumer, KEEP the crate and report in PR body.
- The `§21` design intent (chat-platform adapter trait) is real, but the codebase encoded it differently — each connector handles its own webhook (`handlers/gateway/routes_channels_*`) consuming `argo-channels` types directly. The `Adapter` trait was the wrong abstraction; we routed around it.
