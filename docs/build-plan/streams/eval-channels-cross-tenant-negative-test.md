# eval-channels-cross-tenant-negative-test

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `<latest>` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-channels-cross-tenant-negative-test`.

🟡 **P1 — argo-channels substrate scoped but ZERO cross-tenant tests.** Source: evaluator round-15 SHARED.md handoff 2026-05-23T13:30:00Z.

> "argo-channels deep walk (9597 LOC, 35 files): substrate scoped — ProductionMessageDispatcher carries explicit tenant_id + outbound_message SQL binds it; per-channel modules workspace-boundary-isolated like argo-credentials. 130 lib tests pass. 🔴 GAP: rg 'cross.tenant|tenant_a|tenant_b' crates/argo-channels/ returns ZERO matches — same shape as round-12 argo-orchestrator (SQL right, no regression test pins it). Recommend P1 brief: eval-channels-cross-tenant-negative-test (≥3 tests: outbound dedup, approval token cross-secret, Slack inbound cross-secret)."

The same shape as round-3 audit-export bug + round-12 orchestrator gap: SQL substrate is correct today, but no regression test pins the contract. Without tests, a future refactor silently drops the predicate.

## Scope

Add ≥3 cross-tenant negative tests for `argo-channels`:

1. **Outbound dedup**: tenant_a sends approval token to channel `slack-1`; tenant_b sends DIFFERENT approval to channel `slack-1` with same dedup key → assert tenant_b's send is NOT deduped against tenant_a's (each tenant has independent dedup namespace).

2. **Approval token cross-secret**: tenant_a creates approval with token T; tenant_b tries to consume token T → assert refusal (404 byte-identical to nonexistent-token per #2068 pattern).

3. **Slack inbound cross-secret**: tenant_a's Slack signing secret S_a; inbound webhook signed with tenant_b's secret S_b → assert refusal at signature-verify boundary, audit row emitted with `tenant_mismatch=true`.

Plus the byte-identical-NotFound assertion across all 3 (per #2091's pattern for orchestrator).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `NEW: crates/argo-channels/tests/cross_tenant_test.rs` (~200-350 LOC).
- `crates/argo-channels/Cargo.toml` — dev-deps if needed.
- `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md` — flip argo-channels row.
- `docs/build-plan/BUILD_STATUS.md` — standard row flip.

## Files you must NOT touch

- `crates/argo-channels/src/**` — strict read-only audit. If real leak found, ship `#[ignore]`'d + FINDINGS-ONLY note.

## Acceptance criteria

- [ ] ≥3 cross-tenant negative tests (dedup + approval token + Slack inbound).
- [ ] Byte-identical NotFound assertion per #2068/#2091 pattern.
- [ ] Production source untouched.
- [ ] Gap manifest argo-channels row 🔴 → 🟢 (or kept 🟡 if findings).
- [ ] If real leak found → `#[ignore]`'d + FINDINGS-ONLY note + fix-brief recommendation.
- [ ] `cargo clippy -p argo-channels --all-targets -- -D warnings` clean.
- [ ] Every modified file <500 LOC.
- [ ] BUILD_STATUS.md updated.

## Branch + PR

- Branch: `eval-channels-cross-tenant-negative-test`
- PR title: `test(channels): cross-tenant outbound/approval/inbound negatives (eval-channels-cross-tenant-negative-test)`

## Notes for the implementer

- ~200-350 LOC. 3 tests × ~50-100 LOC each.
- The Slack inbound test is the most interesting — it actually exercises the HMAC-signature-verify path with the wrong tenant's secret.
- Cite round-15 verbatim + reference #2091 as the orchestrator twin.
