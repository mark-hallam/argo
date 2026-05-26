# ops-claim-5-cold-rerun-mcp-and-cu-post-macos-perms

**Owner**: claude.
**Authored against**: origin/main `8eec48008` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh ops-claim-5-cold-rerun-mcp-and-cu-post-macos-perms`.

🟡 **P1 — v1.0 launch remaining item (c) from CLAUDE.md "Remaining for v1.0 launch" list. Now agent-doable.**

Operator granted macOS permissions on the `argo` binary (Accessibility + Screen Recording) on 2026-05-26 per operator note T20:55Z. The only remaining gate to flip Claim 5 (Any-PC execution) from 🟡 **LIKELY-LIVE-pending-cold-rerun** → 🟢 **LIVE** in the CLAUDE.md Status block is a cold rerun of the claim-5 LIVE test triad on current `origin/main`.

Mirror of recipe used for `ops-claim-4-and-6-live-cold-rerun-on-current-main` (MERGED via PR #2285).

## Scope

1. **Locate the three test functions** (already grep-confirmed at brief authoring time):
   - `crates/argo-executor-computer-use-macos/tests/claim_5_live_macos_cu_smoke_test.rs` — contains `claim_5_live_macos_cu_notes_smoke` + `claim_5_live_macos_cu_config_always_validates`
   - `crates/argo-workflow/tests/claim_5_live_mcp_cap_audit_test.rs`
   - `crates/argo-workflow/tests/claim_5_live_mcp_pool_routing_test.rs`

2. **For each test file, run a cold cargo test on current `origin/main`** with an isolated target dir:

   ```bash
   CARGO_TARGET_DIR=/tmp/cold-claim-5-cu-target \
     cargo test --package argo-executor-computer-use-macos \
       --test claim_5_live_macos_cu_smoke_test \
       -- --nocapture
   ```

   ```bash
   CARGO_TARGET_DIR=/tmp/cold-claim-5-mcp-cap-target \
     cargo test --package argo-workflow \
       --test claim_5_live_mcp_cap_audit_test \
       -- --nocapture
   ```

   ```bash
   CARGO_TARGET_DIR=/tmp/cold-claim-5-mcp-pool-target \
     cargo test --package argo-workflow \
       --test claim_5_live_mcp_pool_routing_test \
       -- --nocapture
   ```

   Unique `CARGO_TARGET_DIR` per test is **load-bearing** — do NOT contaminate active worker caches (`/tmp/argo-shared-target/` and `$HOME/.cache/argo-shared-target/`). Per memory `memory_pressure_safeguards`, running cargo on a shared cache while workers are active can trigger jetsam.

3. **Capture full stdout + stderr** per test → attach as fenced blocks in the PR body. Include wall-clock compile time + test runtime per `cargo test`'s own output.

4. **If all three PASS**:
   - Edit `CLAUDE.md` "Status 2026-05-25 (post interactive merge-train session)" block.
   - Flip Claim 5 from `🟡 **LIKELY-LIVE-pending-cold-rerun**` → `🟢 **LIVE**`.
   - Update the verdict-justification text inline to cite this cold-rerun (date + brief id).
   - Update CLAUDE.md "Remaining for v1.0 launch" item (c) to note the cold-rerun closure date.
   - Commit the CLAUDE.md flip as a separate commit in the same PR.

5. **If any FAILs**:
   - Do NOT flip CLAUDE.md.
   - Document the failure mode (compile error / runtime panic / assertion failure / macOS permission issue) in PR body.
   - Hypothesize a root cause (1-2 sentences) — macOS-perms-related failures should be flagged for operator follow-up since perm grant is operator-owned.
   - Author a follow-up brief: `fix-claim-5-live-test-<short-symptom>.md`. File it as a new TODO row in BUILD_STATUS.md.

6. **Clean up** — `rm -rf /tmp/cold-claim-5-cu-target /tmp/cold-claim-5-mcp-cap-target /tmp/cold-claim-5-mcp-pool-target` post-run to free disk.

## Acceptance criteria

- All three test invocations produce a definitive PASS or FAIL (no compile errors that prevent the test from running — those count as FAIL with a follow-up brief).
- PR body has compile-time and test-runtime wall-clock numbers per test.
- If all PASS: CLAUDE.md Status block updated for Claim 5 + item (c) note updated.
- If any FAIL: a follow-up `fix-claim-5-live-test-*` brief is authored AND added to BUILD_STATUS.md.
- No active worker's cache touched (verify by checking the `CARGO_TARGET_DIR` env var was set; do not rely on cwd cargo defaults).

## May-touch

- `CLAUDE.md` — Status block update + item (c) note (only if all PASS)
- This brief file (status updates)
- New brief `docs/build-plan/streams/fix-claim-5-live-test-*.md` (only if FAIL)
- `docs/build-plan/BUILD_STATUS.md` — new TODO row (only if FAIL)

## Out of scope

- Touching the test code (this is a read-only re-run; if the test needs editing, that's the follow-up brief's job)
- Touching any mission-code path the tests exercise
- Running other test suites (focus is the three named LIVE test files)
- Re-running claim-2/3/4/6 (already verified per CLAUDE.md)
- macOS perm re-grant (operator-owned; already done per operator note 2026-05-26T20:55Z)

## Effort

~30-90 minutes (cold compile + 3 sequential test runs; depends on warm/cold cache state of `~/.cache/argo-shared-target`; the three target dirs are isolated from each other but rustc may share via sccache).
