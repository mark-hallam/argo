# codex-secret-redaction-followup-provider-keys — migrate provider api_keys to Redacted<String> (P2)

**Owner**: codex. **Depends on**: `mission-secret-redaction-newtype` MERGED (currently #1774 IN_REVIEW).
**Authored against**: origin/main `8a6029ac` on `2026-05-17`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-secret-redaction-followup-provider-keys`.
**Source**: follow-up to #1774 — once `Redacted<T>` lands in argo-types, additional provider/secret callsites should migrate to use it.

## Mission alignment

- **Security** (CLAUDE.md priority #4) — defense-in-depth. Each migrated callsite removes one accidental-leak vector.
- **Repeatability** (#3) — canonical type across all secret fields.

## Scope

P2. After `mission-secret-redaction-newtype` MERGED, migrate provider-config api_key fields from `String` to `Redacted<String>`:

- `argo-provider::AnthropicConfig::api_key` → `Redacted<String>`
- `argo-provider::OpenAIConfig::api_key` (if present) → `Redacted<String>`
- Any other `pub api_key: String` field under `crates/argo-provider/src/`

Plus the consumer migrations: each callsite that currently does `.clone()` on the api_key field flips to `.expose().clone()`.

LOC: ~100.

## Files you may touch

- `crates/argo-provider/src/anthropic.rs` — flip api_key type. ≤15 modified LOC.
- `crates/argo-provider/src/openai.rs` (if present) — same. ≤15 modified LOC.
- `crates/argo-provider/src/<other>.rs` — flip any other api_key fields. ≤20 modified LOC total.
- Consumer callsites — worker `rg 'api_key.clone\(\)' crates/argo-provider crates/argo-cli` to find them, flip each to `.expose().clone()`. ≤30 modified LOC.
- `crates/argo-provider/Cargo.toml` — narrowly: ensure dep on `argo-types`. ≤2 modified LOC.

## Files you must NOT touch

- The `Redacted<T>` newtype itself — read-only consumer.
- `argo-config` — that brief (`mission-secret-redaction-newtype`) handles config-side. This brief is provider-side.
- Anything outside `argo-provider` + minimal consumer fixes.

## Acceptance criteria

- [ ] All `pub api_key: String` in `crates/argo-provider/src/` flipped to `Redacted<String>`.
- [ ] `rg 'pub api_key: String' crates/argo-provider/` returns 0 matches.
- [ ] All consumer `.clone()` callsites on api_key fields → `.expose().clone()`.
- [ ] `Debug`-printing an `AnthropicConfig` shows `[redacted]` for api_key.
- [ ] No test currently does `assert_eq!(format!("{:?}", config), "<literal key>")` — those tests need fixing or were already buggy.
- [ ] `cargo test -p argo-provider` passes.
- [ ] `cargo clippy -p argo-provider --all-targets -- -D warnings` clean.
- [ ] `cargo test --workspace` does not regress.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `codex-secret-redaction-followup-provider-keys`
- Title: `refactor(provider): migrate api_key fields to Redacted<String> (codex-secret-redaction-followup-provider-keys)`
- Worker verification: crate-scoped clippy + tests + workspace clippy per SWARM_PROTOCOL.md.

## Hidden constraints

- **Pre-claim check: confirm #1774 MERGED.** Worker should `git log origin/main -- crates/argo-types/src/secret.rs` and verify a non-empty result before claiming.
- **`.expose()` is grep-auditable.** Don't introduce a `Deref` shim that hides the access. Every `.expose()` call is an audited operation.
- **Provider key Display in logs.** After migration, any `tracing` log that printed the key via `{}` will print `[redacted]`. Don't fix logs to print the real key (that's the bug we're fixing).
- **Builder pattern stays.** If `AnthropicConfig::builder().api_key(s)` exists, it accepts `impl Into<Redacted<String>>` so the builder caller can pass either `String` or `Redacted<String>`.
- **No `impl From<String> for Redacted<String>`** per the parent brief — accept that the builder needs `Redacted::new(s)` explicitly. Documented in builder doc-comment.
