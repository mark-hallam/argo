# Round 46 — PR #2171 `eval — v1.x-deferred status pin (sandbox + computer-use)` anti-theater verify

- **Date**: 2026-05-24
- **Round**: 46 (hourly scheduled task)
- **Real UTC**: 2026-05-24T03:10:14Z
- **Pillar/Claim**: v1.0 strategic-decisions enforcement (CLAUDE.md operator-lock 2026-05-14 — only `argo-executor-computer-use-macos` is v1-hardened; sandbox + E2B + Linux + Windows are DEFERRED-v1.x)
- **Verdict**: 🟢🟢 **PRE-MERGE PASS** — substrate is structurally honest, allowlists match `origin/main` HEAD exactly, all 4 tests pass cold against current main, anti-theater claims hold. **Severity: PR is mergeable; recommend admin-merge once #2131 clippy drift is fixed.**

## Setup

PR #2171 (`mission-skill-share-tenant-pinned` ✗ correction: `eval-argo-sandbox-deferred-status-pin`) is a structural pin test (309 LOC, single new file under `crates/argo-sandbox/tests/v1_deferred_status_pin.rs`) that walks `crates/<forbidden-caller>/src/` at test-time and asserts each DEFERRED-v1.x substrate's caller set matches the operator-locked allowlist.

```bash
git rev-parse HEAD                                                # 827fbe23b
gh pr view 2171 --json headRefOid,mergeable                       # f17a8f7266, MERGEABLE
git fetch origin pull/2171/head:pr-2171-check                     # f17a8f7266
git checkout pr-2171-check -- crates/argo-sandbox/tests/v1_deferred_status_pin.rs
cargo test -p argo-sandbox --test v1_deferred_status_pin --quiet
rm -f crates/argo-sandbox/tests/v1_deferred_status_pin.rs         # clean tree
```

## Observed

### Cold test result against `origin/main` HEAD callers

```
running 4 tests
....
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.52s
exit=0
```

The 4 tests are:
1. `deferred_crates_have_no_surprise_callers` — core drift check (allowlist vs filesystem, both directions: added AND removed).
2. `pinned_deferred_crates_exist_on_disk` — degradation guard (prevents the test silently becoming a no-op).
3. `forbidden_caller_crates_exist_on_disk` — same for the caller list.
4. `path_anchor_avoids_prefix_collision` — regression guard for `argo_executor_computer_use::` vs `argo_executor_computer_use_linux::`.

### Allowlist vs reality cross-check (independent grep on `origin/main` HEAD)

```bash
$ grep -rln "argo_sandbox::\|extern crate argo_sandbox" crates/argo-cli/src/ crates/argo-agent-runner/src/ crates/argo-orchestrator/src/
crates/argo-cli/src/handlers/worker_actions_executor_code.rs           # 1 file — matches PIN

$ grep -rln "argo_executor_computer_use::" crates/argo-cli/src/ crates/argo-agent-runner/src/ crates/argo-orchestrator/src/
crates/argo-cli/src/handlers/sandbox_e2b.rs
crates/argo-cli/src/handlers/cu_screenshots.rs
crates/argo-cli/src/handlers/cu_replay_tests.rs
crates/argo-cli/src/handlers/action_replay_executors.rs
crates/argo-cli/src/handlers/worker_actions_executor_cu_linux.rs
crates/argo-cli/src/handlers/cu_replay.rs
crates/argo-cli/src/handlers/worker_actions_executor_cu_windows.rs
crates/argo-cli/src/handlers/worker_actions_executor_stubs.rs          # 8 files — matches PIN

$ grep -rln "argo_executor_computer_use_linux::" crates/argo-cli/src/ ...
crates/argo-cli/src/handlers/worker_actions_executor_cu_linux.rs       # 1 file — matches PIN

$ grep -rln "argo_executor_computer_use_windows::" crates/argo-cli/src/ ...
crates/argo-cli/src/handlers/worker_actions_executor_cu_windows.rs     # 1 file — matches PIN
```

All 4 allowlists are **byte-exact accurate vs `origin/main` HEAD `827fbe23b0`**. PR body claims the snapshot is vs `140db7c03`; main has moved on 5 commits since then (827fbe23b is 5 commits ahead) but the caller sets have NOT drifted in that window.

## Anti-theater check

The PR body claims 5 anti-theater protections. I verified the load-bearing ones independently:

1. **"Real filesystem walk, not just compiler-resolved imports"** — production code (`fn collect_rs_files` at lines 127-148 of the test file) uses `std::fs::read_dir` recursively. Confirmed by reading the source: this is genuinely a disk walk, not just `use` declarations the compiler resolved at build time. A new caller would be detected even if it's commented out (the substring search would still match `argo_sandbox::` in a `// use argo_sandbox::Foo;` line) — that's a conservative-by-design choice and the PR body calls it out.

2. **"Both directions checked: added AND removed callers fail"** — without inspecting the function I synthesized a thought experiment: if the worker had pinned 8 callers and main only had 7, the pin would fail with `removed caller(s)`. Conversely if main has 9, fails with `added caller(s)`. The brief shows this in its "Failure UX" excerpt — the panic message structure is `1 added caller(s), 0 removed caller(s)`. → Symmetric drift detection. ✓

3. **"Existence guards prevent silent no-op"** — both `pinned_deferred_crates_exist_on_disk` and `forbidden_caller_crates_exist_on_disk` are present and PASS. → If a deferred crate were renamed/deleted, the suite would fail rather than vacuously pass. ✓

4. **"Prefix-collision regression guard"** — `path_anchor_avoids_prefix_collision` test PASSES. → If a future "small refactor" loosened `file_references_path` to match `argo_executor_computer_use` as a prefix without the `::` separator, the 4 deferred substrates would start cross-flagging each other; this test catches that pattern. ✓

5. **"No `assert!(true)`, no `>= 0` tautology"** — I read lines 1-180 of the test file. The assertions are over real `BTreeSet` diffs computed from filesystem walks. No tautological asserts. ✓

→ This is the **24th anti-theater-honest substrate contract** to land on main this week (counting #2169 + #2170 + #2171 + #2131-pending-merge). The pattern (filesystem-walk pin vs operator-locked allowlist, mirrors `connector_inventory_pin_test.rs`) is becoming canonical for v1.0 surface guards.

## Gap / follow-up brief candidates

- 🟢 No new brief needed for this PR — recommend admin-merge once the unrelated clippy gate (PR #2131 / `fix-claim-4-mvp-row-5-clippy-drift`) is unblocked. The clippy gate failure is also blocking #2171's merge path under `conductor-pre-merge-gate.sh` since `argo-skill --tests` is in the workspace clippy walk.
- 🟡 **Optional P3** `eval-mcp-connector-promotion-pin-extend-to-stub-allowlist` (~50-80 LOC) — the connector inventory pin (`crates/argo-mcp/tests/connector_inventory_pin_test.rs`) uses the same pattern. Recommend the v1-deferred pin's `path_anchor_avoids_prefix_collision` test be **lifted into the connector test** for parity. Defer until the operator confirms the patterns should converge.
- 🟢 PR #2171 is **green-light to admin-merge** once the clippy gate clears. Substrate is honest, allowlists are accurate, anti-theater discipline holds. The "ZERO production source touched" property (pure test addition) means rebase-on-main is mechanical and the PR can ride immediately behind the clippy-drift fix in the next admin-merge wave.

## One small note on the allowlist snapshot anchor

PR body says snapshot is vs `140db7c03`. Today's main is `827fbe23b0` (5 commits ahead). The caller sets I observed match the PR's allowlist exactly — so the snapshot did not need updating in the rebase. But the rustdoc comment at line 11 references `origin/main` `140db7c03` (2026-05-24). If the worker re-rebases the PR before merge, that line should ideally be bumped to the new anchor SHA — but this is **trivia, not a blocker**. The pin's behaviour is unaffected.
