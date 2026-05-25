# ops-claim-4-and-6-live-cold-rerun-on-current-main

**Owner**: claude or codex.
**Authored against**: origin/main `bec35dd45` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh ops-claim-4-and-6-live-cold-rerun-on-current-main`.

🟡 **P1 — v1.0 launch remaining item (a) from CLAUDE.md "Remaining for v1.0 launch" list**

CLAUDE.md item (a): "cold-rerun the remaining claim-N-LIVE regression tests on current main — claims 2/3/5 now have evaluator-round-51/53/54 PASSes; **claim-4 still needs `claim_4_live_install_invoke` cold rerun; claim-6 still needs `claim_6_live_cross_tenant` cold rerun**".

These two tests gate the LIKELY-LIVE → LIVE verdict flip for claims 4 and 6 in the mission ledger (CLAUDE.md Status block). Flipping both to LIVE materially advances the v1.0 demo-readiness checklist.

## Scope

1. **Locate the two test functions**:
   - `rg -l 'fn claim_4_live_install_invoke' crates/`
   - `rg -l 'fn claim_6_live_cross_tenant' crates/`
   Note the crate + test file each lives in.

2. **For each test, run a cold cargo test on current `origin/main`**:
   - `CARGO_TARGET_DIR=/tmp/cold-claim-4-target cargo test --package <crate> --test <file> claim_4_live_install_invoke -- --nocapture --include-ignored` (drop `--include-ignored` if not `#[ignore]`'d).
   - Same recipe for claim 6 with `/tmp/cold-claim-6-target`.
   - Unique `CARGO_TARGET_DIR` per test is **load-bearing** — do NOT contaminate active worker caches (`/tmp/argo-shared-target/` and `$HOME/.cache/argo-shared-target/`). Per memory `memory_pressure_safeguards`, running cargo on a shared cache while workers are active can trigger jetsam.
   - Each test gets its own target dir so test 1's lock-files don't block test 2 if both compile in parallel; runs are still sequential.

3. **Capture full stdout + stderr** per test → attach as fenced blocks in the PR body. Include wall-clock compile time + test runtime per `cargo test`'s own output.

4. **If both PASS**:
   - Edit `CLAUDE.md` "Status 2026-05-25 (post interactive merge-train session)" block.
   - Flip Claim 4 from `🟢 **LIKELY-LIVE**` → `🟢 **LIVE**`.
   - Flip Claim 6 from `🟢 **LIKELY-LIVE**` → `🟢 **LIVE**`.
   - Update the verdict-justification text inline to cite this cold-rerun (date + brief id).
   - Commit the CLAUDE.md flip as a separate commit in the same PR.

5. **If either FAILs**:
   - Do NOT flip CLAUDE.md.
   - Document the failure mode (compile error / runtime panic / assertion failure) in PR body.
   - Hypothesize a root cause (1-2 sentences).
   - Author a follow-up brief: `fix-claim-<N>-live-test-<short-symptom>.md`. File it as a new TODO row in BUILD_STATUS.md under the DEMO-RECORDING UNBLOCK section.

6. **Clean up** — `rm -rf /tmp/cold-claim-4-target /tmp/cold-claim-6-target` post-run to free disk.

## Acceptance criteria

- Both test invocations produce a definitive PASS or FAIL (no compile errors that prevent the test from running — those count as FAIL with a follow-up brief).
- PR body has both compile-time and test-runtime wall-clock numbers per test.
- If PASS: CLAUDE.md Status block updated for both claims.
- If FAIL: a follow-up `fix-claim-<N>-live-test-*` brief is authored AND added to BUILD_STATUS.md.
- No active worker's cache touched (verify by checking the `CARGO_TARGET_DIR` env var was set; do not rely on cwd cargo defaults).

## May-touch

- `CLAUDE.md` — Status block update (only if both PASS)
- This brief file (status updates)
- New brief `docs/build-plan/streams/fix-claim-<N>-live-test-*.md` (only if FAIL)
- `docs/build-plan/BUILD_STATUS.md` — new TODO row (only if FAIL)

## Out of scope

- Touching the test code (this is a read-only re-run; if the test needs editing, that's the follow-up brief's job)
- Touching any mission-code path the tests exercise
- Running other test suites (focus is the two named LIVE tests)
- Re-running claim-2 / claim-3 / claim-5 (those already have evaluator PASSes per CLAUDE.md)

## Effort

~30-90 minutes (cold compile + 2 test runs; depends on warm/cold cache state of `~/.cache/argo-shared-target`; the two target dirs are isolated from each other but rustc may share via sccache).
