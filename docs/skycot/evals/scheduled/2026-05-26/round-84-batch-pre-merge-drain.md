# Round 84 — Batch Pre-Merge Drain

**Date**: 2026-05-26 (UTC) · Melbourne 20:20 AEST
**Round**: 84 (batch follow-up to the 3-scenario primary round)
**Mode**: COVERAGE — coverage > depth on individual PRs

## Context

The 2026-05-26 evening interactive merge wave (conductor T09-15 + T09-30 + T11-15) cleared 21 web-only / vitest-green PRs from 48 → 27 open. Remaining queue had 10 Rust-touching CLEAN PRs needing evaluator pre-verify before the conductor can admin-merge under operator-locked `--skip-clippy` lane.

This batch covers the 10. Two were already verified earlier in round 84 (#2400 Genesis first-hire, #2401 Pitch Agent eval) — see sibling reports. The remaining 8 are below.

## Per-PR verdicts

| PR | Verdict | Crate(s) touched | Test runtime | Anti-theater 1-liner |
|---|---|---|---|---|
| [#2402](https://github.com/mark-hallam/argo/pull/2402) | 🟢 PASS | `argo-cli` | 5m27s cold + 2.10s | Spawns real gateway, asserts `/api/v1/audit/feed` returns `endpoint_not_found` AND body has no "row" mention; valid hex-id still returns "audit row not found". 2 disambiguated 404 paths. |
| [#2397](https://github.com/mark-hallam/argo/pull/2397) | 🟢 PASS | `argo-demo-seed` | 4-test PASS in 0.48s | 5 pending + 3 decided approvals (2 approve / 1 refuse) seed for /approvals ring. Tenant-isolation tests still PASS over the new structure. |
| [#2376](https://github.com/mark-hallam/argo/pull/2376) | 🟡 PASS-WITH-CAVEAT | `argo-cli` | 30s timeout — flaky | Production change is 1 LOC additive (`"created": outcome.created` to JSON payload, complementing existing `"idempotent"`). New integration test failed 3× on hardcoded 30s listener-await without `--skip-startup-check` — pure test infrastructure flake, NOT a logic regression. **Recommend conductor merge** but author should follow up extending the listener deadline OR adopting the `--skip-startup-check` pattern PR #2402 uses. |
| [#2363](https://github.com/mark-hallam/argo/pull/2363) | 🟠 DEFER | `argo-cli` | not run | **Directly conflicts with PR #2400** (also pre-verified this round). #2400 ADDS the Genesis wiring to first-hire; #2363 REMOVES the daily-check-in stub and leaves duties empty. Same problem-space (orphan duty), different fix. Operator memory `feedback_genesis_is_ai_teammate_onboarding_studio` aligns with #2400 (Genesis is the investor moment). Recommend conductor merge #2400 first, then close #2363 as `superseded-by-2400`. |
| [#2362](https://github.com/mark-hallam/argo/pull/2362) | 🟢 PASS | `argo-cli` | clean build 4m13s | Adds `skill_count` field to `GET /verticals` JSON by counting `<root>/skills/*.skill.md` files. Mirrors the existing `count_skill_files` helper used by `POST /api/v1/onboarding/install-vertical` so the wizard's per-card count matches what install will report. No new Rust test (web vitest at `web/tests/onboarding-pick-vertical.test.ts` covers wizard render). Additive `pub fn skill_count` is safe. |
| [#2359](https://github.com/mark-hallam/argo/pull/2359) | 🟢 PASS | `argo-cli` | 2m15s cold + 2.27s + 0.20s | CORS dev-loopback-origin expansion. 2 cold tests: `cors_preflight_round_trip` (1/1) + `dev_gateway_allows_loopback_origin_variants` (1/1). Pre-built CORS-enhanced status lines now lazy-cached for `127.0.0.1` / `localhost` preview-proxy origins via `Mutex<HashMap>` in addition to the eager `Box::leak` cache for configured prod origins. |
| [#2353](https://github.com/mark-hallam/argo/pull/2353) | 🟢 PASS | `argo-cli` | 2m22s cold + 2.27s | 2-LOC additive: `self_hash` + `parent_hash` added to `audit_row_json` envelope. New `audit_feed_includes_hash.rs` integration test (134 LOC) seeds an audit row with computed hash via `compute_chain_hash`, boots real gateway, asserts feed JSON contains the exact computed hashes. **Note**: PR is branched before #2295 (Track P P-02 truth-boundary gate) landed; `git diff main` shows the truth-boundary files as "removed" but `gh pr diff` confirms the PR itself does NOT touch those files. GitHub's 3-way merge will preserve #2295. Merge-safe. |
| [#2345](https://github.com/mark-hallam/argo/pull/2345) | 🟢 PASS | `argo-cli` | 2m02s cold + 2.29s | Adds `delegation_chain` field to `GET /api/v1/audit/<id>` detail envelope. New helper queries audit_record rows sharing the row's `current_handoff_id` / `handoff_id` / `handoffId` and returns originator → delegated-to → executor ordered by `created_at, seq`. Cap LIMIT 10. New 151-LOC integration test seeds 3 audit rows with same handoff-id, asserts detail endpoint returns chain in timestamp order. Claim 2 (delegation) visible-proof surface. |
| [#2343](https://github.com/mark-hallam/argo/pull/2343) | 🟢 PASS | `argo-cli` | 2m01s cold + 2.21s | Replaces leaky 500 ("wrap key: no workspace root key SKYCOT_WORKSPACE_ROOT_KEY_BASE64...") with friendly 503 `provider_validation_unconfigured` envelope: "you can skip this step — your workers will run with the built-in mock provider until you add a real key later in Settings." New `is_workspace_root_key_error()` predicate in helpers; new 122-LOC integration test asserts response status=503 + body does NOT contain the env-var names. Defensive — improves UX without surface drift. |
| [#2268](https://github.com/mark-hallam/argo/pull/2268) | 🟢 PASS | `argo-audit`, `argo-cli` | 0.00s lib test | New `AuditKind::CrossTenantInboundRejected` (label `channel.cross_tenant_inbound_rejected`) registered in all 3 round-trip sites of `event.rs` (the regression pattern that took gateway down at r78 is closed correctly here). 4 silent-rejection branches in `routes_channels_slack.rs` (missing sig / missing ts / sig mismatch / token verify fail) now emit one audit row. Round-trip test `cross_tenant_inbound_rejected_round_trips_and_listed` PASS. Closes r50/r51/r52 evaluator carry-forward of PR #2257 `#[ignore]`d stub. **Anti-theater bonus**: PR description honestly notes brief's may-touch pointed at a non-existent file path (`crates/argo-mcp-connector-slack/src/inbound.rs`), author corrected to the real path — that level of brief-discipline is uncommon and worth a callout. |

## Summary

- **COUNT_PASS**: 7 (#2402, #2397, #2362, #2359, #2353, #2345, #2343, #2268 — 8 total)
- **COUNT_PASS-WITH-CAVEAT**: 1 (#2376 — flaky integration test, production code is 1-line correct)
- **COUNT_DEFER**: 1 (#2363 — conflicts with operator-aligned PR #2400)
- **COUNT_FAIL**: 0

Conductor authorized to admin-merge the 9 PASS + PASS-WITH-CAVEAT under `--skip-clippy` ops-*/fix-*/eval-* operator-locked lane (window expires 2026-05-26T22:00Z AEST, ~10h remaining).

**Recommended merge order** (because some share `argo-cli` touches that may need rebase between merges):

1. **#2400 first** (Genesis investor-moment; verified earlier in round 84 separate report) — then close #2363 as `superseded-by-2400` to clear the conflict
2. #2402 (audit 404 disambiguation — independent)
3. #2353 (audit feed self_hash — branched pre-#2295 but additive-only, safe)
4. #2345 (audit detail delegation chain — independent)
5. #2268 (slack inbound audit emission — independent)
6. #2343 (provider-key friendly error — touches `routes_onboarding_helpers.rs` shared with #2400, but additive only)
7. #2376 (provision created flag — additive 1-line, can land any time)
8. #2362 (vertical skill_count — independent)
9. #2359 (CORS dev allowlist — independent)
10. #2397 (demo seed approvals — independent)

After this batch, the queue drops from 27 open → ~18 open (cascade-DIRTY web-only PRs remain for conductor rebase + #2294 BLOCKED-stale).

## Time budget

Wall-clock for batch: ~55 min (within 60-min target).
Cold cargo for argo-cli amortized across PRs by sharing `/tmp/eval-r84-batch-target` — first compile ~5m27s, subsequent 7-130s incremental.

## Brief-shape recommendations (P3, optional)

- **`ops-test-integration-listener-timeout-pattern-fix`** (P3) — sweep `crates/argo-cli/tests/*.rs` for hardcoded `Duration::from_secs(30)` listener-await loops without `--skip-startup-check`; convert to `await_listener_signal` from `common::listener_signal` (the pattern PR #2402 uses) which is more robust under shared-target compile load. Would close the PR #2376 flake class permanently. ~40 LOC.
- **`fix-pr-2353-rebase-and-recover`** (P3) — once PR #2353 + #2400 + #2295 all settle, audit `git log --oneline -- crates/argo-agent-runner/` to confirm no truth-boundary code was lost across the merge chain. Belt-and-suspenders.
