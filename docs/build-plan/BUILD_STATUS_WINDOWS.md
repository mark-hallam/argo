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

## Queue (priority order — first eligible TODO claimed first)

| Brief | Owner | Status | PR/branch | Notes |
|---|---|---|---|---|
| win-build-smoke-and-error-inventory | windows-fleet | TODO | — | ****P1 FOUNDATION — discovery.**** Dep: none. Brief: streams/win-build-smoke-and-error-inventory.md. |
| win-portability-cred-credentials-unix-perms | windows-fleet | TODO | — | ****P1 PORTABILITY (won't-compile).**** Dep: none. Brief: streams/win-portability-cred-credentials-unix-perms.md. |
| win-portability-cli-handler-unix-perms | windows-fleet | TODO | — | ****P1 PORTABILITY (won't-compile).**** Dep: none. Brief: streams/win-portability-cli-handler-unix-perms.md. |
| win-portability-runtime-tool-unix | windows-fleet | TODO | — | ****P1 PORTABILITY (won't-compile).**** Dep: none. Brief: streams/win-portability-runtime-tool-unix.md. |
| win-portability-tmp-and-home-paths | windows-fleet | TODO | — | ****P1 PORTABILITY.**** Dep: none. Brief: streams/win-portability-tmp-and-home-paths.md. |
| win-computer-use-executor-first-build | windows-fleet | TODO | — | ****P1 CROWN JEWEL — first real Windows compile.**** Dep: none. Brief: streams/win-computer-use-executor-first-build.md. |
| win-computer-use-executor-smoke | windows-fleet | TODO | — | ****P2 — first real Windows run.**** Dep: win-computer-use-executor-first-build MERGED. Brief: streams/win-computer-use-executor-smoke.md. |
| win-test-suite-run-and-failure-inventory | windows-fleet | TODO | — | ****P2 — discovery.**** Dep: win build/portability briefs largely MERGED. Brief: streams/win-test-suite-run-and-failure-inventory.md. |
| win-line-ending-and-path-sep-test-audit | windows-fleet | TODO | — | ****P2 PORTABILITY.**** Dep: none. Brief: streams/win-line-ending-and-path-sep-test-audit.md. |
| win-ci-windows-build-job-authored | windows-fleet | TODO | — | ****P3 — future-guard.**** Dep: none. Brief: streams/win-ci-windows-build-job-authored.md. |
