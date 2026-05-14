# mission-channels-operator-preferred-fallback — Wire operator preferences into fallback dispatch

**Owner**: claude. **Depends on**: claude-channels-fallback-orphan-cleanup MERGED.
**Authored against**: origin/main `21a240d4` on `2026-05-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-channels-operator-preferred-fallback`.

## Scope

Complete the operator-preferred fallback-channel integration that was
deferred when `FallbackChain::with_operator_preferred` was removed as an
orphan. The implementation should resolve the operator's preferred
approval channel from `OperatorPreferencesResolver` at dispatch time and
apply it to the actual notification fallback sequence in `argo-cli`.

LOC: ~150. Branch: `claude-mission-channels-operator-preferred-fallback`.
PR title: `feat(channels): apply operator-preferred fallback order`.

## Files you may touch

- `crates/argo-cli/src/handlers/worker_actions_notify.rs` — narrowly: pass the operator preference resolver/channel into fallback-chain construction. <=40 modified lines.
- `crates/argo-cli/src/handlers/worker_actions_notify_fallback.rs` — narrowly: preserve current chain semantics while honoring an already-resolved preferred channel. <=40 modified lines.
- `crates/argo-cli/src/handlers/worker_actions_notify_fallback_tests.rs` — add coverage for preferred-channel promotion. <=60 modified lines.
- `crates/argo-channels/src/operator_preferences.rs` — only if a small accessor is required. <=20 modified lines.
- `crates/argo-channels/src/operator_preferences_tests.rs` — only if the accessor above is added. <=30 modified lines.

## Files you must NOT touch

- Approval decision semantics.
- Channel transport implementations.
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] Operator primary-channel preference changes the first attempted fallback channel for a dispatch.
- [ ] Missing operator preferences preserve the manifest-defined fallback order.
- [ ] Existing fallback report output remains backward-compatible.
- [ ] `cargo clippy -p argo-cli -p argo-channels --all-targets -- -D warnings` clean.
- [ ] `cargo test -p argo-cli worker_actions_notify_fallback` passes.
- [ ] `cargo test -p argo-channels` passes.
- [ ] BUILD_STATUS.md updated with PR number.

## Notes for the implementer

- The prior orphan was removed because it lived in `argo-channels` with
  no production call site. This follow-up must wire the dispatch path,
  not reintroduce an unused helper.
- Mission-claim alignment: claim 6, multi-channel approval gates.
