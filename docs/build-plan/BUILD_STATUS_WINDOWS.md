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
| win-cu-executor-wire-into-resolve-executor | windows-fleet | IN_REVIEW LATITUDE5440 2026-06-13T17:08Z | #2921 (win-cu-executor-wire-into-resolve-executor) | ****P1 — DEC-054 keystone: make Windows a real executor target.**** Wave-3 (deep buffer; DEC-054 for the executor-wire). Dep: none. Brief: streams/win-cu-executor-wire-into-resolve-executor.md. |
| win-cu-executor-end-to-end-run | windows-fleet | TODO | — | ****P2 — first real model-in-loop CU run on Windows (DEC-054).**** Wave-3 (deep buffer; DEC-054 for the executor-wire). Dep: 
## Goal
With the executor wired (sibling brief), run the FULL computer-use loop on a
real Windows host: a real `run()` driving screenshot→model→action→screenshot.
This is the first end-to-end proof Windows is a live CU target.

## Do (real Windows + an ANTHROPIC_API_KEY)
Drive one small real task (e.g. "open Notepad and type a sentence") through the
wired dispatch path. Capture evidence into docs/skycot/windows/CU-E2E-RUN.md:
the real screenshot bytes (now decodable PNG, #2919), the model's chosen
actions, the input effect (clipboard read-back like #2910), and the
truth-bounded/approval behavior. NO THEATER — real run or honest gap, never a
mocked loop.

## Verify
The doc shows a genuine end-to-end Windows CU run with real screenshot + real
input effect. Branch + IN_REVIEW.
. Brief: streams/win-cu-executor-end-to-end-run.md. |
| win-gateway-e2e-on-windows | windows-fleet | IN_REVIEW Latitude5440 | #2920 (win-gateway-e2e-on-windows) | ****P2 — does the full stack SERVE on Windows.**** DONE: gateway boots+serves on real Windows — /health+/readyz+/livez+/version 200; auth-gated /audit+/workspace round-trip 200 (auth ON → 401, gate works); doc docs/skycot/windows/GATEWAY-E2E.md. GAPs: provider-key boot gate; debug binary lacks bundled-sqlcipher so sessions.db is PLAINTEXT (SQLCipher-at-rest on MSVC unverified); binary 2434e59 (42 behind main, no gateway-serve path changed). Wave-3 (deep buffer; DEC-054 for the executor-wire). Dep: none. Brief: streams/win-gateway-e2e-on-windows.md. |
| win-distribution-packaging-smoke | windows-fleet | IN_REVIEW Latitude5440 | #2922 (win-distribution-packaging-smoke) | ****P2 — the single-binary distribution story on Windows.**** DONE: fresh RELEASE argo.exe built from main (a19eaca), 81,619,456 B single self-contained exe, runs from a clean dir (argo --version/--help OK). Distribution = zip of the one argo.exe (release.yml.disabled); no MSI/installer. PE-import parse: only non-OS dep is VCRUNTIME140.dll (VC++ redist); flagged +crt-static as a follow-up. Doc docs/skycot/windows/DISTRIBUTION.md. Wave-3 (deep buffer; DEC-054 for the executor-wire). Dep: none. Brief: streams/win-distribution-packaging-smoke.md. |
| win-path-canonicalization-deep-audit | windows-fleet | IN_REVIEW LATITUDE5440 2026-06-13T17:47Z | #2924 (win-path-canonicalization-deep-audit) | ****P2 — UNC / drive-letter / long-path audit (paths were the dominant Windows bug class).**** Wave-3 (deep buffer; DEC-054 for the executor-wire). Dep: none. Brief: streams/win-path-canonicalization-deep-audit.md. |
| win-line-endings-gitattributes-pin | windows-fleet | IN_REVIEW Latitude5440 | #2923 (win-line-endings-gitattributes-pin) | ****P2 — defensive CRLF pin (CRLF was suspected 3x; pin it shut).**** DONE: root .gitattributes pins byte-sensitive+source/data/text classes to LF (text eol=lf), marks binaries `binary`. Surgical — only 1 pre-existing CRLF file (calendly.svg, unmatched), ZERO renormalization. VERIFIED on Windows: check-attr eol=lf on migration .sql; CRLF .sql normalized to i/lf on git add; git status shows only .gitattributes. Doc docs/skycot/windows/LINE-ENDINGS.md. Wave-3 (deep buffer; DEC-054 for the executor-wire). Dep: none. Brief: streams/win-line-endings-gitattributes-pin.md. |
| win-ci-windows-test-job-expand | windows-fleet | IN_REVIEW Latitude5440 | #2925 (win-ci-windows-test-job-expand) | ****P3 — expand the disabled Windows CI to the full surface.**** DONE: windows-build.yml.disabled now guards the full surface — workspace build + EXPLICIT CU-executor crate build+test (crown jewel, incl. PNG-encode test) + workspace test; stays .disabled; policy row updated. VERIFIED on Windows: YAML valid (7 steps); cargo test -p argo-executor-computer-use-windows = 52/52 pass. Wave-3 (deep buffer; DEC-054 for the executor-wire). Dep: none. Brief: streams/win-ci-windows-test-job-expand.md. |
| win-full-workspace-test-suite-now-all-compiles | windows-fleet | CLAIMED Latitude5440 2026-06-13T17:52Z | — | ****P1 — definitive Windows test pass (newly unblocked).**** Wave 2 (post full-Windows-build). Brief: streams/win-full-workspace-test-suite-now-all-compiles.md. |
| win-binary-runs-smoke-doctor-and-cli | windows-fleet | CLAIMED LATITUDE5440 2026-06-13T17:49Z | — | ****P2 — does the binary actually RUN on Windows (not just compile/test).**** Wave 2 (post full-Windows-build). Brief: streams/win-binary-runs-smoke-doctor-and-cli.md. |
| win-capability-fs-cap-glob-separator | windows-fleet | MERGED #2913 | #2913 (win-capability-fs-cap-glob-separator) | ****P1 CROSS-PLATFORM CORRECTNESS (potential silent deny) — from #2911.**** From #2911 Windows test inventory. Brief: streams/win-capability-fs-cap-glob-separator.md. |
| win-identity-fs-root-path-portability | windows-fleet | MERGED #2914 | #2914 (win-identity-fs-root-path-portability) | ****P1 CROSS-PLATFORM CORRECTNESS — from #2911.**** From #2911 Windows test inventory. Brief: streams/win-identity-fs-root-path-portability.md. |
| win-process-registry-spawn-portability | windows-fleet | MERGED #2915 | #2915 (win-process-registry-spawn-portability) | ****P1 CROSS-PLATFORM — from #2911 (verify production, not just tests).**** From #2911 Windows test inventory. Brief: streams/win-process-registry-spawn-portability.md. |
| win-store-cascade-and-migration-crlf-investigate | windows-fleet | MERGED #2917 | #2917 (win-store-cascade-and-migration-crlf-investigate) | ****P2 INVESTIGATE — from #2911.**** From #2911 Windows test inventory. Brief: streams/win-store-cascade-and-migration-crlf-investigate.md. |
| win-cu-screenshot-real-png-encode | windows-fleet | MERGED #2919 | #2919 (win-cu-screenshot-real-png-encode) | **P2 (from #2910 smoke) — DONE: replaced the 16-byte stub with real GetDIBits readback + image-crate PNG encode (BGRA→RGBA). VERIFIED on real desktop: monitor#... 1920x1080 → 691,981-byte PNG that decodes to 1920x1080 (real_pixels). Hermetic unit test (exact pixels) + new passive tests/windows_screenshot_real.rs + strengthened windows_smoke asserts; clippy -D warnings clean. CU-EXECUTOR-SMOKE.md gap #1 CLOSED. Brief: streams/win-cu-screenshot-real-png-encode.md.** |
| win-kryptering-cfg-gate-hsm-signing-backend | windows-fleet | MERGED #2905 | #2905 (win-kryptering-cfg-gate-hsm-signing-backend) | **P1 WINDOWS BLOCKER (from #2900 inventory) — the ONLY compile failure blocking the argo binary on Windows; kryptering/PKCS#11 CK_ULONG 32-bit. Feature-gate the HSM backend off on Windows, software-key fallback; unix HSM path unchanged. Brief: streams/win-kryptering-cfg-gate-hsm-signing-backend.md.** |
| win-build-smoke-and-error-inventory | windows-fleet | MERGED #2900 | #2900 (win-build-smoke-and-error-inventory) | ****P1 FOUNDATION — discovery.**** Dep: none. Brief: streams/win-build-smoke-and-error-inventory.md. Deliverable: docs/skycot/windows/BUILD-ERRORS.md — 2/95 crates fail (kryptering→argo-cli NEW; cu-windows×10); unix-perm briefs confirmed STALE. |
| win-portability-cred-credentials-unix-perms | windows-fleet | MERGED #2901 | #2901 (win-portability-cred-credentials-unix-perms) | ****P1 PORTABILITY.**** Dep: none. Brief: streams/win-portability-cred-credentials-unix-perms.md. NOTE: "won't-compile" premise STALE — code already cfg-gated; argo-config+argo-credentials compile + 176 tests pass cold on Windows. PR adds the one missing piece: one-shot warn on the non-unix perm-hardening gap (NOT a silent downgrade). |
| win-portability-cli-handler-unix-perms | windows-fleet | MERGED #2912 | #2912 (win-portability-cli-handler-unix-perms) | ****P1 PORTABILITY.**** Dep: NEW kryptering/cryptoki Windows fix (argo-cli won't compile on Windows — see PR #2900 / docs/skycot/windows/BUILD-ERRORS.md). UNVERIFIABLE on Windows until that lands: cargo build -p argo-cli fails at the kryptering dep, so the brief's verify can't run. Handlers spot-checked already `#[cfg(unix)]`-gated, so premise likely also STALE — re-validate after the pkcs11 fix. Brief: streams/win-portability-cli-handler-unix-perms.md. |
| win-portability-runtime-tool-unix | windows-fleet | MERGED #2902 | #2902 (win-portability-runtime-tool-unix) | ****P1 PORTABILITY.**** Dep: none. Brief: streams/win-portability-runtime-tool-unix.md. NOTE: compile premise STALE, BUT found a REAL prod bug — path_guard `..` defense-in-depth no-opped on Windows verbatim (\\?\) paths; fixed (4 traversal tests flipped green). argo-runtime 105/105 green + oom_restart Windows-doc honesty. 11 remaining argo-tool fails are OUT OF SCOPE: 9 path-sep test assertions → win-line-ending-and-path-sep-test-audit; 2 process_registry shell tests → NEW finding (recommend a brief). |
| win-portability-tmp-and-home-paths | windows-fleet | MERGED #2904 | #2904 (win-portability-tmp-and-home-paths) | ****P1 PORTABILITY.**** Dep: none. Brief: streams/win-portability-tmp-and-home-paths.md. NOTE: /tmp half is all test/doc literals (→ line-ending brief); REAL fix = home resolution was HOME-only (broke native Windows `~/.argo` → bare relative path) in argo-config expand_tilde + argo-credentials dirs_home_dir — added USERPROFILE fallback (unix byte-identical, no new dep). 110+28 tests green + clippy clean on Windows. |
| win-computer-use-executor-first-build | windows-fleet | MERGED #2906 | #2906 (win-computer-use-executor-first-build) | ****P1 CROWN JEWEL — first real Windows compile.**** Dep: none. Brief: streams/win-computer-use-executor-first-build.md. DONE: crate COMPILES + `clippy -D warnings` CLEAN on Windows (10 windows-0.58 fixes: Win32_Security feat; IUIAutomationElement is_invalid→map_err ×2; HWND.0 is_null; MONITORINFOF_PRIMARY local const; BitBlt Result is_ok; WTS handle un-Some). NO THEATER: encode_bitmap_as_png still a pre-existing stub (PNG-magic+dims, not pixels); nothing RUN — runtime is the smoke brief. Report: docs/skycot/windows/CU-EXECUTOR-BUILD.md. |
| win-computer-use-executor-smoke | windows-fleet | MERGED #2910 | #2910 (win-computer-use-executor-smoke) | ****P2 — first real Windows run.**** Dep: win-computer-use-executor-first-build MERGED. Brief: streams/win-computer-use-executor-smoke.md. |
| win-test-suite-run-and-failure-inventory | windows-fleet | MERGED #2911 | — | ****P2 — discovery.**** Dep: win build/portability briefs largely MERGED. Brief: streams/win-test-suite-run-and-failure-inventory.md. |
| win-line-ending-and-path-sep-test-audit | windows-fleet | MERGED #2908 | #2908 (win-line-ending-and-path-sep-test-audit) | ****P2 PORTABILITY.**** Dep: none. Brief: streams/win-line-ending-and-path-sep-test-audit.md. HEAD START (from PR #2902): argo-tool has 9 path-sep test-assertion fails — path_guard::tests::{resolve_relative_joins_onto_root, resolve_absolute_passes_through_unchanged (note: "/etc/passwd" isn't absolute on Windows — needs cfg or platform-abs path), resolve_parent_climb_lands_outside_root, resolve_curdir_and_nested_climb_normalized}; tools::{file_patch,file_read×2,file_write×2}::in_ctx_* . Production output is correct OS-native; only test literals are unix-centric. |
| win-ci-windows-build-job-authored | windows-fleet | MERGED #2918 | #2918 (win-ci-windows-build-job-authored) | ****P3 — future-guard.**** Dep: none. Brief: streams/win-ci-windows-build-job-authored.md. |
