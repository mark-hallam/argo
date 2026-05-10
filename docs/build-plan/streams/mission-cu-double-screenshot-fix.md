# Mission load-bearing wire — avoid double-screenshot when E2B returns inline screenshot

**Owner**: claude. **Depends on**: none.
**Authored against**: origin/main `9fe4c612` on `2026-05-10`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-cu-double-screenshot-fix`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 5 item #95 ("Avoid double-screenshot"). At `executor.rs:346-349`, the post-action screenshot logic has two match arms that BOTH call `self.e2b.screenshot(&sandbox).await?`, even when E2B's action response already returned `screenshot_b64` inline. Result: every step pays an unnecessary HTTP round-trip when the sandbox would have shipped the post-action screenshot in the same response. Mission Claim 5 ("real-world execution") loses ~100-300ms per step.

## Scope

Fix the bug in the post-action screenshot match at `executor.rs:346-349`:

```rust
// before (BUG: both arms make the network call)
screenshot = match result.screenshot_b64.as_deref() {
    Some(_b64) => self.e2b.screenshot(&sandbox).await?,
    None => self.e2b.screenshot(&sandbox).await?,
};
```

```rust
// after (use inline when available, fall back to fresh fetch)
screenshot = match result.screenshot_b64.as_deref() {
    Some(b64) => decode_inline_screenshot(b64)?,
    None => self.e2b.screenshot(&sandbox).await?,
};
```

`decode_inline_screenshot` is a NEW helper (≤ 30 LOC) at the top of `executor.rs` (or `e2b.rs` if more idiomatic) that base64-decodes the string into `Vec<u8>` and surfaces a typed `CuError::Protocol("e2b returned malformed inline screenshot: ...")` on decode failure. The existing `screenshot` HTTP path is unchanged.

This is NOT in scope: switching the entire CU loop to streaming screenshots; pre-fetching screenshots speculatively; modifying `e2b.rs`'s screenshot HTTP method; changing the CU step cadence.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `crates/argo-executor-computer-use/src/executor.rs` — fix the match at line 346-349; call new `decode_inline_screenshot` helper on the `Some(b64)` arm.
- `crates/argo-executor-computer-use/src/e2b.rs` — narrowly scoped: ADD `pub(crate) fn decode_inline_screenshot(b64: &str) -> Result<Vec<u8>, CuError>` near the existing screenshot HTTP method. NO other edits.
- `crates/argo-executor-computer-use/src/anthropic_tests.rs` (or `executor_tests.rs` if it exists; verify in first 5 minutes) — narrowly scoped: ADD 3 NEW unit tests: inline-screenshot-decoded; missing-inline-falls-back-to-http; malformed-base64-surfaces-protocol-error.
- `crates/argo-executor-computer-use/Cargo.toml` — narrowly scoped: confirm `base64` is in `[dependencies]` (it should be; if not, add ONE line). No other Cargo edits.

## Files you must NOT touch

- `crates/argo-executor-computer-use/src/anthropic.rs` / `anthropic_parse.rs` — request-side; orthogonal
- `crates/argo-executor-computer-use/src/pool.rs` / `pool_reaper.rs` (in flight in sibling brief) — orthogonal
- `crates/argo-executor-computer-use/src/learn.rs` — feedback loop unchanged
- Any non-executor crate

## Acceptance criteria

- [ ] When `result.screenshot_b64` is `Some`, the loop uses the decoded inline bytes — does NOT call `self.e2b.screenshot(...)`
- [ ] When `result.screenshot_b64` is `None`, the loop falls back to `self.e2b.screenshot(...)` (today's behavior preserved)
- [ ] Malformed base64 surfaces as `CuError::Protocol("e2b returned malformed inline screenshot: ...")` (do NOT silently fall back to HTTP — that masks an upstream bug)
- [ ] `decode_inline_screenshot` is `pub(crate)` and lives in `e2b.rs` (or `executor.rs` — implementer's call) ≤ 30 LOC including doc-comment
- [ ] 3 new tests cover the decode-success / fallback-to-http / decode-failure paths
- [ ] No new public API on the crate
- [ ] `cargo build -p argo-executor-computer-use` clean
- [ ] `cargo clippy -p argo-executor-computer-use --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-executor-computer-use` passes (skip pre-existing flakes per KNOWN_FLAKES.md)
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-cu-double-screenshot-fix`
- PR title: `cu-executor: use inline screenshot from E2B response, avoid extra HTTP fetch (mission-cu-double-screenshot-fix)`

## Notes for the implementer

- The bug is unambiguous: both match arms of `result.screenshot_b64.as_deref()` make the same HTTP call. The fix is replacing the `Some(_b64)` arm's body.
- `base64` decoding: use `base64::Engine::decode` from the `base64` crate (already in workspace). Standard alphabet + no padding-strict probably matches what E2B emits; verify against `e2b.rs` test fixture.
- The `Vec<u8>` type returned must match what `screenshot` returns today (raw PNG bytes). The downstream `persist_screenshot` call expects raw PNG bytes — keep that contract.
- This brief is small (≤ 100 LOC including tests). If it grows past 200 LOC, you've taken on extra scope — stop and re-read.
- If the `result.screenshot_b64` field is actually unused elsewhere (verify with `rg`), keep it — removing it is out of scope and risks breaking E2B-side serde shape compatibility.
