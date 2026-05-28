# PR #2353 post-merge audit — truth-boundary code preserved

**Audited against**: `origin/main` at `206086a0e` (2026-05-28).
**Brief**: `fix-pr-2353-rebase-and-recover` (P3 belt-and-suspenders verification).
**Trigger**: evaluator round 84 noted that `git diff main` on the pre-rebase
state of PR #2353 (`fix-audit-feed-include-self-hash-in-envelope`) showed the
Track P P-02 truth-boundary files (added by #2295) as "removed". Evaluator
concluded GitHub's 3-way merge would preserve them; PR was merged at T10-05.
This is the post-merge verification.

## Result: ✅ PRESERVED — no recovery needed

All 5 acceptance checks pass on current `origin/main`:

| # | Check | Result |
|---|---|---|
| 1 | `git log -1 -- crates/argo-agent-runner/src/turn_helpers_truth_boundary.rs` | `ed0d34785 enforce truth boundary at turn exit (#2295)` — HEAD is #2295, not a revert |
| 2 | `git log -1 -- crates/argo-agent-runner/src/turn_helpers_truth_boundary_tests.rs` | `ed0d34785 ...` — same |
| 3 | `git log -1 -- crates/argo-capability/src/refusal_registry.rs` | `ed0d34785 ...` — same |
| 4 | `cargo test -p argo-agent-runner truth_boundary` | 11 passed, 0 failed (all #2295 tests intact) |
| 5 | `rg "ClaimAllowed\|ClaimBlocked\|ClaimEscalated" crates/argo-audit/src/event_kind.rs` | All 3 variants present (lines 28, 30, 32) |

GitHub's 3-way merge behaved as the evaluator predicted. The `git diff main`
anomaly on the pre-rebase PR branch was a presentation artifact of comparing
two branches that diverged before #2295 landed — the merge commit itself
correctly carried both #2295 and #2353's changes forward.

## Tests run (full list)

All 11 `truth_boundary` tests in `argo-agent-runner` passed in 0.99s:

- `truth_boundary_channel_parser_accepts_manifest_spellings`
- `truth_boundary_refusal_variant_is_deterministic`
- `truth_boundary_no_policy_short_circuits_without_audit`
- `truth_boundary_escalated_claim_enqueues_approval_and_audit`
- `truth_boundary_channel_scope_allows_rehearsal`
- `truth_boundary_run_turn_with_tools_gates_final_once`
- `truth_boundary_cross_tenant_policy_does_not_gate`
- `truth_boundary_channel_scope_forbids_web_embed`
- `truth_boundary_allowed_claim_passes_with_audit`
- `truth_boundary_provenance_reaches_audit_payload`
- `truth_boundary_forbidden_claim_uses_refusal_and_audit`

No follow-up fix-* brief required. Ledger this brief as MERGED-AS-VERIFIED.
