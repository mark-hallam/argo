# codex-rustdoc-examples-argo-mcp — backfill rustdoc examples in argo-mcp (P2 fast-mode)

**Owner**: codex. **Sizing hint**: fast-mode candidate — pure doc backfill, no API change.
**Depends on**: none (single-crate, mechanical).
**Authored against**: origin/main `de644311` on `2026-05-17`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-rustdoc-examples-argo-mcp`.
**Source**: parallel to MERGED `codex-rustdoc-examples-argo-types` (#1776) + `codex-rustdoc-examples-argo-config` (#1786). `argo-mcp` is the third-most-imported crate and has partial doctest coverage.

## Mission alignment

- **Repeatability** (CLAUDE.md priority #3) — examples are testable (`cargo test --doc`).
- **Reliability** (#1) — examples that compile = APIs that match their docs.

## Scope

P2 fast-mode. Walk `crates/argo-mcp/src/` and add `# Examples` blocks with compileable doctests to every `pub` item missing one. Target ≥25 new examples.

Discipline (identical to the two prior briefs):
- Wrap examples in ` ```rust ... ``` ` so `cargo test --doc -p argo-mcp` exercises them.
- Examples are minimal: 1-5 lines after `use` import.
- Don't add to `#[doc(hidden)]` items.
- Preserve existing `///` prose; append `# Examples` underneath.

LOC: ~200.

## Files you may touch

All files under `crates/argo-mcp/src/`:
- `lib.rs`
- `pool.rs` (PoolConfig, PoolKey, PoolStats, McpClientPool, PoolGuard, acquire fn)
- `client.rs`
- `error.rs` (McpError enum)
- `protocol.rs`
- `server.rs`

**No changes to:**
- Test files (`*_tests.rs`).
- Non-pub items.
- Other crates.
- Cargo.toml.

## Files you must NOT touch

- Other crates. Argo-mcp ONLY.
- `argo-types::error` re-exports.

## Acceptance criteria

- [ ] `cargo test --doc -p argo-mcp` passes (every example compiles + runs).
- [ ] `cargo doc --no-deps -p argo-mcp 2>&1 | grep -c "missing documentation"` strictly lower than at start, OR already at floor 0.
- [ ] At least 25 new `# Examples` blocks added.
- [ ] No example uses unstable language features.
- [ ] No example spawns processes or makes network calls (MCP is subprocess-spawning so examples must use stubs / mock implementations).
- [ ] `cargo clippy -p argo-mcp --all-targets -- -D warnings` clean.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `codex-rustdoc-examples-argo-mcp`
- Title: `docs(mcp): backfill rustdoc examples for argo-mcp public API (codex-rustdoc-examples-argo-mcp)`
- Worker verification: doctest + clippy + workspace clippy per SWARM_PROTOCOL.md.

## Hidden constraints

- **Doctest imports explicit.** Each example: `use argo_mcp::Foo;`.
- **`acquire` example uses a mock factory** — `pool.acquire(|client_factory| async move { let client = client_factory().await?; /* ... */ Ok(()) }).await` with `#[ignore]` so it doesn't actually spawn a subprocess in CI.
- **No real connector names** in examples — use `"example-connector"` not `"slack"`.
- **`McpError` is in `argo-mcp::error`**, NOT `argo-types::error` — keep imports local.
- **Use `# fn main() -> Result<...>` wrapper** for fallible APIs.
