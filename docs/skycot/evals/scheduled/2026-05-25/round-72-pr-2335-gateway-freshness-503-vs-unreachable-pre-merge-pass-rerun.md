# Round 72 · #2335 gateway-freshness UNREACHABLE-vs-503 PRE-MERGE PASS rerun

**Date**: 2026-05-25 (Melbourne 2026-05-26 07:09 AEST)
**Round**: 72
**Pillar / Claim**: Process drift prevention (ops infrastructure; supports schema-bump carry-closure by giving the conductor a stable contract for "gateway up-but-degraded" vs "gateway dead").
**Verdict**: 🟢 **PASS** (P1 — closes the 5-round γ carry; ready for merge under overnight `--skip-clippy` auth)

## Setup

`#2335 ops: check-gateway-freshness.sh distinguishes HTTP 503 from UNREACHABLE` was PRE-MERGE PASS'd at r71 (20:20Z). r72 reruns the verification against the CURRENT PR head (`73ee841f4`, unchanged since r71) to confirm no regression intervened — and because the carry-closing PRs in the r71 batch are still open (#2335/#2336/#2337/#2338) while #2334 already merged.

PR state: OPEN · mergeable UNKNOWN (gh hasn't computed since the last push) · CLEAN diff (2 files: `check-gateway-freshness.sh` +24/-3, `check-gateway-freshness-tests.sh` rewritten to use env-var fakes instead of binding a Python HTTP server).

## Observed

**Production logic** (`scripts/check-gateway-freshness.sh` on PR head, lines 117-123):
```bash
if [[ "$HEALTH_STATUS" == "503" ]]; then
    printf 'gateway_schema=v%s codebase_latest=v%s drift=%s\n' \
        "$GATEWAY_SCHEMA_N" "$CODEBASE_LATEST_N" "$(( CODEBASE_LATEST_N > GATEWAY_SCHEMA_N ? CODEBASE_LATEST_N - GATEWAY_SCHEMA_N : 0 ))"
    printf 'STATUS: STALE-503  (gateway up but reporting schema_version_mismatch - restart binary: `bash scripts/restart-live-gateway.sh`)\n'
    exit 1
fi
```

Earlier in the same function (lines ~110-116), a 5xx response _without_ a parseable `schema_version` falls through to `STATUS: UNREACHABLE` exit 2 — so the 503-with-body case is the only path that triggers the new STALE-503 exit-1.

**Smoke test rerun** — fetched PR head into a temp branch, extracted both scripts into `/tmp/r72-pr2335-test/scripts/`, initialized a minimal git repo with `crates/argo-store/src/migrations/v82.rs` + `v83.rs` so the production script's `git rev-parse` + tip-migration glob both resolve. Ran the PR head's tests-script against the PR head's production script:

```
Scenario 0: gateway schema == codebase latest → exit 0 (FRESH).
  ✅ drift=0 → exit 0 + FRESH
Scenario 1: gateway 3 behind codebase → exit 1 (STALE).
  ✅ drift=3 → exit 1 + STALE
Scenario 2: /health returns HTTP 503 with schema_version → exit 1 (STALE-503).
  ✅ HTTP 503 + schema_version → exit 1 + STALE-503
Scenario 3: nothing listening on $PORT → exit 2 (UNREACHABLE).
  ✅ no listener → exit 2 + UNREACHABLE
Scenario 4: /health returns body without schema_version → exit 3.
  ✅ no schema_version in body → exit 3 + PARSE_ERROR
✅ All 5 scenarios passed.
```

**Live cross-check** against the running gateway (PID 98169, uptime ~1h since the r71 restart):
```
$ bash scripts/check-gateway-freshness.sh
gateway_schema=v83 codebase_latest=v83 drift=0
STATUS: FRESH  (drift <= 2)
```
(Note: the live script is main's, not the PR head's. The PR head changes the 503 path; the FRESH path is unchanged.)

## Anti-theater check

How I proved this isn't just "test rewrites itself green":

1. **Smoke harness changed shape, not just labels** — the PR replaces the Python HTTP-server fake with `FAKE_HEALTH_BODY` / `FAKE_HEALTH_STATUS` env-var hooks inside `check-gateway-freshness.sh` itself. That's a more honest harness: tests bypass the script's own curl, but the script's parsing + classification logic runs unchanged. The new Scenario 2 sets `FAKE_HEALTH_STATUS=503` + a body containing `schema_version` and asserts the production code emits `STALE-503` exit 1.
2. **Read the production branch** — lines 117-123 are the new code path; the conditional is `HEALTH_STATUS == "503"` (string literal, not regex), so HTTP 502/504 etc. still fall through to UNREACHABLE. This is intentional per the docstring contract ("STALE — gateway up but reporting schema_version_mismatch") — 503 specifically is what `routes_health.rs` emits when `STORED_SCHEMA_VERSION != EXPECTED_SCHEMA_VERSION`.
3. **The r66→r70 false-alarm pattern is the target** — during the 5-round γ carry, the gateway was returning HTTP 503 with `{"status":"schema_version_mismatch","schema_version":82,"expected":83}` and the freshness script was reporting `UNREACHABLE` (because the schema_version was inside an error envelope the old sed didn't catch). With this fix, the same scenario would print `STATUS: STALE-503` + the actionable `restart-live-gateway.sh` command in the message — exactly the operator nudge the carry was missing for 5 rounds.
4. **No `--skip` flag** in the PR's diff — the smoke runner is invoked from `make test-scripts` (or directly via `bash scripts/check-gateway-freshness-tests.sh`); no escape hatch.

## Gap / follow-up brief candidates

None new. The PR closes the γ-carry pattern cleanly.

**Recommendation for next conductor tick**: 🟢 P1 MERGE #2335 — PRE-MERGE PASS confirmed across r71 + r72 (no regression in 49min wall); falls under the overnight `--skip-clippy` ops-* pattern auth; ~1min admin-merge.

## Closure ledger

After #2335 merges:
- 4 of 5 r66→r70 carries are sealed (γ closed by both the live-gateway restart at r71 + the script fix in #2335; β closed by #2334 merge at r72 start; θ-r70 confirmed no-recurrence; z-r66 cosmetic-skip).
- 1 carry remains: **ω-r68 supervisor slot-1 respawn-after-rc-0 RECURRENCE-5** — see process-drift report.

**Cost**: ~4K tokens, ~5min wall for this scenario (most of it was the temp-checkout setup + smoke harness teardown).
