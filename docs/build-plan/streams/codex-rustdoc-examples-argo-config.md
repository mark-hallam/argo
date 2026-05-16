# codex-rustdoc-examples-argo-config — backfill rustdoc examples in argo-config (P2)

**Owner**: codex. **Depends on**: none (single-crate, mechanical, doc-only).
**Authored against**: origin/main `8a6029ac` on `2026-05-17`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-rustdoc-examples-argo-config`.
**Source**: parallel to `codex-rustdoc-examples-argo-types` (in flight) — `argo-config` is the next-most-imported crate and has partial doctest coverage today.

## Mission alignment

- **Repeatability** (CLAUDE.md priority #3) — examples are testable (`cargo test --doc`) and self-document.
- **Reliability** (#1) — examples that compile = APIs that match their docs.

## Scope

P2. Walk `crates/argo-config/src/` and add a `# Examples` block with one compileable doctest to every `pub` item missing one. Target ≥80% coverage.

Discipline (same as argo-types brief):
- Wrap examples in ` ```rust ... ``` ` so `cargo test --doc -p argo-config` exercises them.
- Examples are minimal: 1–5 lines after `use` import.
- Don't add to `#[doc(hidden)]` items.
- Preserve existing `///` prose; append `# Examples` underneath.

LOC: ~200 (lines of doc comments + example bodies).

## Files you may touch

All files under `crates/argo-config/src/`:

- `lib.rs`
- `sections_*.rs` (siem, provider, gateway, audit, etc. — worker grep first)
- Any other `pub` surface files.

**No changes to:**
- Test files (`*_tests.rs`).
- Non-pub items.
- Other crates.
- Cargo.toml (no new deps).

## Files you must NOT touch

- Other crates. This brief is `argo-config` ONLY.
- `argo-types` — that's the sibling brief (`codex-rustdoc-examples-argo-types`).
- The CI gate that checks doc coverage — consume only.

## Acceptance criteria

- [ ] `cargo test --doc -p argo-config` passes (every example compiles + runs).
- [ ] `cargo doc --no-deps -p argo-config 2>&1 | grep -c "missing documentation"` strictly lower than at start (capture before-count in PR body).
- [ ] At least 25 new `# Examples` blocks added.
- [ ] No example uses unstable language features.
- [ ] No example reads files or makes network calls.
- [ ] `cargo clippy -p argo-config --all-targets -- -D warnings` clean.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `codex-rustdoc-examples-argo-config`
- Title: `docs(config): backfill rustdoc examples for argo-config public API (codex-rustdoc-examples-argo-config)`
- Worker verification: doctest + clippy + workspace clippy per SWARM_PROTOCOL.md.

## Hidden constraints

- **Doctest imports explicit.** Each example needs `use argo_config::Foo;`.
- **Sensitive types (api_key fields) MUST NOT example with real values.** Use `"YOUR_API_KEY_HERE"` or `"sk-test-stub"`. Reviewers grep for accidental real-secret-shaped strings.
- **Use the `# fn main() -> Result<...>` wrapper pattern** for fallible APIs (rustdoc auto-wraps).
- **Don't add network examples.** Same rule as the argo-types brief.
- **Don't conflict with parallel argo-types brief.** This brief is strictly `argo-config`; if a re-exported type lives in argo-types, leave it alone here.
