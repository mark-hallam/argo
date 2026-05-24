# eval-claim-1-live-memory-roundtrip-extension

**Owner**: claude
**Priority**: P2 - claim 1 live regression extension
**Authored against**: origin/main `50a3ea9f0` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-claim-1-live-memory-roundtrip-extension`.

## Source

Evaluator round 19 recommended a quiet-tick extension for Claim 1: prove that live memory survives the runtime path a worker actually uses, not only lower-level store helpers. Existing coverage in `crates/argo-agent-runner/tests/memory_namespace_runtime_tests.rs` already proves the in-core `memory` tool binds to a worker namespace.

## Goal

Extend Claim 1 memory regression coverage so a write made through the production memory tool path is visible on a subsequent turn/tool call for the same worker namespace and invisible to a different worker namespace.

## May Touch

- `crates/argo-agent-runner/tests/memory_namespace_runtime_tests.rs`
- `crates/argo-agent-runner/tests/claim_1_live_memory_roundtrip.rs` (NEW, if splitting keeps the existing file smaller)
- `crates/argo-agent-runner/src/turn_memory.rs` only for comments or tiny test-only helper exposure if unavoidable.
- `docs/skycot/evals/scheduled/2026-05-24/eval-claim-1-live-memory-roundtrip-extension.md` (NEW report)
- `docs/build-plan/BUILD_STATUS.md`

## Must Not Touch

- Do not change production memory semantics unless the new regression exposes a clear bug and the fix stays inside the listed files.
- Do not widen memory visibility between workers or tenants.
- Do not edit web UI or gateway routes.

## Acceptance

- [ ] A regression writes memory through the same `Tool`/runtime dispatch surface used by `builtin_tools_with_runtime`, then reads/query-scores it from a subsequent same-worker invocation.
- [ ] A second worker namespace under the same tenant cannot read the first worker's row.
- [ ] The test uses real `SessionStore`, `MemoryStore`, and `OrchestratorStore` in-memory fixtures rather than mocks.
- [ ] The module docs cite the Claim 1 live invariant being pinned.
- [ ] Add a short eval note under `docs/skycot/evals/scheduled/2026-05-24/` summarizing exactly what is and is not proven.
- [ ] `cargo test -p argo-agent-runner --test memory_namespace_runtime_tests` or the new focused test binary passes.

## Notes

Keep this as a regression extension. If a full LLM turn is too broad, pin the production tool-binding path and document that agent text generation remains out of scope.
