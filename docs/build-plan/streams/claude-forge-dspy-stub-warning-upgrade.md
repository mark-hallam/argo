# claude-forge-dspy-stub-warning-upgrade — Make Forge stub failure mode explicit + documented

**Owner**: claude. **Depends on**: nothing.
**Authored against**: origin/main `29b798be` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-forge-dspy-stub-warning-upgrade`.

## Scope

Phase A.5 audit found: `argo-forge::Forge` trait (forge.rs:7) is implemented only by `StubForge` in production. The stub returns a generic "configure Python worker" error when `optimize()` is called. Operators hitting this path see an opaque error with no path forward.

This is a v1-acceptable stub (real DSPy worker is post-v1), but the failure mode needs to be:
1. **Explicit**: the error message names the missing dependency clearly.
2. **Documented**: the operator sees a link to the follow-on brief that will wire the real optimizer.
3. **Logged**: failures are audit-chained, not silently dropped.

LOC: ~80.

## Files you may touch

- `crates/argo-forge/src/forge.rs` — narrowly: replace the generic stub error with a structured `ForgeError::OptimizerNotConfigured { docs_url: "..." }`. ≤30 modified lines.
- `crates/argo-forge/src/forge_tests.rs` (or sibling) — narrowly: test that the structured error path is hit. ≤20 modified lines.
- `crates/argo-forge/src/lib.rs` — narrowly: re-export the new error variant. ≤5 modified lines.
- NEW `docs/skycot/operator/forge-optimizer-status.md` — one-page operator-facing doc: "Forge is intentionally stubbed in v1; real DSPy worker is post-v1; here's the brief that will implement it."

## Files you must NOT touch

- Anything else in argo-forge logic.
- The cron that ENQUEUES forge runs (separate concern).
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] `ForgeError::OptimizerNotConfigured` variant exists with a docs_url field.
- [ ] StubForge returns this variant; downstream error rendering shows the docs link to operators.
- [ ] Audit-chain emits a FORGE_STUB_INVOCATION event when the stub is hit.
- [ ] Operator doc exists at `docs/skycot/operator/forge-optimizer-status.md`.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] `cargo test --workspace` passes.
- [ ] BUILD_STATUS.md row updated with PR number.

## Branch + PR

- Branch: `claude-forge-dspy-stub-warning-upgrade`
- PR title: `chore(forge): make stub failure mode explicit + documented (claude-forge-dspy-stub-warning-upgrade)`

## Notes for the implementer

- Mission-claim alignment: production hardening (clarifies known v1 stubs vs accidental gaps).
- This is NOT a Forge implementation. The real DSPy worker is post-v1 and will be a separate brief.
- Drift-prevention: brief adds ONE new error variant (named, scoped to forge). No new pub structs in other crates.
