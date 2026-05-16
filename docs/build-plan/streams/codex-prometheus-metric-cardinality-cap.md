# codex-prometheus-metric-cardinality-cap — cardinality cap for /metrics labels (P1)

**Owner**: codex. **Sizing hint**: GPT-5.5 Pro extra-high reasoning suits this — production-hardening judgment call. **Depends on**: `codex-observability-prometheus-grafana` MERGED (#1775).
**Authored against**: origin/main `bf84d768` on `2026-05-17`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-prometheus-metric-cardinality-cap`.
**Source**: PR #1775 follow-up — Grafana-Cloud / Datadog bill on cardinality. Without a cap, a buggy code path emitting `tenant_id` as a label can cardinality-bomb the operator's metrics bill.

## Mission alignment

- **Observability** (CLAUDE.md priority #2) — metrics are infra; protecting against own-goal cardinality explosions is part of the platform contract.
- **Commercial usability** (#5) — operator-account budgets matter; a runaway cardinality on day 1 is a $10k surprise bill.

## Scope

P1. Add cardinality protection to `argo-telemetry`'s Prometheus exporter (added by #1775):

1. **Per-metric series cap.** Each metric family has a max distinct label-set count. Configurable per metric, defaults to:
   - `histogram` / `summary`: 1000 series
   - `counter` / `gauge`: 5000 series
   - Operator overrides via `[telemetry.prometheus.cardinality_cap]` config block.

2. **Soft + hard overflow behavior.**
   - At 80% of cap → emit a `WARN` log: "metric `argo_request_duration_seconds` approaching cardinality cap (804/1000)".
   - At 100% → drop new label sets, emit a counter `argo_metric_cardinality_capped_total{metric="..."}` for visibility, and emit one `ERROR` log per metric per 5-min window (rate-limited).
   - Never panic, never block. Fail-open with visibility.

3. **Cardinality estimator.** Use HyperLogLog (`hyperloglog` crate) for the count — bounded memory regardless of cap size; ~5% error tolerance is fine.

4. **`/metrics/cardinality` admin endpoint.** Returns JSON `{ family: { current_count, cap, capped_total } }` for ops triage. Admin-RBAC-gated (`Capability::AuditWrite` or new `Capability::OperatorReadOnly`).

LOC: ~250.

## Files you may touch

- `crates/argo-telemetry/src/prometheus.rs` (existing — added by #1775) — wire the cardinality check into the metric-record hot path. ≤80 modified LOC.
- `crates/argo-telemetry/src/cardinality.rs` (NEW) — `CardinalityTracker` struct + HLL impl + per-family caps. ≤120 LOC.
- `crates/argo-telemetry/src/cardinality_tests.rs` (NEW) — under cap / at warn threshold / at hard cap / capped-total emission. ≤80 LOC.
- `crates/argo-telemetry/Cargo.toml` — narrowly: add `hyperloglog = "1"`. ≤3 modified LOC.
- `crates/argo-config/src/sections_telemetry.rs` (existing or NEW) — `[telemetry.prometheus.cardinality_cap]` config schema. ≤40 modified LOC.
- `crates/argo-cli/src/handlers/gateway/routes_metrics_cardinality.rs` (NEW) — admin endpoint. ≤50 LOC.
- `crates/argo-cli/src/handlers/gateway/boot.rs` — mount route. ≤5 modified LOC.

## Files you must NOT touch

- Existing /metrics route or scrape format — purely add the cap, don't restructure the surface.
- The 4 Grafana dashboards shipped in #1775 — they'll handle the new `argo_metric_cardinality_capped_total` family automatically; no dashboard JSON edits in scope.
- Anything outside argo-telemetry + argo-config + 2 cli files.

## Acceptance criteria

- [ ] Recording a 1001st distinct label set on a histogram-family metric → dropped, `argo_metric_cardinality_capped_total{metric="x"}` incremented.
- [ ] At 80% threshold → WARN log emitted exactly once per metric per 5-min window.
- [ ] At 100% threshold → ERROR log emitted exactly once per metric per 5-min window.
- [ ] `GET /metrics/cardinality` returns valid JSON per spec; admin-RBAC-gated.
- [ ] Default caps applied if no `[telemetry.prometheus.cardinality_cap]` config block.
- [ ] HLL estimator memory is bounded (test: 100k inserts uses <100KB).
- [ ] `cargo test -p argo-telemetry -p argo-config` passes.
- [ ] `cargo clippy -p argo-telemetry -p argo-config -p argo-cli --all-targets -- -D warnings` clean.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `codex-prometheus-metric-cardinality-cap`
- Title: `feat(telemetry): cardinality cap + capped-total counter + /metrics/cardinality endpoint (codex-prometheus-metric-cardinality-cap)`
- Worker verification: crate-scoped + workspace clippy per SWARM_PROTOCOL.md.

## Hidden constraints

- **HLL doesn't track exact members.** If you need exact counts for ops triage, fall back to a bounded HashSet capped at 2× the family's cap and accept the memory cost. Pick HLL by default, expose `--exact-cardinality` env for ops.
- **Drop, don't reject.** A buggy code path emitting a new label set every request will hit hard cap and stop emitting new series — but existing series still record. Don't drop the whole metric.
- **Rate-limit logs.** 5-min window per metric prevents log flooding if the offending code path is hot.
- **`argo_metric_cardinality_capped_total` is itself a counter** with a single label (`metric`) — make sure THIS counter doesn't have its own cardinality issue (cap at metric-family count, not infinite).
- **HLL error tolerance** — 5% means a cap of 1000 might fire between 950 and 1050 actual series. Document this in the config block's rustdoc.
- **No emit-side caching of label hash.** The HLL insert is the cache.
