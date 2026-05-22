# Mission load-bearing wire — bump CU tool variant from `computer_20241022` to current

**Owner**: claude. **Depends on**: none (Anthropic API client surface; pure version bump).
**Authored against**: origin/main `c91e37e5` on `2026-05-10`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-cu-tool-variant-update`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 5 item #93 ("Update CU tool variant from `computer_20241022` to current"). The hardcoded variant string at `anthropic.rs:152` was current for the original CU launch but Anthropic has since shipped at least one newer revision. Stale variant string risks: (a) silently degraded action support; (b) broken parsing if response shape diverges; (c) inability to use newer primitives the API now supports. Mission Claim 5 ("real-world execution") wants the executor on a supported variant.

## Scope

Bump the hardcoded `"computer_20241022"` variant string in `crates/argo-executor-computer-use/src/anthropic.rs` to the current Anthropic-published variant, and route the choice through a `tool_variant` field in `CuConfig` (in `config.rs`) so future bumps are a config edit, not a code change. Default the field to the new current variant. Existing callers that don't set the field get the new default.

The switch should be cohesive with parser expectations — if the new variant emits any new action shapes (e.g. wheel-scroll, drag-with-modifier) that today's parser would reject as "unknown action", **either** add narrow parser support **or** declare the unknown action a non-goal in the brief and bail to existing error path. (The implementer makes the call after reading the current Anthropic docs; the brief budgets up to 2 NEW parser arms IFF the new variant adds primitives the existing executor already supports.)

This is NOT in scope: speculative new primitives the executor doesn't yet implement; per-tenant variant overrides (one global default is fine for v1); a feature-flag gate (the bump is irreversible — pin the new default in code).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `crates/argo-executor-computer-use/src/anthropic.rs` — bump the variant string at line 152 to read from `cfg.tool_variant` rather than the hardcoded literal.
- `crates/argo-executor-computer-use/src/config.rs` — narrowly scoped: ADD `tool_variant: String` field to `CuConfig` (or whatever the config struct is named — verify in first 5 minutes), defaulting to the new current variant. Update `Default` impl.
- `crates/argo-executor-computer-use/src/anthropic_tests.rs` — narrowly scoped: tests that build a request body should assert the variant string matches the new default; ADD 1 NEW test asserting `cfg.tool_variant = "..."` overrides the request body.
- `crates/argo-executor-computer-use/src/anthropic_parse.rs` — IF the new variant adds primitives the executor supports, narrow extension: add up to 2 NEW match arms to `parse_tool_use` for the new action names. Otherwise, NO edits here.
- `crates/argo-executor-computer-use/src/anthropic_parse_tests.rs` (path may be `anthropic_tests.rs` — verify) — IF parser arms added, 2 NEW tests covering the new primitive parses.
- `crates/argo-executor-computer-use/src/config_tests.rs` (NEW if absent; if present, append) — 2 unit tests: default-config-has-new-variant; explicit-override-applies.

## Files you must NOT touch

- `crates/argo-executor-computer-use/src/executor.rs` — dispatch is independent
- `crates/argo-executor-computer-use/src/pool.rs` / `pool_reaper.rs` (in flight in sibling brief) — orthogonal concern
- `crates/argo-executor-computer-use/src/e2b.rs` — sandbox provisioning unchanged
- `crates/argo-executor-computer-use/src/learn.rs` — feedback loop unchanged
- Any non-executor crate

## Acceptance criteria

- [ ] `CuConfig.tool_variant: String` exists with a `Default` impl pointing at the new current variant string
- [ ] `anthropic.rs:152` reads from `cfg.tool_variant` instead of the hardcoded `"computer_20241022"` literal
- [ ] No occurrence of `"computer_20241022"` remains anywhere in the crate (verify with `rg -F 'computer_20241022' crates/argo-executor-computer-use/`)
- [ ] If new variant primitives are added: parser supports them with up to 2 new match arms; tests cover them
- [ ] If new variant primitives are NOT supported: brief notes the deferred primitives in PR body for a follow-up brief
- [ ] All existing 8+ parser tests still pass
- [ ] 1 NEW test asserts config override changes request body
- [ ] 2 NEW config tests cover default + override
- [ ] `cargo build -p argo-executor-computer-use` clean
- [ ] `cargo clippy -p argo-executor-computer-use --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-executor-computer-use` passes (skip pre-existing flakes per KNOWN_FLAKES.md)
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-cu-tool-variant-update`
- PR title: `cu-executor: bump CU tool variant to current via CuConfig (mission-cu-tool-variant-update)`

## Notes for the implementer

- First 5 minutes: confirm the current Anthropic-published variant by checking `https://docs.anthropic.com/en/docs/build-with-claude/computer-use` (or the SDK docs for whatever version is in `Cargo.toml`). Pin THAT variant as the new default. The brief deliberately doesn't name it because Anthropic ships these on their own cadence — pick the current one at implementation time.
- Don't introduce a feature flag. Pin the new default. If the new variant has been deprecated by the time someone re-reads this brief, that's a follow-up brief to bump again.
- The 2-NEW-arms parser budget is hard. If the new variant adds 3+ primitives, file a follow-up brief and ship just the bump.
- The config field name `tool_variant` is preferred over `model_variant` or `cu_version` — match the Anthropic terminology.
- If `CuConfig` already carries other "version-like" fields, keep the new one alongside them; don't refactor unrelated config.
