# n-of-m-bind-approver-identity-and-self-approval-exclusion

**Owner**: claude.
**Authored against**: origin/main `502b76bc0` on `2026-06-08`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh n-of-m-bind-approver-identity-and-self-approval-exclusion`.
**Builds on**: the OPEN PR #2747 branch `claude-n-of-m-multi-step-approval` (the N-of-M plumbing). Base your work on that branch (rebased on current `origin/main`), do NOT re-implement the tally from scratch.

## Why (conductor adversarial review of #2747, 2026-06-08 — HIGH, blocks merge)
PR #2747 built the N-of-M threshold-approval plumbing correctly: distinct-vote tally, tenant-scoped, migration v92 (additive), hash-chained audit, and the gate genuinely holds the action until N votes — all tested. **But it is NOT yet a trustworthy separation-of-duties control, for two reasons, so it was HELD (not merged):**

1. **[HIGH] Approver identity is not bound to the signed approval token.** `crates/argo-cli/src/handlers/gateway/routes_channels.rs` `verify_token` HMACs only `approval_id | decision | exp`. The `actor` (who approved) is supplied by each channel handler, NOT carried in the token: push hardcodes `"push-operator"` (`routes_channels_push.rs`), email reads the inbound From (`routes_channels_email.rs`, spoofable), slack reads the payload (`routes_channels_slack.rs`). Because `ThresholdRule::is_authorized` returns `true` for everyone when `approvers` is empty (the documented "any authenticated approver" default, `argo-identity/src/auto_approval.rs`), one person holding a SINGLE valid approve-token can POST it to the push path (actor=`push-operator`) AND the email path (actor=`attacker@x`) → two DISTINCT actor strings → threshold=2 met by one human. The `UNIQUE(tenant_id, approval_request_id, approver)` constraint only stops the same actor STRING voting twice; it does nothing against one principal minting two actor strings. This defeats the entire point of N-of-M.

2. **[HIGH] No self-approval / requester-exclusion.** Neither `routes_approvals_threshold.rs` nor `ThresholdRule` compares a voter against the action's requester/creator, so the requester can cast one of the N approve votes for their own sensitive action. Separation of duties requires the requester be excluded from the approving set.

This is the SAME root-class fix the operator locked on WS5 (DEC: bind capability/authority to IDENTITY, never trust a self-asserted string/header). The first-principles version of N-of-M is one where the N approvers are cryptographically-distinct, authenticated humans — not N distinct strings.

## Grep-before-write
- `rg -n 'fn sign_approval_token|fn verify_token|struct .*Claims|approval_id|decision|exp' crates/argo-cli/src/handlers/gateway/routes_channels.rs crates/argo-channels/src/token.rs` — find the token mint+verify + the Claims shape. **Trace where approve-tokens are MINTED** (the approval-request fan-out to channels) to learn whether one token is minted per (approver, request) or one per request. The fix differs:
  - If minted per-approver already → add the approver/`sub` identity into the HMAC claims at mint, verify it at settle, and reject when the channel-derived actor ≠ the token's bound approver.
  - If minted per-request (no per-approver notion) → N-of-M is unenforceable without per-approver tokens; add a per-approver token issuance (one signed token per named approver) as part of this brief.
- `rg -n 'is_authorized|approvers|ThresholdRule|requester|created_by|requested_by' crates/argo-identity/src/auto_approval.rs crates/argo-cli/src/handlers/gateway/routes_approvals_threshold.rs` — find the authorization + where the action's requester identity is available.
- `rg -n 'settle\(' crates/argo-cli/src/handlers/gateway/routes_channels*.rs` — every channel settle callsite that must pass the authenticated approver, not a constant.

## Fix (first-principles: distinct = cryptographically-distinct authenticated humans)
1. **Bind approver identity into the token.** Add an `approver` (and/or `sub`) field to the signed approval token's HMAC input. At settle, derive the approver from the TOKEN (not the channel payload) — or, if the channel must supply it, REJECT when the channel-derived actor ≠ the token-bound approver. A replayed token then always resolves to the SAME approver → counts once. Keep back-compat for the single-approver path (a threshold-absent approval still settles).
2. **Require real approver identity for threshold votes.** For any approval with `threshold > 1`, require a non-empty `approvers` allowlist AND that the voter be one of them (by the token-bound identity), rejecting the empty-allowlist "anyone" default that makes (1) exploitable.
3. **Exclude the requester.** Record the action's requester/creator identity; reject (or do-not-count) a vote whose approver == requester. Document it.
4. Do NOT weaken anything that passed review (tenant-scoping, additive migration, audit chaining, the held-until-N gate).

## Acceptance (truth-bounded — the exploit must be closed)
1. New adversarial tests (these are the load-bearing proofs):
   - **same-principal-replay-across-channels does NOT reach threshold**: one approve-token settled via two different channel actor strings counts as ONE distinct approver (threshold=2 stays HELD). This is the exact #2747 exploit — it must now fail to finalize.
   - **requester cannot self-approve**: a vote by the requester is rejected / not counted.
   - **distinct authenticated approvers DO reach threshold** (the happy path still works).
   - keep #2747's passing tests (idempotent same-actor, reject-denies, cross-tenant isolation, held-until-N) green.
2. `cargo test -p argo-cli -p argo-identity -p argo-channels` green (paste tails). `cargo check -p argo-cli --bin argo` exit 0.
3. `cargo clippy` clean on touched files (boundary-note ONLY the documented pre-existing `await_holding_lock` in `routes_settings_credentials*.rs`).
4. Flags `ARGO_DUTY_AUTOFIRE` / `ARGO_EXECUTION_GRADUATION` stay OFF; no production-default change beyond the token-format hardening.

## On ship (same commit)
Update `docs/build-plan/FEATURE_MATRIX.md` N-of-M row to ✅ Live (the COMPLETE separation-of-duties control: approver-identity-bound, self-approval-excluded) + regen `docs/skycot/feature-matrix.html` with `python3 scripts/render-feature-matrix.py`. Flip the BUILD_STATUS.md `n-of-m-multi-step-approval` row BLOCKED→IN_REVIEW (and reference this brief). Note in the PR it closes the conductor review-HIGH on #2747.

## Effort
M–L (token-format change across mint + verify + all channel settle callsites + requester-exclusion + adversarial tests). Security-critical: review the result adversarially again before merge.
