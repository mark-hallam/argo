# Round 73 — PR #2335 gateway 503-vs-UNREACHABLE classification: POST-MERGE anti-theater

- **Date**: 2026-05-25 (Melbourne 2026-05-26 08:09→08:30 AEST)
- **Round**: 73 (scheduled-task hourly cron)
- **Pillar / Claim**: process observability (γ-carry closure proof — rounds 66→72 false-alarm pattern)
- **Verdict**: 🟢 PASS · P2 (process)
- **Headline**: #2335 merged `030da5e94` at 2026-05-25T21:11:43Z (~3min after r72 ship). 5-scenario POST-MERGE drive of `scripts/check-gateway-freshness.sh` via `FAKE_HEALTH_STATUS` / `FAKE_HEALTH_BODY` test seam proves the new 503-classification branch fires on literal `503 + schema_version`, that the existing UNREACHABLE branches are preserved for `502`/`504` + `503 no-schema`, and that the baseline FRESH path is unchanged. The γ-carry (rounds 66→72) is now SEALED on its actual production wire, not just the test harness.

## Setup

PR #2335 merge commit `030da5e94`:

```
ops: classify gateway 503 freshness responses (#2335)
scripts/check-gateway-freshness-tests.sh | 138 +++++++++++--------------------
scripts/check-gateway-freshness.sh       |  38 ++++++++-
2 files changed, 80 insertions(+), 96 deletions(-)
```

Production wire (lines 116–123 of `scripts/check-gateway-freshness.sh`):

```bash
if [[ "$HEALTH_STATUS" == "503" ]]; then
    printf 'gateway_schema=v%s codebase_latest=v%s drift=%s\n' \
        "$GATEWAY_SCHEMA_N" "$CODEBASE_LATEST_N" "$(( CODEBASE_LATEST_N > GATEWAY_SCHEMA_N ? CODEBASE_LATEST_N - GATEWAY_SCHEMA_N : 0 ))"
    printf 'STATUS: STALE-503  (gateway up but reporting schema_version_mismatch - restart binary: `bash scripts/restart-live-gateway.sh`)\n'
    exit 1
fi
```

The 503 branch sits AFTER the no-schema-version 5xx fallback (line 107 `if [[ "$HEALTH_STATUS" =~ ^5 ]] then UNREACHABLE exit 2`). So a 503-with-schema falls through to the new branch; a 503-no-schema is caught earlier as UNREACHABLE. Literal-string `== "503"` (not regex `^503$` or `=~ ^50`) guarantees 502/504/500 fall through to the generic UNREACHABLE handler.

## Observed (5 scenarios)

| # | Scenario | FAKE_HEALTH_STATUS | FAKE_HEALTH_BODY | Expected | Observed rc | Observed output |
|---|---|---|---|---|---|---|
| A | FRESH baseline preserved | 200 | `{"status":"ok","schema_version":83}` | 0 / FRESH | **0** | `gateway_schema=v83 codebase_latest=v83 drift=0 / STATUS: FRESH  (drift <= 2)` |
| B | **NEW: 503 + schema → STALE-503** | 503 | `{"status":"degraded","schema_version":82}` | 1 / STALE-503 | **1** | `gateway_schema=v82 codebase_latest=v83 drift=1 / STATUS: STALE-503  (gateway up but reporting schema_version_mismatch - restart binary: …)` |
| C | 502 no-schema → UNREACHABLE | 502 | `server-error-no-body` | 2 / UNREACHABLE | **2** | `STATUS: UNREACHABLE  (/health returned HTTP 502 without schema_version)` |
| D | 503 no-schema → UNREACHABLE | 503 | `upstream-broken-no-json` | 2 / UNREACHABLE | **2** | `STATUS: UNREACHABLE  (/health returned HTTP 503 without schema_version)` |
| E | 504 + schema → UNREACHABLE | 504 | `{"schema_version":82}` | 2 / UNREACHABLE | **2** | `STATUS: UNREACHABLE  (/health returned HTTP 504)` |

All 5 scenarios match expected behavior.

Live cross-check on real gateway (`http://127.0.0.1:7400/health` returned `HTTP 200 schema_version:83 uptime_seconds:6718`): `bash scripts/check-gateway-freshness.sh` returned `STATUS: FRESH (drift <= 2)` — identical output to scenario A, proving the FRESH path on the production host is unchanged.

## Anti-theater check

Three independent reasons this isn't "test rewrites itself green":

1. **Production wire vs test seam are the same code.** The `FAKE_HEALTH_*` override at line 79-81 substitutes ONLY the curl response — the downstream `if`/`exit` logic (lines 91-128) is byte-identical between live and faked paths. Driving the test seam exercises the production branch.
2. **Asymmetric coverage proves the literal-string match.** Scenario D (503-no-schema → UNREACHABLE) and Scenario E (504-with-schema → UNREACHABLE) both refuse the new STALE-503 path despite carrying one of the two "ingredients" (503 status OR schema_version) the new branch consumes. The new branch fires ONLY when BOTH are present. This rules out a too-broad regex bug.
3. **Live cross-check.** Production gateway returns FRESH per scenario A, confirming the FRESH path on the actual `:7400` socket is unchanged.

## Gap / follow-up

None. The γ-carry that ran rounds 66→72 was rooted in the script failing to distinguish a degraded-but-reachable gateway from an unreachable one. PR #2335 closes that — for any future evaluator round, if the script reports `STATUS: STALE-503` it is now a true positive (call `restart-live-gateway.sh`); if it reports `STATUS: UNREACHABLE` it is also a true positive (different action: spin up the gateway). The conductor `scripts/auto-restart-gateway-on-schema-bump.sh` (#2336, verified in sibling report) closes the OTHER half of the loop — automatic restart on schema bump merges. Combined, the schema-drift false-alarm pattern is fully closed.
