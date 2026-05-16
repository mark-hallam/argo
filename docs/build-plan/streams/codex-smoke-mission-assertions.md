# codex-smoke-mission-assertions — Add 5 mission-scenario assertions to extended smoke (F-13 P2, demo-blocker complement)

**Owner**: codex. **Depends on**: F-03 (`codex-memory-tool-namespace-wiring`), F-04 (`codex-per-worker-learning-cron-runtime-spawn`), F-05 (`codex-notify-fallback-risk-filter`), F-06 (`codex-policy-as-code-enforcement`), F-10 (`codex-macos-smoke-real-action`) all MERGED.
**Authored against**: origin/main `58e853b6` on `2026-05-16`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-smoke-mission-assertions`. ALSO verify each dep above is MERGED.
**Source**: F-13 P2 in `docs/skycot/audit/2026-05-16-one-shot-architecture-audit.md`.

## Mission alignment

- **V1 ship gate (`V1_DEFINITION.md:165`):** `scripts/smoke-skycot.sh` must pass.
- **All 5 success scenarios (`V1_DEFINITION.md:41`):** smoke must PROVE each, not skip-pass.

## Scope

P2 — smoke is currently broader but not a complete proof of the 5 success scenarios. Per audit F-13, missing assertions:

1. **Memory tool namespace use through agent dispatch loop** (not just prompt hydration) — proves Claim 1's `memory_namespace` end-to-end.
2. **Fallback risk filtering** — proves Claim 6's risk-appropriate routing.
3. **Policy-as-code with real tenant/role/roster context** — proves Claim 1's `auto_approval_policy` + V1 scenario #4.
4. **Per-worker learning cron scheduled by runtime boot** — proves V1 scenario #2.
5. **Real macOS action execution** (not `argo --help`) — proves V1 scenario #1.

F-10 fixes #5 directly. This brief adds #1-#4 as positive assertions in the smoke gate.

Scope:
- Add scenario #26 to `scripts/smoke-skycot.sh`: `cargo test -p argo-agent-runner --test memory_namespace_runtime_tests`  (the F-03 test) — proves memory tool dispatches with worker namespace.
- Add scenario #27: `cargo test -p argo-skill --test per_worker_learning_cron_runtime` (the F-04 test) — proves cron is scheduled.
- Add scenario #28: `cargo test -p argo-cli --test policy_as_code_enforcement_integration` (NEW test from F-06) — proves quiet hours + signatures + audit-chain enforced.
- Add scenario #29: `cargo test -p argo-cli worker_actions_notify_fallback` — proves F-05 fix lands.
- Update each `heading "N. ..."` comment to clarify what it asserts (so "skip" paths are visible).
- Treat any test marked `skip` as `WARN`, not `PASS` (smoke script change in `run_scenario`).

LOC: ~120.

## Files you may touch

- `scripts/smoke-skycot.sh` — add scenarios 26-29 + skip-vs-pass distinction. ≤80 modified lines.
- `crates/argo-cli/tests/policy_as_code_enforcement_integration.rs` (NEW from F-06) — verify it's the right test target. ≤20 modified lines (skip if F-06's brief already creates it).

## Files you must NOT touch

- The dependent F-03, F-04, F-05, F-06 implementation files — they're in their own briefs.
- F-10 macOS smoke — separate brief.
- Other smoke scenarios — extend, don't refactor.

## Acceptance criteria

- [ ] `scripts/smoke-skycot.sh` has scenarios 26-29 covering F-03, F-04, F-06, F-05 assertions.
- [ ] Skip-paths in any scenario emit `SKIP` (not `OK`) — operator sees "compatibility check" vs "mission assertion."
- [ ] Smoke passes 29/29 (with explicit-skip annotations) on macOS+ARGO_NATIVE_MACOS_INTEGRATION=1.
- [ ] Each new scenario's heading comment names the mission claim it proves.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `codex-smoke-mission-assertions`
- PR title: `test(smoke): add memory/cron/policy/fallback mission assertions to extended smoke (codex-smoke-mission-assertions) — F-13 P2`

## Notes for the implementer

- The audit explicitly says: "Treat skip-heavy tests as 'compatibility checks,' not mission success checks." The script change makes this distinction visible in run output.
- This brief is the LAST of the v1-related audit fixes. After it lands + dependencies merge, the smoke is honest.
- Re-running smoke post-merge should be the trigger for the v1.0 demo recording. Update RESUME.md to reflect this dependency chain.
