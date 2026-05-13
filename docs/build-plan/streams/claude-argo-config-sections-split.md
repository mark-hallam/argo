# PRIORITY — split `argo-config/src/sections.rs` under 500 LOC

**Owner**: claude. **Depends on**: nothing (pure refactor).
**Authored against**: origin/main `88a2a769` on `2026-05-13`.

> **WHY THIS IS PRIORITY:** `crates/argo-config/src/sections.rs`
> is currently **519 LOC** — over the 500-LOC ceiling. Pre-claim
> ceiling refusals block every TODO brief that adds a new config
> section (SSO, SCIM, IP allowlist follow-ups, retry backoff, etc.).

## Scope

`crates/argo-config/src/sections.rs` at **519 LOC** holds the
`[runtime]`, `[provider]`, `[memory]`, `[gateway]`, `[mcp]`,
`[approval]`, `[audit]`, `[security]`, … config structs. Each new
operator-facing section adds ~20-40 LOC. The legitimate pending
additions (rate-limiter knobs, throttle bands, audit knobs) cannot
land without a split.

Look at the file's actual structure first; typical groupings:

- `sections_runtime.rs` — `[runtime]`, throttling, sandbox, retry
- `sections_provider.rs` — `[provider]`, transport, manifests
- `sections_memory.rs` — `[memory]`, namespaces, compression
- `sections_gateway.rs` — `[gateway]`, rate-limit, IP allowlist
- `sections_security.rs` — already extracted per a recent brief; verify
- `sections_audit.rs` — `[audit]`, chain, redaction
- `sections_approval.rs` — `[approval]`, escalation
- `sections_mcp.rs` — `[mcp]`, connector pool

Public API stable: `pub use` everything in `sections.rs` so callers
keep their `use argo_config::sections::ProviderSection;` imports
unchanged.

## Approach

1. `rg -n '^pub struct .*Section' crates/argo-config/src/sections.rs` to enumerate.
2. Carve into ≤250 LOC sibling files (TOML deserialization is verbose; aim small).
3. Re-export from `sections.rs` (which becomes a thin module + re-export hub, ≤150 LOC).
4. Add `mod sections_*;` to `crates/argo-config/src/lib.rs`.

## Files you may touch

- `crates/argo-config/src/sections.rs` — shrink to ≤150 LOC
- NEW siblings: `sections_runtime.rs`, `sections_provider.rs`, `sections_memory.rs`, `sections_gateway.rs`, `sections_audit.rs`, `sections_approval.rs`, `sections_mcp.rs` (or whatever groupings — adjust)
- `crates/argo-config/src/lib.rs` — `pub mod` entries
- Existing test files (`sections_tests.rs`) — only if function signatures change; pure-move shouldn't require edits

## Files you must NOT touch

- Test bodies — section shapes unchanged
- The `loader.rs` / `loader_tests.rs` — config-load logic is orthogonal
- Any consumer crate's `use argo_config::sections::*;` import — public re-export covers them

## Acceptance criteria

- [ ] `sections.rs` < 500 LOC (target ≤ 150)
- [ ] Every new sibling < 500 LOC
- [ ] `cargo build --workspace` clean (cross-crate API)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean (sections.rs is consumed widely)
- [ ] All `cargo test -p argo-config` tests pass unchanged
- [ ] BUILD_STATUS.md updated

## Branch + PR

- Branch: `claude-argo-config-sections-split`
- PR title: `argo-config: split sections.rs under 500 LOC (claude-argo-config-sections-split)`

## Notes

- Watch for nested struct definitions — keep the parent and its
  nested children co-located in one sibling.
- The `-split` suffix auto-skips the pre-claim ceiling check.
- Pure mechanical work.
