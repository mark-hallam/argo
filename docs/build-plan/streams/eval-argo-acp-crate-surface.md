# eval-argo-acp-crate-surface

**Owner**: claude
**Priority**: P3 - operator tooling substrate audit
**Authored against**: origin/main `ef55f5422` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-argo-acp-crate-surface`.

## Source

The mission-vs-code gap manifest still marks `argo-acp` as `NOT AUDITED`.
The crate is intentionally standalone operator tooling substrate for Agent
Client Protocol harnesses, with auth, permission mapping, server dispatch, and
wire envelopes.

## Goal

Pin the `argo-acp` public API and verify that its standalone status, handshake
surface, auth gate, permission mapping, and JSON-RPC wire contracts are honest.

## May Touch

- `crates/argo-acp/tests/surface_contract.rs` (NEW)
- `crates/argo-acp/src/tests.rs` (only if the contract belongs in existing unit tests)
- `docs/skycot/evals/scheduled/2026-05-24/eval-argo-acp-crate-surface.md` (NEW report)
- `docs/build-plan/BUILD_STATUS.md`

## Must Not Touch

- Do not wire `argo-acp` into CLI or gateway code.
- Do not change production ACP behavior unless a contract test exposes an
  obvious compile-only drift and the fix is tiny.
- Do not add network, LLM, or live harness calls.

## Acceptance

- [ ] Pin the exported inventory from `crates/argo-acp/src/lib.rs` in a contract test.
- [ ] Cover harness capability defaults for at least VS Code, Cursor CLI, and generic harnesses.
- [ ] Cover auth verification failure and success shape without live tokens.
- [ ] Cover at least one permission mapping contract to `argo_types::Capability`.
- [ ] Cover JSON-RPC request/response/error serde shape for a representative ACP method.
- [ ] Write an eval report with verdict, standalone-status note, gaps found, and follow-up brief names only for real findings.
- [ ] `cargo test -p argo-acp --tests` passes, or the PR body records any unrelated clean-main failure with a focused green command.

## Notes

This is an audit/pin brief. If live integration gaps appear, queue follow-ups
rather than wiring them here.
