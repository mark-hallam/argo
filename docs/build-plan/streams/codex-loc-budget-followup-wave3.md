# codex-loc-budget-followup-wave3 — proactive split, 450-500 LOC band wave 3

**Owner**: codex. **Sizing**: 5 split PRs (one per file) × ~150-300 LOC per child.
**Authored against**: origin/main `68e88222` on `2026-05-20`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-loc-budget-followup-wave3`.
**Source**: follow-up to `mission-audit-loc-band-wave-2` (operator-locked 2026-05-20 wave-3 placeholder); same pattern, different 5 files.
**Depends on**: `mission-audit-loc-band-wave-2` MERGED (so the same files don't get split twice).

## Scope

CLAUDE.md mandates `<500 LOC per file`. After wave-1 (#1790) and wave-2 (in flight), 44 files remain in the 450-500 band. This brief picks the next 5 NOT covered by wave-2 to bring the band count below 30 (the original ambition from wave-2's authoring).

The 5 files (mix of "biggest non-wave-2 sibling" + "production-critical"):

| File | LOC | Why priority |
|---|---|---|
| `crates/argo-agent-runner/src/turn.rs` | 488 | **Production-critical** — the agent loop's per-turn driver. Touched by skill-loop closure, tool-trace emission, mission-claim-4 wiring. High ceiling-pressure risk. |
| `crates/argo-store/src/workflow_definition.rs` | 487 | **Newly added via #1846** — preempt growth before downstream briefs add fields. Logical split: read-path vs write-path siblings. |
| `crates/argo-cli/src/cli_session_cmds.rs` | 486 | Session-management CLI command dispatch. Likely split by command-group (approve, list, status, etc.) into sibling modules. |
| `crates/argo-store/src/approvals_tests.rs` | 497 | Test file; biggest in band. Extract per-scenario `#[test]` modules into siblings. |
| `crates/argo-store/src/encryption.rs` | 478 | **Security-critical** — SQLCipher + key-handling code. Touched by upcoming `mission-audit-sqlcipher-enterprise-default`. Split into key-derivation, encrypt-path, decrypt-path siblings. |

Split each file into 2-3 sibling modules using the same pattern as wave-2 + PR #1790: extract logical groups (parsing, dispatch, rendering, persistence) into `_<topic>.rs` siblings; re-export from the parent for back-compat.

## Parallelism

- Manager-worker allowed: **yes**
- Max children: 5
- Max depth: 2
- Auto-spawn: manual (operator decides whether to take all 5 or a subset)
- Manager package: implementer authors a tracking issue listing the 5 child splits + claims them sequentially or hands to multiple workers

## Files you may touch

For each of the 5 cited files: the file itself plus 1-2 sibling files extracted from it. Naming convention: if `turn.rs` splits into a step-driver + a tool-dispatch path, the siblings are `turn_step_driver.rs` + `turn_tool_dispatch.rs`. Parent re-exports the public API; child modules are `pub(super)` or `pub(crate)` as appropriate.

## Files you must NOT touch

- Public API surface — split is **internal-only**. Every existing `pub` symbol stays at its current path. Use re-exports.
- Test fixtures — only `approvals_tests.rs` is being split because the test file itself is over the band. Other test files are out of scope.
- Module-level documentation — keep the parent module's docstring; reference the children below it.

## Acceptance criteria

For EACH child PR (5 total):

- [ ] Original file < 350 LOC after split (gives 150 LOC of future-growth headroom).
- [ ] Each new sibling < 450 LOC (stays in safe band).
- [ ] No public API change — `cargo public-api` (if configured) or `cargo doc --no-deps` produces identical output.
- [ ] No test changes other than file-path renames if a test moves.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] `cargo test -p <affected-crate>` passes.
- [ ] BUILD_STATUS.md updated with PR number per child.
- [ ] Final count of files in 450-500 band drops **below 30** (current baseline 44; wave-2's 9 effective splits + this brief's 5 = 14, taking 44 → 30. Even if 4 of 5 ship, count drops to ~31.)

## Branch + PR

- Branch: `codex-loc-budget-followup-wave3-<short-file-tag>` (e.g. `codex-loc-budget-followup-wave3-turn`)
- PR title: `refactor: split <file> into sibling modules (codex-loc-budget-followup-wave3 / <short-file-tag>)`
- PR body: cite this brief + the parent intent + the LOC-after-split numbers.

## Notes for the implementer

- **`turn.rs` is the highest-risk split** — touch with care. Pre-implementation: grep all callers of every public fn in `argo-agent-runner::turn`; if any caller is in must-NOT-touch territory, surface as BLOCKED before starting.
- **Test files**: extract individual `#[test]` scenarios into separate `<topic>_tests.rs` siblings. Use `#[test]` modules per scenario for grep-ability.
- **Encryption split** is security-sensitive — preserve all existing test coverage; do NOT change the SQLCipher init order or the key-derivation path. If the natural split puts key-derivation in a sibling, ensure the parent's `init_encryption` callers still trigger the same exact sequence.
- **Workflow_definition split**: read-path (lookup, list_versions) vs write-path (create_version, archive_version, advance_head) is the natural boundary. Migration code stays in the parent.
- **Sequencing**: if you take ≤3 of the 5, prioritize `turn.rs` (production risk) and `encryption.rs` (security risk). The test-file + workflow_definition can wait.

## Out of scope

- Files in wave-2's 10-file scope (auth_magic_link, worker_actions_notify, doctor, broad_role, export, sso_saml, store, approvals, ws_inbox_events_tests + the already-split routes_dispatch_post).
- Files outside the 450-500 band (the 500+ band is already empty per HARD RULE 6; the <450 band is not under ceiling pressure).
- Any non-split refactoring — keep the diff a pure structural move.
