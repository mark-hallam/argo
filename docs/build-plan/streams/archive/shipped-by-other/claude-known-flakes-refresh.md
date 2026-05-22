# claude-known-flakes-refresh — refresh KNOWN_FLAKES.md to match the boundary notes workers consistently flag

**Owner**: claude. **Depends on**: nothing. **Phase A docs hygiene — small, high-leverage.**

**Authored against**: origin/main 6b22a1ac on 2026-05-09 (conductor-authored after observing workers flag the same 8 boundary tests across 6+ recent PRs).

## Why

Every recent worker self-exit note + PR boundary section flags the same 8 pre-existing test failures as "outside may-touch, reproduce on clean main". `docs/build-plan/KNOWN_FLAKES.md` was last refreshed before some of these failures appeared (notably the post-#1039 strict-mode regressions in `worker_actions_executor*`).

Workers spend ~30 seconds per iteration verifying these are actually pre-existing (running `cargo test` on a stash, comparing). A refreshed KNOWN_FLAKES.md saves that time across the swarm — small docs change, multiplied by every iteration.

## Approach (concrete)

1. Read current `docs/build-plan/KNOWN_FLAKES.md`.
2. Add the 6 post-#1039 strict-mode failures workers consistently report:
   - `worker_actions_executor::stream_a_tests::resolve_executor_dispatches_each_kind_through_tagged_form`
   - `worker_actions_executor::worker_actions_executor_tests::resolve_executor_rejects_unknown_executor_kind`
   - `worker_actions_executor_cu_macos::cu_macos_tests::dispatch_on_macos_writes_bundle`
   - `worker_actions_executor_cu_macos::cu_macos_tests::manifest_hyphen_alias_resolves_to_macos_kind`
   - `worker_actions_executor_cu_macos::cu_macos_tests::manifest_underscore_form_resolves_to_macos_kind`
   - `worker_actions_executor_cu_macos::cu_macos_tests::unknown_kind_with_macos_in_name_does_not_resolve`
3. For each, add a one-line root cause: vertical-manifest schema-validation tightening from #1039; tests need fixture updates that haven't been queued yet.
4. Group them under a new section "Post-#1039 strict-mode regressions" with a "follow-up brief: TBD" pointer.
5. Verify each entry by running `cargo test -p argo-cli <test-name>` on clean origin/main and confirming reproduction.

## Files you may touch

- `docs/build-plan/KNOWN_FLAKES.md` — narrowly: append "Post-#1039 strict-mode regressions" section + 6 entries

## Files you must NOT touch

- Any Rust source — this is docs-only
- Any other markdown — KNOWN_FLAKES.md is the only target

## Acceptance criteria

- [ ] All 6 listed tests confirmed failing on clean `origin/main`
- [ ] Each entry has the test name, the failure mode (e.g., "expects empty manifest, gets 1 manifest"), and a one-line root-cause hypothesis
- [ ] No Rust changes; no test changes; pure docs

## Branch + PR

- Branch: `claude-known-flakes-refresh`
- PR title: `docs: refresh KNOWN_FLAKES.md with post-#1039 strict-mode regressions (claude-known-flakes-refresh)`

## Notes for the implementer

- Quick brief — should ship in <30 min wall-clock.
- Match existing entry format in KNOWN_FLAKES.md.
- Do NOT attempt to fix the failing tests in this brief; that's the follow-up brief's job.

## Unblocks

- Saves ~30s/iteration of worker verification time across the swarm.
- Frees a future maintenance brief to actually fix the regressions (rather than just documenting them).
