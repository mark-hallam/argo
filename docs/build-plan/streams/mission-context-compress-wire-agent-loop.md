# mission-context-compress-wire-agent-loop — wire context_compress/truncate into run_turn_with_events_inner

**Owner**: claude. **Depends on**: `argo_runtime::context_compress` + `context_truncate` (shipped); `argo_config::ContextCompressConfig` (shipped).
**Authored against**: origin/main `6e3e31d8` on `2026-05-14`.

> **Phase 3 wire-or-delete decision (CEO-authorized 2026-05-14): WIRE.** Audit cluster 3 O-3 found ~600 LOC subsystem with `compress()` + `truncate_to_fit()` never called from agent-runner. Config + operator knobs ship but the hot path doesn't consult them.

## Scope

In `argo-agent-runner::run_turn_with_events_inner` (the inner production turn loop), before each provider call:
1. Estimate token count of full message history.
2. If over `cfg.context.compress_threshold_tokens` (config default 80% of model context window): call `context_compress::compress(...)` on older messages.
3. If STILL over `cfg.context.truncate_threshold_tokens` (config default 95%): call `context_truncate::truncate_to_fit(...)`.
4. Emit `runtime.context_compressed` audit row on every compression (already a defined AuditKind variant).

## Files you may touch
- `crates/argo-agent-runner/src/turn.rs` or `turn_helpers.rs` — narrowly: ≤80 LOC at the per-call-prep site.
- `crates/argo-runtime/src/context_compress.rs` + `context_truncate.rs` — narrowly: minor API tweaks if needed.
- `crates/argo-agent-runner/tests/context_compress_smoke.rs` (NEW, ≤150 LOC) — assert: long session compresses; very long session truncates; audit rows emitted.

## Acceptance
- [ ] Long-session turn (10+ rounds, ~50k tokens accumulated) triggers compression on next call.
- [ ] Very long session triggers truncation.
- [ ] Audit rows emitted on every compress + truncate.
- [ ] Disabled-config path (`cfg.context.enabled=false`) preserves pre-brief behavior.
- [ ] Workspace clippy clean.

## Branch
- `claude-mission-context-compress-wire-agent-loop`
