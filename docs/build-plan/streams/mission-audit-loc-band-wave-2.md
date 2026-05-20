# Stream Audit-P2-4 — split files in the 450-500 LOC band (proactive wave 2)

**Owner**: codex.
**Depends on**: PR #1790 (`codex-fn-loc-followup-wave1`) MERGED — that PR is splitting the top-5 fn-LOC violators. This brief tackles the next priority tier: files (not functions) close to the 500-LOC ceiling.
**Authored against**: origin/main `d2e4a579` on `2026-05-19`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-audit-loc-band-wave-2`.
**Source**: 2026-05-19 codex audit P2-3, validated by claude `VALIDATION.md` (CONFIRMED — 40 Rust files in 450-500 LOC band, top 10 listed below).

## AMENDED 2026-05-20 by conductor (post-BLOCK)

Original brief BLOCKED 2026-05-20 by codex pre-implementation: routes_dispatch_post.rs is now at 302 LOC (out of the 450-500 band — incidental collateral split from #1790's fn-LOC work). Effective splits = 9, not 10. Brief's "<30 final count" target was infeasible: 43 baseline - 9 effective splits = ~34 best case.

Conductor adjustments:
- Baseline updated 40 → 43 (current measured count).
- Target updated `<30` → `<35` to match the 9-effective-splits ceiling.
- Future wave-3 brief will pick up the remaining drift toward <30. The 5 next-in-band candidates (turn.rs, workflow_definition.rs, cli_session_cmds.rs, encryption.rs, plus a 5th in-band file) are out of scope for THIS wave-2 — they get their own brief once wave-2 ships.

Original 10 cited files: 9 still in band, 1 (routes_dispatch_post.rs) already at 302 LOC. Worker may either (a) skip routes_dispatch_post entirely (the file is already split), or (b) include it as a no-op confirmation of the existing state.

## Scope

CLAUDE.md mandates `<500 LOC per file`. **43 files** are currently in the 450-500 band (baseline grew since brief authored) — every small security fix in those files risks pushing one over the ceiling and forcing an unplanned split. Splitting proactively, on customer-critical paths, reduces ship-blocking churn during the security-hardening pass.

The 10 highest-priority files (mix of "biggest" + "most touched by security work"):

| File | LOC | Why priority |
|---|---|---|
| `crates/argo-cli/src/handlers/worker_actions_notify.rs` | 498 | Touched by multi-channel approvals work + notify dispatch hardening |
| `crates/argo-cli/src/handlers/gateway/routes_dispatch_post.rs` | 498 | Already the largest fn (post-#1790 split — sibling LOC still high) |
| `crates/argo-cli/src/handlers/doctor.rs` | 485 | Diagnostic surface; frequently extended |
| `crates/argo-cli/src/handlers/gateway/auth_magic_link.rs` | 484 | Touched by every auth-perimeter brief |
| `crates/argo-genesis/src/broad_role.rs` | 483 | Touched by `mission-audit-genesis-jd-llm-summarizer` |
| `crates/argo-compliance/src/export.rs` | 483 | Touched by audit-export work |
| `crates/argo-cli/src/handlers/gateway/sso_saml.rs` | 481 | Enterprise IAM; tight ceiling pressure |
| `crates/argo-skill/src/store.rs` | 480 | Touched by `mission-pm-skill-overlay-load` (P0) — split FIRST among these to avoid blocking the P0 |
| `crates/argo-store/src/approvals.rs` | 479 | Approval lifecycle; touched by content-preview work |
| `crates/argo-cli/src/handlers/gateway/ws_inbox_events_tests.rs` | 485 | Test file; lower priority but easy split (extract test scenarios into siblings) |

Split each file into 2-3 sibling modules using the same pattern PR #1790 used: extract logical groups (parsing, dispatch, rendering, persistence) into `_<topic>.rs` siblings, re-export from the parent for back-compat.

## Parallelism

- Manager-worker allowed: **yes** (this is the type of brief that benefits from a manager-worker split — one "wave 2" parent brief spawning 10 child split-PRs)
- Max children: 10
- Max depth: 2
- Auto-spawn: manual (operator decides whether to take all 10 or a subset)
- Manager package: implementer authors a tracking issue listing the 10 child splits + claims them sequentially or hands to multiple workers

## Files you may touch

For each of the 10 cited files: the file itself plus 1-2 sibling files extracted from it. Naming convention: if `worker_actions_notify.rs` splits into a payload-builder + a dispatch path, the siblings are `worker_actions_notify_payload.rs` + `worker_actions_notify_dispatch.rs`. Parent re-exports the public API; child modules are `pub(super)` or `pub(crate)` as appropriate.

## Files you must NOT touch

- Public API surface — split is **internal-only**. Every existing `pub` symbol stays at its current path. Use re-exports.
- Test fixtures — tests get split only when the test file itself is over the band (e.g. `ws_inbox_events_tests.rs` in the list).
- Module-level documentation — keep the parent module's docstring; reference the children below it.

## Acceptance criteria

For EACH child PR (10 total):

- [ ] Original file < 350 LOC after split (gives 150 LOC of future-growth headroom).
- [ ] Each new sibling < 450 LOC (stays in safe band).
- [ ] No public API change — `cargo public-api` (if configured) or `cargo doc --no-deps` produces identical output.
- [ ] No test changes other than file-path renames if a test moves.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] `cargo test -p <affected-crate>` passes.
- [ ] BUILD_STATUS.md updated with PR number per child.

For the parent (this brief):

- [ ] All 10 child splits land (or operator explicitly closes some as "leave alone — scheduled for refactor inside another brief").
- [ ] (Informational, not acceptance) Final count of files in 450-500 band trends DOWN from baseline — exact post-split count depends on how many cited files drop out of band, and may not reach any specific target if new code has accreted into the band during the brief's lifetime. **The PER-FILE criteria above are the load-bearing acceptance**; aggregate band count is a guidance metric only.

## Branch + PR

Per child:
- Branch: `codex-mission-audit-loc-band-wave-2-<short-file-tag>` (e.g. `codex-mission-audit-loc-band-wave-2-notify`)
- PR title: `refactor: split <file> into sibling modules (mission-audit-loc-band-wave-2 / <short-file-tag>)`

## Notes for the implementer

- **Order matters**: split `argo-skill/src/store.rs` FIRST. The PM teammate P0 brief (`mission-pm-skill-overlay-load`) needs to add a method to that file; if it adds the method to an already-480-LOC file, the file pushes over 500. Pre-splitting unblocks the P0.
- Test files in the band: extract individual test scenarios into separate `<topic>_tests.rs` siblings. Use `#[test]` modules per scenario for grep-ability.
- Use the exact PR #1790 pattern (read that PR's diff for reference). Don't reinvent.
- This brief is operator-discretionary — if the swarm is bandwidth-constrained, ship the top 3 (`store.rs`, `auth_magic_link.rs`, `worker_actions_notify.rs`) and defer the rest. Document which were deferred in the parent PR description.

## Out of scope (follow-up briefs)

- Files currently UNDER 450 LOC — they're not at risk.
- The next 30 files in the band (40 minus the top 10) — wave 3 if needed.
- Architectural splits (one crate into two, etc.) — out of scope; this is file-level only.
- Updating `scripts/check-loc-budget.sh` to enforce a tighter ceiling (e.g. 400) — premature.

## AMENDMENT 2 (2026-05-20 post-second-BLOCK by conductor)

Worker self-blocked again 2026-05-20T01:13Z after the first amendment because the aggregate "<35" target was still unreachable on current main (baseline drifted 43 → 44; effective 9 splits → 37 in band, not 35).

**Diagnosis**: aggregate-band-count targets are intrinsically unstable. The band grows organically as new code lands during the brief's lifetime, making any aggregate target a moving goalpost. The structurally-honest acceptance is per-file: "each cited file ends < 350 LOC after split; each sibling < 450 LOC". Those are deterministic + always reachable.

**Action**: aggregate-count line demoted to informational. The per-file acceptance criteria carry the load.

**Salvage path**: worker pushed branch `codex-mission-audit-loc-band-wave-2-v2` at commit `bc091f38` with all in-scope cited splits done + `bash scripts/codex-pre-push.sh` passing. A fresh worker (or the original worker on next iteration) should open the PR against current main with that branch's content. No re-implementation needed.
