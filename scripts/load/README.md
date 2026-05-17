# Gateway load + soak baseline

`scripts/load/gateway-soak.sh` drives the running argo gateway with one
of three concurrent-request profiles (or a multi-minute soak) and writes
a single JSON results blob. It establishes whole-system numbers
(p50/p95/p99 latency, RSS, CPU, DB-file growth, audit-row growth) that
the crate-level criterion benches in `mission-criterion-load-tests`
cannot see.

Brief: `docs/build-plan/streams/codex-gateway-load-soak-baseline.md`.

## Prerequisites

* `oha` — the load driver. Install one of:
  * `brew install oha`
  * `cargo install oha`
* `python3` — JSON aggregation (any 3.8+).
* `sqlite3` — optional, used to count audit rows. The harness silently
  reports zero deltas if it's missing.
* `argo` binary on `PATH` — only required when running against a
  locally spawned gateway. `--target-url` skips the local spawn.

## Profiles

| Profile     | Shape                                              | Default duration |
|-------------|----------------------------------------------------|------------------|
| `burst`     | 100 RPS → `/health`                                | 60 s             |
| `sustained` | 50 RPS mix of `/health` + `/routes` (25 each)      | 5 min            |
| `spike`     | 0→500 RPS ramp in 10 s (5 × 2 s buckets), 30 s hold | 40 s             |
| `--soak N`  | Sustained profile, N minutes (overrides profile)   | operator         |

Only public-GET allowlisted endpoints are driven (`/health`, `/routes`,
`/version`) so the harness works against any gateway, including one
configured with `ARGO_GATEWAY_BEARER_TOKEN`. Write-path load testing
(signup, dispatch) is out of scope for this baseline — it needs the
cleanup harness from `#1810`.

## Running

```bash
# Local: brings up its own gateway in a temp ARGO_HOME.
bash scripts/load/gateway-soak.sh --profile burst
bash scripts/load/gateway-soak.sh --profile sustained
bash scripts/load/gateway-soak.sh --profile spike

# Soak: sustained for 30 minutes.
bash scripts/load/gateway-soak.sh --soak 30

# Remote: drive an already-running gateway.
bash scripts/load/gateway-soak.sh \
  --profile sustained \
  --target-url http://staging.skycot.internal:7400
```

Results land at `${TMPDIR:-/tmp}/skycot-load-soak-<ts>-<profile>.json`
(or under `--report-dir <path>`).

## Result shape

```jsonc
{
  "profile": "burst",
  "soak_min": 0,
  "start": "2026-05-17T11:20:00Z",
  "end":   "2026-05-17T11:21:00Z",
  "duration_s": 60,
  "base_url": "http://127.0.0.1:7400",
  "p50_ms": 1.4, "p95_ms": 6.8, "p99_ms": 11.2,
  "requests_total": 6000, "requests_successful": 6000,
  "achieved_rps": 99.8,
  "rss_kb_start": 24576, "rss_kb_end": 25104, "rss_kb_peak": 25216,
  "rss_growth_pct": 2.15,
  "cpu_pct_avg": 18.6, "ps_samples": 60,
  "db_bytes_start": 49152, "db_bytes_end": 49152,
  "db_bytes_delta": 0,
  "audit_rows_start": 0, "audit_rows_end": 0,
  "audit_rows_delta": 0,
  "tags": ["burst_health"],
  "thresholds": { /* config echoed for repro */ },
  "verdict": "ok",
  "verdict_notes": ["all thresholds OK"]
}
```

## Interpreting output

The harness emits a single `verdict` field with three bands. Tune the
edges in the `## config / thresholds` block at the top of
`gateway-soak.sh`.

| Metric                | OK            | WARN            | FAIL              |
|-----------------------|---------------|-----------------|-------------------|
| `p95_ms`              | < 500 ms      | 500–2000 ms     | > 2000 ms         |
| `rss_growth_pct` (5m) | < 10 %        | 10–50 %         | > 50 % (memleak)  |
| DB bytes / audit row  | < 2 KB        | > 2 KB          | —                 |

Exit code: `0` on OK or WARN, `1` on FAIL. The JSON carries the
verdict + per-rule notes so dashboards can split WARN out separately.

### When to escalate

* `verdict=fail` on a burst run → drop into `argo gateway audit list`
  for the 5xx envelope, then open a P1 with the JSON attached.
* `verdict=warn` for RSS growth on a sustained run → re-run as a
  `--soak 30`. If the slope persists, that's a memory leak — open a
  P1 with both report files.
* p95 in the WARN band on burst but OK on sustained → likely cold
  caches; re-run after a 60-second warmup `--profile burst`.

## Notes on accuracy

* **`oha` latency percentiles** are per-invocation. The sustained and
  spike profiles fire multiple `oha` calls; the aggregator surfaces
  the **worst** percentile across runs, which is the safe answer for
  "did we breach p95?" but slightly pessimistic versus a true
  cross-run sample. Treat the numbers as floor-of-headroom, not gospel.
* **`ps -o %cpu=`** is lifetime-average on macOS, instantaneous on
  Linux. The 1 Hz sampler smooths both into a useful trend value, but
  cross-OS comparison should be done with intent.
* **Spike ramp** is approximated as 5 × 2 s buckets (`oha` cannot
  ramp inside one run). The hold phase is a true sustained burst.
