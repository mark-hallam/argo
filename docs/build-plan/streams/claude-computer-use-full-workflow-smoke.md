# claude-computer-use-full-workflow-smoke — Real macOS computer-use end-to-end smoke (Claim 5)

**Owner**: claude. **Depends on**: `mission-vertical-action-executor-completion` MERGED (for non-stub action-executor wiring).
**Authored against**: origin/main `c928d8fe` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-computer-use-full-workflow-smoke`.

## Scope

Phase C audit found that Claim 5 (real-world PC execution) has STRONG unit-test coverage but lacks an E2E test that drives the **real** macOS computer-use executor through a full workflow. The existing `argo-executor-computer-use-macos/tests/notes_smoke.rs` is env-gated single-action; smoke-skycot.sh step 19 mocks BrowserExecutor only. No test proves the end-to-end loop: worker → action queue → CU executor spawn → multi-step screen interaction → completion.

This brief adds the missing E2E smoke for the macOS path (the v1 primary execution surface per the 2026-05-14 operator-locked decision).

LOC: ~300.

## Files you may touch

- NEW `crates/argo-cli/tests/computer_use_macos_e2e_smoke.rs` — macOS-gated `#[cfg(target_os = "macos")]` test. Drives an agent through queuing a 3-step CU action (screenshot → click → screenshot+verify), running the executor, asserting the action ledger reflects success + 2+ screenshot artifacts. Uses Apple's built-in `Notes.app` or similar harmless target (no Marketplace/Slack/external surface).
- `scripts/smoke-skycot.sh` — narrowly: add as conditional step 21 (skip on non-macOS hosts). ≤15 modified lines.
- `crates/argo-cli/Cargo.toml` — confirm test discovery + `argo-executor-computer-use-macos` dep on macOS targets. ≤10 modified lines.

## Files you must NOT touch

- The macOS executor's source (this is a CONSUMER test).
- argo-agent-runner's source (consume the existing API).
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] `cargo test --test computer_use_macos_e2e_smoke -p argo-cli` passes on macOS.
- [ ] Test is correctly gated for non-macOS (compiles as no-op or skipped).
- [ ] Test takes <30s wall-clock (uses pre-cached app launch where possible).
- [ ] Test produces 2+ screenshot artifacts in the run's audit directory.
- [ ] `bash scripts/smoke-skycot.sh` on macOS exits 0 including new step 21; on non-macOS the step is skipped with a clear log line.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] BUILD_STATUS.md row updated with PR number.

## Branch + PR

- Branch: `claude-computer-use-full-workflow-smoke`
- PR title: `test(executor): real macOS computer-use end-to-end smoke (claude-computer-use-full-workflow-smoke)`

## Notes for the implementer

- Mission-claim alignment: Claim 5 (real-world PC execution) + scenario #1 (autonomous non-CLI worker action).
- The test target should be a **system app available on all macOS** (Notes.app or TextEdit.app). Do NOT depend on installed third-party apps.
- Use `e2e-deterministic` fixtures for the action sequence — don't rely on the LLM choosing the actions, hardcode the 3-step plan.
- If `argo-agent-runner` hasn't been wired to spawn the CU executor yet (check first), expand scope OR file a follow-up brief for the wiring (do not silently expand may-touch).
- Drift-prevention: brief adds a NEW test file only; no new pub types.
