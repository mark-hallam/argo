---
date: 2026-05-25
round: 69
pillar: store / gateway health (Claim 6 multi-tenant by construction — schema-version safety rail)
verdict: 🟡 PARTIAL — code-side P0 RESOLVED via #2331 merge; ops-side restart NOT YET RUN, gateway still HTTP 503
severity: P1 (downgraded from r67/r68 P0 because root cause is now fixed in code; remaining work is a process restart, not a code change)
pr: 2331 (MERGED as commit 8abb093b1 at 2026-05-26 04:10:31 +1000 = 2026-05-25T18:10:31Z UTC)
---

# Round 69 — PR #2331 EXPECTED_SCHEMA_VERSION 82→83 — P0 fix MERGED, but live gateway binary still stale → 503 persists

## Setup

Reading order:

1. `tail -60 docs/skycot/agents/SHARED.md` showed previous evaluator round = 68; conductor `dec9a7d12 status: tick T17-11` authored α-r68 brief `streams/fix-pr-2317-bump-expected-schema-version-to-83.md` mid-round in response to my r67/r68 P0 escalation.
2. `git fetch origin main --quiet && git log --oneline origin/main -25` revealed **PR #2331 merged as `8abb093b1`** at ~18:10Z (~2min before my round 69 start at ~18:12Z UTC).
3. `gh pr view 2331 --json mergeable,mergeStateStatus,additions,deletions,changedFiles` returned `MERGEABLE / CLEAN, +1/-1, 1 file` *before* I refreshed git log — confirms PR was clean-mergeable, then conductor (presumably T18-xx tick) merged it.
4. `git show --stat 8abb093b1` confirmed merged diff scope: 1 file, +1/-1 in `crates/argo-store/src/lib.rs` only — no scope creep.

## Observed (post-merge source verification)

PR #2331 final merged diff:

```diff
diff --git a/crates/argo-store/src/lib.rs b/crates/argo-store/src/lib.rs
@@ -361,7 +361,7 @@ pub type StoreResult<T> = Result<T, ArgoError>;
-pub const EXPECTED_SCHEMA_VERSION: u32 = 82;
+pub const EXPECTED_SCHEMA_VERSION: u32 = 83;
```

This is exactly the α-r68 brief shape I recommended in round 68 (`1-LOC, bump 82→83`). No collateral changes.

**Tests that derive from `EXPECTED_SCHEMA_VERSION`** (all now align with v83 migration registered at `crates/argo-store/src/migrations/registry_compliance.rs:141`):

- `crates/argo-store/src/lib_tests.rs:11`
- `crates/argo-store/src/approvals_tests.rs:65, 80`
- `crates/argo-store/src/migration_reconciliation_tests.rs:15, 122, 179, 228, 253, 330`

That's 9 callsites that previously failed `assert_eq!(store.schema_version().await.unwrap(), EXPECTED_SCHEMA_VERSION)` on origin/main (actual=83 after v83 migration ran, EXPECTED=82 pre-bump → `assertion failed`). Post-merge they assert 83 == 83. Anti-theater check: not a vacuous fix — `store_open.rs:279` `match actual.cmp(&EXPECTED_SCHEMA_VERSION)` is the load-bearing check that powers `verify_schema_version` which `/health` reports via the `degraded` reason field.

**Grep confirms no leftover hardcoded `82`** in code (only `crates/argo-store/src/migrations/registry_compliance.rs:136` which is the v82 migration's own version field — correct):

```bash
$ grep -rn "schema.*82\|82.*schema\|version: 82\|version=82" --include="*.rs" --include="*.toml" --include="*.md" crates/
crates/argo-store/src/migrations/registry_compliance.rs:136:            version: 82,
```

## Observed (live gateway state — STILL DEGRADED)

```bash
$ curl -s http://127.0.0.1:7400/health
{"status":"degraded","reason":"schema_version_mismatch"}
HTTP 503

$ curl -s "http://127.0.0.1:7400/api/v1/audit?limit=1" | head -1
{"items":[{"action_type":"hipaa_compliance_check",...}],"next_cursor":"..."}
HTTP 200
```

`/health` says degraded; `/api/v1/audit` returns real data (one row at `id=019e5dbaec03…7f3f41e`, timestamp `2026-05-25T06:03:11.747736+00:00`).

**Why?** The running gateway binary is stale.

```bash
$ ps -eo pid,etime,command | grep argo.*gateway
37695 12:09:46 /tmp/argo-shared-target/release/argo gateway start --bind 127.0.0.1 --port 7400
```

PID 37695, uptime 12h09m — started at ~2026-05-25T06:02Z UTC. The PR #2331 fix merged at 18:10Z. The running binary was compiled with `EXPECTED_SCHEMA_VERSION = 82` (literal frozen in `.text`). Database is at v83 (v83 migration shipped earlier via #2317 / `dc6d224eb` and ran). Hence: `actual=83.cmp(&82) == Greater` → `"schema_version=83 but binary expects 82; schema is ahead of the binary — binary is too old, roll back"` per `store_open.rs:285`.

The fix is **on disk in source**; the running gateway is **NOT YET serving from a binary built post-merge**.

## Anti-theater check

Three independent confirmations the fix is real and not vacuous:

1. **Source diff is the minimum mechanical change** — no test rewrite, no comment-only change, no constant-renamed-elsewhere. The single constant `pub const EXPECTED_SCHEMA_VERSION: u32 = 83` is the one referenced by `store_open.rs:279` (`actual.cmp(&EXPECTED_SCHEMA_VERSION)`), by 9 test callsites, and is exported in `lib.rs` so external crates (`argo-cli`, the gateway boot path) see the same value.
2. **v83 migration is fully registered**: `crates/argo-store/src/migrations/registry_compliance.rs:141` calls `version: 83, up: |tx| Box::pin(v83::up(tx))` — the migration runner at `migrations/registry.rs:20` `run()` will pick it up and write 83 to `schema_version` table. `crates/argo-store/src/migrations/v83.rs` defines the legacy-tenant quarantine SQL.
3. **The 503 will resolve as soon as the gateway binary is rebuilt + the process restarted** — `scripts/restart-live-gateway.sh` is documented as canonical (per CLAUDE.md "(b)" line about #2283). This is a mechanical operator/conductor step, not a code change. If I were re-doing the fix-brief author work, I'd note **NEVER rely on a long-running binary picking up a const-change without restart** — but this isn't a regression risk in the PR itself, it's a deployment-cadence gap.

## Gap / follow-up brief candidates

| Priority | Brief shape | Scope |
|---|---|---|
| 🟢 P0 OPS (operator/conductor action, NOT new brief) | Run `bash scripts/restart-live-gateway.sh` to rebuild binary + restart PID 37695. Expected post-restart state: `/health` → HTTP 200, `{"status":"ok","schema_version":83}`. | 0 LOC code change; pure ops |
| 🔴 P2 (carry from r68 — γ-r68) | `fix-check-gateway-freshness-sh-distinguish-503-from-unreachable` — currently `scripts/check-gateway-freshness.sh` returns `UNREACHABLE` on the 503 response (verified this round: ran the script, got `STATUS: UNREACHABLE  (no response from http://127.0.0.1:7400/health — gateway not running?)` despite gateway PID 37695 alive + serving `/api/v1/audit` HTTP 200). This is a false-negative that **masks the degraded state from conductor tick scans**. Fix: read HTTP status code — distinguish `000` (connection refused) from `5xx` (gateway alive but degraded). ~5 LOC bash. | `scripts/check-gateway-freshness.sh` |
| 🔴 P3 (carry from r68 — β-r68) | `ops-add-expected-schema-version-vs-tip-migration-ci-lint` — CI lint that diffs `EXPECTED_SCHEMA_VERSION` against the highest registered migration version in `crates/argo-store/src/migrations/registry_*.rs`. Would have caught #2317's missing bump pre-merge. | New `scripts/check-schema-version-matches-migration-tip.sh` + GHA wire-up (note: GHA bypassed per CLAUDE.md DISABLED_WORKFLOWS_POLICY — alternative is `conductor-pre-merge-gate.sh` injection) |
| 🟡 P3 NEW (ν-r69) | `ops-restart-live-gateway-on-schema-bump-merge` — extend `conductor-pre-merge-gate.sh` (or post-merge hook) to detect PRs that touch `EXPECTED_SCHEMA_VERSION` and trigger `scripts/restart-live-gateway.sh` automatically after admin-merge. Removes the manual restart step + closes the "fix merged, ops not done" gap from this round. ~20 LOC bash. | `scripts/conductor-pre-merge-gate.sh` or new `scripts/post-merge-restart-on-schema-bump.sh` |

## Mission impact

Code-side: ✅ Schema-version safety rail (Claim 6 multi-tenant by construction prerequisite — `verify_schema_version` is the gate that refuses to bind against a divergent schema, preventing cross-tenant data corruption on stale-binary boot). Restored.

Live-side: 🟡 Until gateway restarts, `/health` is degraded but reads (`/api/v1/audit`) succeed because the schema is *forward-compatible* with the v82-era binary for most operations — the binary only refuses on `actual > expected` at boot or on explicit `verify_schema_version` calls, not on every query. Risk: if the v83 migration's `audit_record.tenant_id = '__legacy__'` rewrite changed any path the v82 binary assumes about tenant_id shape, we'd see runtime errors on filtered reads. Spot-check above (`/api/v1/audit?limit=1` HTTP 200 with a real row) suggests no immediate breakage, but the operator should restart the gateway to close this window.

## Recommendation for conductor tick post-this-round

1. **🟢 P0 OPS**: Run `bash scripts/restart-live-gateway.sh` to make the fix live. Until then, the demo MP4 recording path (per CLAUDE.md "Remaining for v1.0 launch" item (d)) is blocked because Scene-1 will hit a 503 banner if the runbook checks `/health`.
2. **🔴 P2**: Author γ-r69 (carry from r68) `fix-check-gateway-freshness-sh-distinguish-503-from-unreachable` — 4th consecutive eval round flagging this script's false-negative.
3. **🔴 P3**: Author β-r69 (carry from r68) `ops-add-expected-schema-version-vs-tip-migration-ci-lint`.
4. **🟡 P3 NEW**: Consider ν-r69 `ops-restart-live-gateway-on-schema-bump-merge` to make this class of incident self-healing.

**Round-67/r68 P0 escalation chain SUCCEEDED**: 3 evaluator rounds (r67 P1 → r68 P0 → r68 mid-round conductor brief-author → r69 merged). Total wall: r67 ship at ~16:09Z → r69 verified merged at ~18:12Z = ~2h03m end-to-end. Code-side: closed. Ops-side: pending restart.

55th anti-theater verification this week (now r69's first).
