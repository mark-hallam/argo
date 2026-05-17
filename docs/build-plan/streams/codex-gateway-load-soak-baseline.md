# codex-gateway-load-soak-baseline — whole-system load + soak baseline (P1 operational-audit blind-spot)

**Owner**: codex. **Sizing hint**: GPT-5.5 Pro extra-high — judgment on shape of harness.
**Depends on**: none.
**Authored against**: origin/main `37f268a6` on `2026-05-17`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-gateway-load-soak-baseline`.
**Source**: 2026-05-17 audit blind-spot — the audit was static-analysis-focused. Whole-system behavior at scale (RAM/CPU/p95 latency/WS limits/audit-chain growth) is unverified. Static crate-level benches exist (#1365 mission-criterion-load-tests, MERGED 2026-05-12) but no end-to-end gateway load baseline.

## Mission alignment

- **Reliability** (CLAUDE.md priority #1) — "what happens at 100/500/1000 concurrent signups" is unknown today.
- **Observability** (#2) — establishes baseline numbers for operator dashboards (Prometheus from #1775 is shipped; this brief provides what-to-expect-at-scale).

## Scope

P1. New gateway-level load harness in `scripts/load/gateway-soak.sh` that:

1. Spawns the gateway (release binary + a temp `~/.argo` dir).
2. Drives 3 concurrent-request profiles via `oha` or `hey` (whichever's already in workspace; if not, `oha` is the standard rust-friendly choice):
   - **Burst:** 100 RPS to `/healthz` for 60s
   - **Sustained:** 50 RPS mix of `/healthz` + `/meta/tools` for 5min
   - **Spike:** 0 → 500 RPS in 10s → hold 30s → 0
3. Captures per-profile metrics:
   - p50/p95/p99 latency per endpoint
   - Gateway RSS (parse from `ps -o rss=`)
   - Gateway CPU% (parse from `ps -o %cpu=`)
   - DB file size delta (`du -sb ~/.argo/db.sqlite`)
   - Audit row count delta
   - WebSocket connection count if applicable (skip if WS not under load — that's a follow-up)
4. Emits a JSON results file: `/tmp/skycot-load-soak-$(date).json`.
5. Pass/warn/fail thresholds (configurable in script header):
   - p95 < 500ms = OK, 500-2000ms = WARN, >2000ms = FAIL
   - RSS growth over 5min sustained < 10% = OK, 10-50% = WARN, >50% = FAIL (memory leak)
   - DB growth proportional to audit count: < 2KB/row = OK

### Soak component

Optionally (`--soak <minutes>`): run the sustained profile for N minutes (default off; operator-invoked). Captures RSS growth, audit-chain query perf as audit table grows, GC behavior.

LOC: ~300 (bash + light Python for JSON aggregation).

## Files you may touch

- `scripts/load/gateway-soak.sh` (NEW) — main harness. ≤200 LOC bash.
- `scripts/load/aggregate-results.py` (NEW) — JSON aggregation from oha/hey output + ps + du. ≤80 LOC.
- `scripts/load/README.md` (NEW) — operator runbook + how to interpret output. ≤80 LOC.
- `.github/workflows/load-soak.yml.disabled` (NEW) — manual-trigger workflow (`workflow_dispatch`) for operator-triggered runs on a dedicated runner. ≤60 LOC. Disabled by default; operator enables before launch.

## Files you must NOT touch

- Gateway code — harness is pure-external (HTTP + ps + du).
- Existing crate-level benches (`mission-criterion-load-tests`) — orthogonal.
- Real CI workflows — load runs are operator-invoked, not per-PR.

## Acceptance criteria

- [ ] `bash scripts/load/gateway-soak.sh --profile burst` runs end-to-end against a local gateway.
- [ ] Output JSON has `{profile, duration_s, requests_total, p50_ms, p95_ms, p99_ms, rss_kb_start, rss_kb_end, cpu_pct_avg, db_bytes_delta, audit_rows_delta}`.
- [ ] All 3 profiles (burst / sustained / spike) supported.
- [ ] `--soak 30` runs the sustained profile for 30 minutes and reports RSS growth %.
- [ ] Operator runbook documents: how to install `oha`, how to run each profile, how to interpret thresholds, when to escalate.
- [ ] Disabled workflow stub exists for operator-triggered CI runs.
- [ ] `bash -n scripts/load/gateway-soak.sh` syntax-clean.
- [ ] `shellcheck scripts/load/gateway-soak.sh` clean (only pre-existing warnings if any).
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `codex-gateway-load-soak-baseline`
- Title: `feat(load): whole-system gateway load + soak harness (codex-gateway-load-soak-baseline)`
- Worker verification: bash + python syntax checks + dry-run against localhost per SWARM_PROTOCOL.md.

## Hidden constraints

- **No real production targets.** Run is local-only by default. If pointing at a deployed gateway, that's `--target-url` (mirror of #1779 deploy-smoke flag).
- **Throwaway data only.** Tests use `/healthz` + `/meta/tools` which are read-only / cheap. Don't load-test signup or dispatch (those write tenants/sessions; load-testing them needs the cleanup harness from #1810).
- **`oha` vs `hey` vs `wrk`** — `oha` is rust-friendly + outputs JSON. Use it if available; `hey` is the fallback. Install via brew/cargo.
- **WebSocket load testing** out of scope (separate brief).
- **CPU sampling needs care.** `ps -o %cpu=` is point-in-time; for `cpu_pct_avg` over the run, sample at 1Hz and average.
- **Closes audit blind-spot** for "what happens at scale" — provides a runnable baseline before launch.
