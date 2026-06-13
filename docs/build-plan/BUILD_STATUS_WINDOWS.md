# BUILD_STATUS — WINDOWS FLEET

**Separate ledger for the Windows worker fleet (operator-locked 2026-06-13).**
A Windows machine drains this queue in
parallel with the macOS swarm. The two fleets share `origin/main` but read
DIFFERENT ledgers, so they never race on the same brief:

- macOS swarm  → `docs/build-plan/BUILD_STATUS.md`
- Windows fleet → THIS file

## Fleet rules (every Windows worker)
1. Claim the **first eligible `TODO` row by document order** below. Flip it to
   `CLAIMED` (your machine + UTC), `git add` THIS file only, commit
   `status: claim <brief>`, `git push` (fast-forward; on non-ff: fetch+rebase
   THIS file, retry — that is how claims serialize across machines).
2. Build the brief on a branch `win-<brief>`; **verify ON REAL WINDOWS**
   (cargo build/test/clippy for the touched crate — that Windows signal is the
   whole point of this fleet).
3. Push the branch. Flip the row to `IN_REVIEW` with the branch name. **Do NOT
   self-merge** — the macOS conductor gates + adversarial-reviews + merges
   every Windows PR (single review chokepoint; portability cfg-gating can
   silently break the unix path, so it must be reviewed on the unix side).
4. NO THEATER: if a Windows primitive can't work yet, the cfg(windows) branch
   says so honestly — never fake a pass.
5. When this ledger drains, pull TODOs from the main `BUILD_STATUS.md`
   (any OS-agnostic backlog row) so the fleet never idles.

## Update 2026-06-13 (post first Windows build, #2900)
First real Windows build: only 2 of ~95 crates fail. (1) `kryptering` HSM dep
→ NEW blocker brief `win-kryptering-cfg-gate-hsm-signing-backend` (top of queue).
(2) the CU-executor (already covered). The `*-unix-perms` briefs were STALE
(code already cfg-gated) — cred one shipped as an observable-gap warn (#2901),
cli-handler + runtime-tool marked STALE. `tmp-and-home` stays (runtime, not
compile). CU-executor first-build is the crown jewel once the binary links.

## Queue (priority order — first eligible TODO claimed first)

| Brief | Owner | Status | PR/branch | Notes |
|---|---|---|---|---|
| win-kryptering-cfg-gate-hsm-signing-backend | windows-fleet | TODO | — | **P1 WINDOWS BLOCKER (from #2900 inventory) — the ONLY compile failure blocking the argo binary on Windows; kryptering/PKCS#11 CK_ULONG 32-bit. Feature-gate the HSM backend off on Windows, software-key fallback; unix HSM path unchanged. Brief: streams/win-kryptering-cfg-gate-hsm-signing-backend.md.** |
| win-build-smoke-and-error-inventory | windows-fleet | IN_REVIEW LATITUDE5440 2026-06-13T08:54Z | #2900 (win-build-smoke-and-error-inventory) | ****P1 FOUNDATION — discovery.**** Dep: none. Brief: streams/win-build-smoke-and-error-inventory.md. Deliverable: docs/skycot/windows/BUILD-ERRORS.md — 2/95 crates fail (kryptering→argo-cli NEW; cu-windows×10); unix-perm briefs confirmed STALE. |
| win-portability-cred-credentials-unix-perms | windows-fleet | IN_REVIEW LATITUDE5440 2026-06-13T09:12Z | #2901 (win-portability-cred-credentials-unix-perms) | ****P1 PORTABILITY.**** Dep: none. Brief: streams/win-portability-cred-credentials-unix-perms.md. NOTE: "won't-compile" premise STALE — code already cfg-gated; argo-config+argo-credentials compile + 176 tests pass cold on Windows. PR adds the one missing piece: one-shot warn on the non-unix perm-hardening gap (NOT a silent downgrade). |
| win-portability-cli-handler-unix-perms | windows-fleet | BLOCKED LATITUDE5440 2026-06-13T09:14Z | — | ****P1 PORTABILITY.**** Dep: NEW kryptering/cryptoki Windows fix (argo-cli won't compile on Windows — see PR #2900 / docs/skycot/windows/BUILD-ERRORS.md). UNVERIFIABLE on Windows until that lands: cargo build -p argo-cli fails at the kryptering dep, so the brief's verify can't run. Handlers spot-checked already `#[cfg(unix)]`-gated, so premise likely also STALE — re-validate after the pkcs11 fix. Brief: streams/win-portability-cli-handler-unix-perms.md. |
| win-portability-runtime-tool-unix | windows-fleet | CLAIMED LATITUDE5440 2026-06-13T09:16Z | — | ****P1 PORTABILITY (won't-compile).**** Dep: none. Brief: streams/win-portability-runtime-tool-unix.md. |
| win-portability-tmp-and-home-paths | windows-fleet | TODO | — | ****P1 PORTABILITY.**** Dep: none. Brief: streams/win-portability-tmp-and-home-paths.md. |
| win-computer-use-executor-first-build | windows-fleet | TODO | — | ****P1 CROWN JEWEL — first real Windows compile.**** Dep: none. Brief: streams/win-computer-use-executor-first-build.md. |
| win-computer-use-executor-smoke | windows-fleet | TODO | — | ****P2 — first real Windows run.**** Dep: win-computer-use-executor-first-build MERGED. Brief: streams/win-computer-use-executor-smoke.md. |
| win-test-suite-run-and-failure-inventory | windows-fleet | TODO | — | ****P2 — discovery.**** Dep: win build/portability briefs largely MERGED. Brief: streams/win-test-suite-run-and-failure-inventory.md. |
| win-line-ending-and-path-sep-test-audit | windows-fleet | TODO | — | ****P2 PORTABILITY.**** Dep: none. Brief: streams/win-line-ending-and-path-sep-test-audit.md. |
| win-ci-windows-build-job-authored | windows-fleet | TODO | — | ****P3 — future-guard.**** Dep: none. Brief: streams/win-ci-windows-build-job-authored.md. |
