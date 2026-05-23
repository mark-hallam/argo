# eval-argo-sandbox-deferred-status-pin

**Owner**: codex.
**Depends on**: none.
**Authored against**: origin/main `a39e6d6ee` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-argo-sandbox-deferred-status-pin`.

🟢 **P3 v1.x DEFERRAL HONESTY.** Pin the `argo-sandbox` + `argo-executor-computer-use` (E2B) DEFERRED-v1.x status with a structural assertion so post-v1.0 audits can confirm nothing crept in.

## Why

CLAUDE.md `v1.0 strategic decisions` declares:
- `argo-executor-computer-use-macos` is the only OS-native executor v1 hardens.
- E2B, Linux, and Windows executors are **DEFERRED-v1.x** — they stay shipped, just not v1-polished.
- `argo-sandbox` is the E2B microVM substrate, also DEFERRED-v1.x.

The gap-manifest carries these as `🔴 NOT AUDITED` / `🟢 P3 (v1.x)` — which is true today but provides zero guard if a worker accidentally promotes one of them via "small refactor" PR. We need a cheap structural pin that locks the deferral until operator explicitly flips the flag.

Pattern: same idea as the `connector_inventory_pin_test.rs` introduced by `eval-mcp-connector-real-vs-stub-inventory` — a single test that asserts "the deferred set is exactly this list, fail loudly if drift."

## Scope

1. **Read the workspace `Cargo.toml`** + the v1.0 strategic-decisions section of `CLAUDE.md` + the relevant streams files (`streams/claude-defer-non-macos-computer-use.md`).
2. **Add a single pin test** at `crates/argo-sandbox/tests/v1_deferred_status_pin.rs` (or sibling location if codex judges another crate fits better) that asserts:
   - `argo-sandbox`'s only public callers from `argo-cli` / `argo-agent-runner` / `argo-orchestrator` are zero (or the documented "shipped but unused" set).
   - `argo-executor-computer-use` (E2B) crate is in the workspace but has no production caller path; the only references are tests and capability declarations.
   - `argo-executor-computer-use-linux` + `argo-executor-computer-use-windows` are similarly callerless.
3. **Pin assertions inspect the actual call graph** (via grep over `crates/`) — not just "compile passes". If a new caller appears, the test fails with a clear message: "v1.x-deferred executor `<name>` has acquired a new caller `<file:line>`; either remove the caller or update CLAUDE.md to flip the deferral."
4. **Audit-only**. If a real caller already exists today that the manifest claims is absent, ship as `#[ignore]`'d FINDINGS-ONLY and surface a fix-brief.

## Anti-theater requirement

The pin must **fail loud on drift**. Forbidden: `assert!(true)`, `assert_eq!(callers.len() >= 0, true)`. The test should read the production crates from the filesystem (via `cargo metadata` or a shellout to `grep -r "use argo_sandbox" crates/`) and emit a structured diff against the locked allowlist.

A reviewable failure-message snippet:

```
panic: v1.x-deferred crate argo-sandbox has 1 new caller:
  - crates/argo-agent-runner/src/foo.rs:42  (added since v1.0 deferral)
Either:
  (a) remove the caller — argo-sandbox is DEFERRED-v1.x per CLAUDE.md.
  (b) update CLAUDE.md to flip the deferral + update this test's allowlist.
```

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `crates/argo-sandbox/tests/v1_deferred_status_pin.rs` (NEW; or sibling location justified in PR body)
- `docs/build-plan/BUILD_STATUS.md` — standard row flip.

## Files you must NOT touch

- Production source in `crates/argo-sandbox/`, `crates/argo-executor-computer-use*/`. Audit-only.
- `CLAUDE.md` itself — flipping a deferral is operator-decisional, not swarm-decisional.

## Acceptance criteria

- [ ] Single pin test that fails loud on drift; passing today against current `origin/main`.
- [ ] Test reads actual call graph from `crates/` (not just compile-time references); allowlist documented inline.
- [ ] If real caller drift found: ship as `#[ignore]`'d FINDINGS-ONLY + name a fix-brief.
- [ ] `cargo clippy -p argo-sandbox --all-targets -- -D warnings` clean (or the crate that hosts the test).
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Estimated size

~100-250 LOC of test code + a small allowlist file. Single crate. Fast-mode-eligible.
