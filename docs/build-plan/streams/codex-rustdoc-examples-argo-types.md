# codex-rustdoc-examples-argo-types — backfill rustdoc examples in argo-types (P2)

**Owner**: codex. **Depends on**: none (single-crate, mechanical, doc-only).
**Authored against**: origin/main `e2fe4b70` on `2026-05-16`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-rustdoc-examples-argo-types`.
**Source**: CLAUDE.md core-platform invariant — "Every public Rust API has rustdoc with at least one example". `argo-types` is the most-imported crate in the workspace; partial coverage today.

## Mission alignment

- **Repeatability** (CLAUDE.md priority #3) — examples are testable (`cargo test --doc`) and self-document.
- **Reliability** (#1) — examples that compile = APIs that match their docs.

## Scope

P2. Walk `crates/argo-types/src/` and add a `# Examples` block with one **compileable** example to every `pub fn`, `pub struct::new`, and `pub enum` variant that the audit shows is missing one. Target ≥ 80% coverage of public items in argo-types.

Discipline:
- Examples must be **doc-tested**: wrap in ` ```rust ... ``` ` fences. CI runs `cargo test --doc -p argo-types`.
- Examples are minimal: 1-5 lines after the `use` import, no `Result<...>` ceremony unless the API returns one.
- Examples demonstrate the **happy path** plus, where relevant, one error case behind `# ignored = "panics"`.
- Don't add examples to `#[doc(hidden)]` items or internal helpers.

Existing-doc-string discipline:
- If a `///` already exists, append `# Examples` underneath. Don't rewrite the existing prose.
- If no `///`, add a one-line summary + an `# Examples` block.

LOC: ~200 (lines of doc comments + example bodies).

## Files you may touch

All files under `crates/argo-types/src/`:

- `crates/argo-types/src/lib.rs` (342 LOC)
- `crates/argo-types/src/anomaly.rs`
- `crates/argo-types/src/budget.rs`
- `crates/argo-types/src/capability.rs`
- `crates/argo-types/src/cost_ceilings.rs`
- `crates/argo-types/src/error.rs`
- `crates/argo-types/src/ids.rs`
- `crates/argo-types/src/judge.rs`
- `crates/argo-types/src/lock.rs`
- `crates/argo-types/src/messages.rs`
- (and any others under `src/`)

**No changes to:**
- Test files (`*_tests.rs`).
- Non-pub items.
- Other crates.

## Files you must NOT touch

- Any other crate. This brief is `argo-types` ONLY.
- Test fixtures.
- The Cargo.toml — no new deps.

## Acceptance criteria

- [ ] `cargo test --doc -p argo-types` passes (every example compiles + runs).
- [ ] `cargo doc --no-deps -p argo-types 2>&1 | grep -c "missing documentation"` is **strictly lower** than at start (capture before-count in PR body).
- [ ] At least 30 new `# Examples` blocks added (worker counts and includes in PR body).
- [ ] No example uses unstable language features.
- [ ] No example calls `.unwrap()` on `Result` unless the function signature returns `()` from `main`.
- [ ] `cargo clippy -p argo-types --all-targets -- -D warnings` clean (including the new doctests).
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `codex-rustdoc-examples-argo-types`
- Title: `docs(types): backfill rustdoc examples for argo-types public API (codex-rustdoc-examples-argo-types)`
- Worker verification: doctest + clippy + workspace clippy per SWARM_PROTOCOL.md.

## Hidden constraints

- **Doctest imports are explicit.** Each `# Examples` block needs `use argo_types::Foo;` — not implicit.
- **Examples are doctests by default.** If you want a code block that doesn't compile, use `text` instead of `rust` as the language tag (rare).
- **No `unwrap()`-everywhere.** Prefer `?` with a `# fn main() -> Result<(), Box<dyn std::error::Error>>` wrapper pattern (rustdoc auto-wraps).
- **Examples are tests.** Don't include an example that calls a network service or reads a file — doctests run in CI.
- **Don't churn for the sake of churn.** If a function already has a good example, leave it alone. Add to gaps only.
- **`enum` variant docs.** Each `pub enum` variant gets its own `///` line; an example on the enum-level is fine, doesn't need per-variant.
