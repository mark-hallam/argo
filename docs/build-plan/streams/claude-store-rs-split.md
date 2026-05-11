# Split-prep — argo-identity::store.rs into siblings

**Owner**: claude. **Depends on**: nothing — pure refactor; existing public API of `IdentityStore` must remain byte-identical on the outside.
**Authored against**: origin/main `d7b0165c` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-store-rs-split`.

> SOURCE: HARD RULE 6 ceiling-refusal of `mission-routing-skillmatch-rolematch` Tier M brief. `crates/argo-identity/src/store.rs` is currently 598 LOC — already over the 500 hard ceiling. The routing brief plans to add ~50 LOC of `add_worker_skill`/`remove_worker_skill`-adjacent work; the pre-claim ceiling check (`scripts/check-pre-claim-ceiling.sh`) blocks every worker that tries to claim it. This split-prep extracts ~100 LOC of worker_skill methods + the 53-LOC `archive_worker` method into siblings, bringing store.rs to ~450 LOC with comfortable headroom for the routing follow-up and future per-worker mutations.

## Scope

Move ~150 LOC out of `crates/argo-identity/src/store.rs` into two new sibling files, with NO public-API change:

1. **NEW `crates/argo-identity/src/store_skills.rs` (≤ 150 LOC)** — extracts the 4 worker_skill methods (`add_worker_skill`, `remove_worker_skill`, `workers_with_skill`, `skills_for_worker`) into a sibling `impl<'a> IdentityStore<'a> { ... }` block. Same `pub async fn` signatures; same `ArgoResult` return types. Pattern mirrors the existing `store_archive_tests.rs` and `store_skill_tests.rs` sibling-test wires.

2. **NEW `crates/argo-identity/src/store_archive_methods.rs` (≤ 80 LOC)** — extracts the 53-LOC `archive_worker` method (lines 366-418 in current store.rs) into a sibling impl block. Name disambiguates from existing `store_archive_tests.rs` sibling.

3. **NEW `crates/argo-identity/src/store_inline_tests.rs` (≤ 50 LOC)** — extracts the small inline `#[cfg(test)] mod tests` block (currently lines 564-591) into a sibling file. Existing `#[cfg(test)] mod archive_tests;` and `mod worker_update_tests;` sibling decls stay in store.rs.

4. **Edit `crates/argo-identity/src/store.rs`** — narrowly scoped: REMOVE the 4 worker_skill methods + REMOVE `archive_worker` method + REMOVE the inline test block; ADD `#[cfg(test)] mod store_inline_tests;` sibling-decl. Resulting LOC should be ~440-450.

5. **Edit `crates/argo-identity/src/lib.rs`** — narrowly scoped: ONE-line `mod store_skills;` + ONE-line `mod store_archive_methods;` registrations (no `pub`, internal-only).

The public crate API does not change. All callers continue to invoke `IdentityStore::add_worker_skill(...)`, `IdentityStore::archive_worker(...)`, etc. through the canonical Rust trick that you can have multiple `impl Foo` blocks for the same type in different files within the same crate.

This is NOT in scope: any behavior change in the moved methods; new methods or signatures; renaming or rescoping any method; changes to `IdentityStore::new`, `ensure_schema`, `create_worker`, `get_worker`, `list_workers`, `update_level`, `update_charter`, `update_worker`; any test rewrites beyond the move (the inline 28-LOC test block is moved verbatim into `store_inline_tests.rs`).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-identity/src/store_skills.rs` (NEW, ≤ 150 LOC) — sibling impl block w/ the 4 worker_skill methods + their `use` imports
- `crates/argo-identity/src/store_archive_methods.rs` (NEW, ≤ 80 LOC) — sibling impl block w/ `archive_worker` + its `use` imports
- `crates/argo-identity/src/store_inline_tests.rs` (NEW, ≤ 50 LOC) — sibling test mod (whatever the inline `mod tests { ... }` body currently contains in store.rs lines 564-591)
- `crates/argo-identity/src/store.rs` — REMOVE the extracted methods + REMOVE the inline test block; REPLACE with `#[cfg(test)] mod store_inline_tests;` sibling-decl; expected final LOC ~440-450
- `crates/argo-identity/src/lib.rs` — narrowly scoped: ADD `mod store_skills;` + `mod store_archive_methods;` (likely just above or below existing `mod store;` declaration)

## Files you must NOT touch

- Any other file in `crates/argo-identity/src/`
- Any other crate
- `crates/argo-store/migrations/**` — no schema change
- The 4 sibling test files that already exist: `store_archive_tests.rs`, `store_skill_tests.rs`, `store_trust_grade.rs`, `worker_update_tests.rs` — these stay as-is

## Acceptance criteria

- [ ] `crates/argo-identity/src/store.rs` line count is **≤ 480** (preferably 440-450) — `wc -l` confirms
- [ ] `cargo build -p argo-identity` clean (no public-API change → no downstream rebuild surface)
- [ ] `cargo clippy -p argo-identity --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-identity` passes byte-identical test outcomes (same test count, same names, all green)
- [ ] `crates/argo-identity/src/store_skills.rs` < 500 LOC
- [ ] `crates/argo-identity/src/store_archive_methods.rs` < 500 LOC
- [ ] `crates/argo-identity/src/store_inline_tests.rs` < 500 LOC
- [ ] No downstream crate (argo-cli, argo-runtime, argo-skill, etc.) needs any change — search via `rg "IdentityStore::add_worker_skill\|IdentityStore::archive_worker" crates/` and confirm none of the call sites need updating
- [ ] Pre-claim ceiling check passes for `mission-routing-skillmatch-rolematch` after this lands: `bash scripts/check-pre-claim-ceiling.sh mission-routing-skillmatch-rolematch` exits 0
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-claude-store-rs-split`
- PR title: `identity: split store.rs into store_skills + store_archive_methods siblings (claude-store-rs-split)`

## Notes for the implementer

- Mechanical refactor. Don't change anything substantive. If you find yourself thinking "this method is weird, let me improve it" — stop. That's a separate brief.
- The Rust pattern that makes this work: multiple `impl<'a> IdentityStore<'a> { ... }` blocks for the same type across different `.rs` files within the same crate compile fine. Each sibling file needs its own `use` imports.
- Order of operations: write the new sibling files first (with full content moved over), confirm `cargo build -p argo-identity` is happy, THEN delete from store.rs. This avoids leaving the tree in a doubly-defined state during the refactor.
- The inline test mod at lines 564-591 currently uses imports like `use super::*;`. In the sibling file, change those to `use super::IdentityStore;` plus any other `use` lines needed — `super::*` won't pick up the methods that moved to other siblings, but the test only exercises `IdentityStore` public API so explicit imports are cleaner.
- The `#[cfg(test)] mod tests { ... }` inline block becomes a `mod store_inline_tests;` sibling-decl in store.rs (one line). The file `store_inline_tests.rs` itself contains the test mod body without the outer `#[cfg(test)] mod tests { ... }` wrapper — it's all at top level inside the file. Mirror the pattern used by the existing `archive_tests` / `worker_update_tests` sibling files in the same crate.
- Unblocks `mission-routing-skillmatch-rolematch` (Tier M routing primitive) on merge. Worth tagging that PR's body with "unblocks mission-routing-skillmatch-rolematch ceiling refusal".
