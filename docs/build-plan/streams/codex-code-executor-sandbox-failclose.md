# codex-code-executor-sandbox-failclose — Local backend fail-closed in production; capability gate (F-12 P2)

**Owner**: codex. **Depends on**: nothing.
**Authored against**: origin/main `58e853b6` on `2026-05-16`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-code-executor-sandbox-failclose`.
**Source**: F-12 P2 in `docs/skycot/audit/2026-05-16-one-shot-architecture-audit.md`.

## Mission alignment

- **Claim 5 (real-world execution on PC):** Computer-use + MCP for human-on-PC tasks (`docs/skycot/MISSION.md:80`).
- **CLAUDE.md security invariant:** Capability-based security + tenant boundaries (`CLAUDE.md:46`, `:48`); reject unbounded execution patterns.
- **F-12 scope:** Production safety, not v1-demo-critical (mission demo uses macOS CU, not code execution).

## Scope

P2 production safety. The code executor has two backends:
- `BackendKind::E2b` — comment says "v1 returns a 'not yet wired' error when selected; HTTP client lands in a follow-up" (`crates/argo-cli/src/handlers/worker_actions_executor_code.rs:112-113`)
- `BackendKind::Local` — comment says "bash -c in the parent OS namespace. No isolation" (`worker_actions_executor_code.rs:117`)

Operator decision (based on Claim 5 + CLAUDE.md security): **fail-closed in production** for Local; **keep Local dev-only** behind explicit capability check. Don't wire E2B in this brief — that's a separate follow-up `codex-e2b-client-implementation` when operator prioritizes cloud sandbox.

Scope:
- Add a runtime check: when `BackendKind::Local` is selected AND the action's effective policy says `production = true` (or tenant config says `allow_local_code_execution = false`), FAIL CLOSED with a clear "local code execution disabled in production" error.
- Make `BackendKind::Local` require a new `Capability::LocalCodeExecution` capability that's NOT in default roles. Owner/admin gets it explicitly; user does not.
- Update the executor to log a redacted warning when `Local` is selected — "code executor using non-isolated local backend; intended for dev only".
- When `BackendKind::E2b` is selected, keep the existing "not yet wired" error path (don't lie that it works).
- Add tests: production profile + Local backend → fail closed; dev profile + Local → permitted with warning; E2b → "not wired" error unchanged.

LOC: ~180.

## Files you may touch

- `crates/argo-cli/src/handlers/worker_actions_executor_code.rs` — capability check + production gate at backend resolution. ≤80 modified lines.
- `crates/argo-types/src/capability.rs` — add `Capability::LocalCodeExecution` variant. ≤20 modified lines.
- `crates/argo-capability/src/lib.rs` — match arm for new variant; default roles get it for owner/admin only. ≤40 modified lines.
- `crates/argo-config/src/sections.rs` — `[code_executor]` config block (`allow_local_in_production` default false). ≤30 modified lines.
- `crates/argo-cli/src/handlers/worker_actions_executor_code_tests.rs` (sibling extend) — 3 regression tests. ≤80 LOC.

## Files you must NOT touch

- `crates/argo-executor-computer-use/**` — different executor, out of scope.
- `BackendKind::E2b` HTTP client wiring — separate follow-up brief.
- `bash -c` invocation in Local backend internals — that's the dev path; we're adding gate, not removing isolation.

## Acceptance criteria

- [ ] `Capability::LocalCodeExecution` exists in argo-types.
- [ ] Default user role does NOT have it; owner/admin does.
- [ ] Production profile + Local backend → returns error "local code execution disabled in production" (or equivalent).
- [ ] Dev profile + Local backend → permitted, emits warning log.
- [ ] E2b backend still returns "not yet wired" error (unchanged).
- [ ] `[code_executor]` config block parses + drives the gate.
- [ ] 3 regression tests pass.
- [ ] `cargo test -p argo-cli -p argo-capability -p argo-types worker_actions_executor_code` passes.
- [ ] `cargo clippy -p argo-cli -p argo-capability -p argo-types -p argo-config --all-targets -- -D warnings` clean.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `codex-code-executor-sandbox-failclose`
- PR title: `feat(executor): fail-closed Local backend in production + LocalCodeExecution capability (codex-code-executor-sandbox-failclose) — F-12 P2`

## Notes for the implementer

- Production-vs-dev: derive from explicit config (`argo-config`) or env var; do not infer from heuristics like `cfg(debug_assertions)`.
- The warning log on dev-Local should include the action's tenant + worker for trace.
- Capability check happens BEFORE backend selection; backend selection happens AFTER capability passes.
- After this brief, code execution is production-safe by default. E2B HTTP client wiring is a separate brief operator can prioritize when cloud sandbox matters.
