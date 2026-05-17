# codex-rustdoc-examples-argo-store — backfill rustdoc examples in argo-store (P2 fast-mode)

**Owner**: codex. **Sizing hint**: fast-mode candidate — pure doc backfill, no API change.
**Depends on**: none (single-crate, mechanical).
**Authored against**: origin/main `c2d3417d` on `2026-05-17`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-rustdoc-examples-argo-store`.
**Source**: completes the rustdoc-examples series for the 4 most-imported crates. Parallel to MERGED #1776 (argo-types), #1786 (argo-config), #1795 (argo-mcp).

## Mission alignment

- **Repeatability** (CLAUDE.md priority #3) — examples are testable (`cargo test --doc`).
- **Reliability** (#1) — examples that compile = APIs that match their docs.

## Scope

P2 fast-mode. Walk `crates/argo-store/src/` and add `# Examples` blocks with compileable doctests to every `pub` item missing one. Target ≥25 new examples.

Discipline (identical to the three prior briefs):
- Wrap examples in ` ```rust ... ``` ` so `cargo test --doc -p argo-store` exercises them.
- Examples are minimal: 1-5 lines after `use` import.
- Don't add to `#[doc(hidden)]` items.
- Preserve existing `///` prose; append `# Examples` underneath.
- argo-store touches SQLite — examples that need a DB use the existing in-memory test helper.

LOC: ~200.

## Files you may touch

All `pub` items under `crates/argo-store/src/`:
- `lib.rs` (`SessionStore`, `BlobStore`, `Pool`, etc.)
- `*_query.rs` (tenant_query, user_query, user_tenant_query, support_query, etc.)
- `tenant_rate_limit.rs`, `tenant_quota.rs`
- `gdpr.rs`, `encryption.rs`
- Migration helpers (NOT the `.sql` files themselves)

**No changes to:**
- Test files (`*_tests.rs`).
- Migration `.sql` files.
- Non-pub items.
- Other crates.
- Cargo.toml.

## Files you must NOT touch

- Other crates. argo-store ONLY.
- The migration runner contract (apply_all, etc.) — exists; just doc it, don't change behavior.

## Acceptance criteria

- [ ] `cargo test --doc -p argo-store` passes (every example compiles + runs).
- [ ] `cargo doc --no-deps -p argo-store 2>&1 | grep -c "missing documentation"` strictly lower than at start, OR already at floor 0.
- [ ] At least 25 new `# Examples` blocks added.
- [ ] No example writes to a real DB file — all examples use the in-memory helper or are pure type-only.
- [ ] No example uses unstable language features.
- [ ] `cargo clippy -p argo-store --all-targets -- -D warnings` clean.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `codex-rustdoc-examples-argo-store`
- Title: `docs(store): backfill rustdoc examples for argo-store public API (codex-rustdoc-examples-argo-store)`
- Worker verification: doctest + clippy + workspace clippy per SWARM_PROTOCOL.md.

## Hidden constraints

- **Doctest imports explicit.** Each example: `use argo_store::SessionStore;`.
- **In-memory DB pattern** — most existing tests use a `Pool::open_memory()`-style helper; if so, examples for query helpers do the same. Look at `argo-store/src/tenant_query_tests.rs` for the pattern. Otherwise mark examples `no_run`.
- **Don't include real SQL in examples.** Use the public query helpers; the helpers wrap the SQL.
- **Use `# fn main() -> Result<...>` wrapper** for fallible APIs (most queries return `Result`).
- **No new dependencies.** Stay within argo-store's existing Cargo.toml.
