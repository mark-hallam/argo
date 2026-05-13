# PRIORITY — split `argo-store/src/lib.rs` under 500 LOC (v2)

**Owner**: claude. **Depends on**: nothing (pure refactor).
**Authored against**: origin/main `88a2a769` on `2026-05-13`.

> **WHY THIS IS PRIORITY:** `crates/argo-store/src/lib.rs` is
> currently **507 LOC** — over the 500-LOC ceiling. Re-grown since
> the prior `claude-argo-store-lib-split` brief landed; further
> store-adjacent work refuses at pre-claim ceiling check. This
> brief is the **v2** split (prior v1 lived at #~915 era).

## Scope

`crates/argo-store/src/lib.rs` at **507 LOC** is the SessionStore
facade — public methods on `SessionStore` plus type re-exports.
Recent additions (cross-tenant audit field threading, retro
lifecycle helpers, memory namespace API) pushed it back over.

Look at the actual content first; typical groupings:

- `SessionStore::new` / `connect` / `migrate` constructors
- Worker CRUD methods
- Session CRUD methods
- Approval CRUD methods
- Audit-row append methods
- Memory-fact CRUD methods
- Retro lifecycle methods
- Cross-tenant scope helpers

Public API surface is `SessionStore` impl block — too risky to
split into multiple impl blocks (HARD RULE 3 caps each impl block
at 200 LOC). Solution: extract method bodies to free-standing
helper modules + keep `impl SessionStore` lean (thin wrappers
calling into helpers).

## Approach

1. Identify which methods on `impl SessionStore` are heaviest.
2. Carve heavy ones into `crates/argo-store/src/store_<topic>.rs` siblings holding the free-standing helper functions.
3. Keep `lib.rs` impl block at ≤200 LOC (HARD RULE 3): each method becomes a 2-5 LOC delegate calling the helper.
4. Internal-only — no public API change. Consumers keep their `store.create_worker(…).await?` calls unchanged.

## Files you may touch

- `crates/argo-store/src/lib.rs` — shrink to ≤200 LOC (HARD RULE 3 on the impl block also caps it)
- NEW siblings: `store_worker.rs`, `store_session.rs`, `store_approval.rs`, `store_audit.rs`, `store_memory.rs`, `store_retro.rs` (or whatever groupings the actual content supports)
- Add `mod store_*;` declarations next to existing `mod` lines

## Files you must NOT touch

- Public API of `SessionStore` — every public method stays
- Migration files (`migrations/v*.rs`)
- `Cargo.toml` (no new deps)
- Consumer crates — re-exports cover everything

## Acceptance criteria

- [ ] `lib.rs` < 500 LOC AND the `impl SessionStore` block < 200 LOC (HARD RULE 3)
- [ ] Every new sibling < 500 LOC
- [ ] `cargo build --workspace` clean
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] All `cargo test -p argo-store` tests pass byte-for-byte unchanged
- [ ] Cross-crate consumer tests (argo-cli integration) pass unchanged
- [ ] BUILD_STATUS.md updated

## Branch + PR

- Branch: `claude-argo-store-lib-split-v2`
- PR title: `argo-store: split lib.rs under 500 LOC (v2) (claude-argo-store-lib-split-v2)`

## Notes

- Brief slug includes `-split` so pre-claim ceiling check auto-skips.
- The original v1 split lives in archive. This v2 follows the same pattern; the file just grew again.
- Pure mechanical extraction. No behavior change.
- Watch the `pub use` re-exports at the top of lib.rs — preserve them all.
