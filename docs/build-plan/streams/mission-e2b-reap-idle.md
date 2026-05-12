# Mission load-bearing wire — periodic E2B idle-sandbox reaper

**Owner**: claude. **Depends on**: existing `crates/argo-executor-computer-use/src/e2b.rs` (E2bClient + E2bCall trait).
**Authored against**: origin/main `84e47150` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-e2b-reap-idle`.

> SOURCE: MISSION_BACKLOG.md Section 8 Claim 8 #243 ("E2B sandbox cost optimization (`reap_idle` caller)"). **Tier P (perf-ops).** Today: when a computer-use session crashes mid-run, the sandbox is leaked — E2B keeps billing for an instance that no operator is using. Sandboxes auto-expire on the E2B side after their template's `pause_timeout` (typically 5-15 min), but during that window operators are charged. This brief adds a periodic reaper that explicitly closes sandboxes idle longer than a configured threshold, plus a one-shot CLI for operator-driven cleanup.

## Scope

EXTEND `crates/argo-executor-computer-use/src/e2b.rs` (≤ 80 LOC delta — surgical, not a rewrite):

- Add `pub struct E2bSandboxInfo { id: String, template: String, started_at: chrono::DateTime<chrono::Utc>, last_activity_at: chrono::DateTime<chrono::Utc>, status: String }` — what E2B's `/sandboxes` list endpoint returns per sandbox.
- Add `async fn list_sandboxes(&self) -> Result<Vec<E2bSandboxInfo>, CuError>` to the `E2bCall` trait + the `E2bClient` impl. Endpoint: `GET /sandboxes` (returns all sandboxes the API key owns).
- Add `async fn reap_idle(&self, max_idle_secs: u64) -> Result<E2bReapReport, CuError>` to the `E2bCall` trait + the `E2bClient` impl. Calls `list_sandboxes`, filters those where `now - last_activity_at > max_idle_secs`, calls `close_sandbox` on each, returns an `E2bReapReport { scanned: usize, reaped: Vec<String>, errors: Vec<(String, String)> }`.
- `MockE2bClient` in `testing.rs` gains the same trait methods (scripted return values).

NEW `crates/argo-executor-computer-use/src/e2b_reap_tests.rs` (NEW, ≤ 130 LOC, included via `#[path]`) — 6 unit tests using `wiremock` for HTTP-level behavior + `MockE2bClient` for trait-level: list-sandboxes-2-returned, reap-idle-with-no-stale-returns-empty-report, reap-idle-with-3-stale-closes-all-3, reap-idle-with-1-close-error-continues-and-reports, reap-idle-honors-max-idle-threshold (boundary: exactly-at-threshold is NOT reaped), reap-idle-handles-pagination-if-100+-sandboxes.

### CLI

EXTEND `crates/argo-cli/src/cli_command_enum.rs` (≤ 15 LOC delta) — new top-level `SandboxCmd::E2b` subgroup with two variants:
```
argo sandbox e2b list                           # JSON list of all owned sandboxes
argo sandbox e2b reap-idle --max-age <secs>     # one-shot reaper; default 600s
argo sandbox e2b reap-idle --max-age 600 --dry-run    # list what would be reaped without closing
```

NEW `crates/argo-cli/src/handlers/sandbox_e2b.rs` (NEW, ≤ 130 LOC) — thin handler that constructs an `E2bClient` from env (`ARGO_E2B_API_KEY`), calls `list_sandboxes` or `reap_idle`, prints JSON-line output (matches existing `argo` CLI conventions: list/show emit JSON).

### Cron integration

EXTEND `crates/argo-runtime/src/cron_tick.rs` (≤ 25 LOC delta — verify file exists at claim time via `rg -l 'cron_tick' crates/argo-runtime/src/`; if shaped differently, fall back to a NEW maintenance-job file under argo-runtime/src/ following the existing in-flight `mission-cron-tick-maintenance-jobs` brief shape, but DON'T conflict with that brief — coordinate at claim time). Add `e2b_reap_idle` as a registered cron job with default interval 15 min + default `max_idle_secs = 600`. Gated by `ARGO_E2B_API_KEY` env presence — no-op when not configured.

NO migration. NO change to existing CU executor session lifecycle (the reaper is orthogonal: it cleans up leaked sandboxes; healthy sessions close their own sandboxes via the existing `Drop` / explicit-close path).

This is NOT in scope: E2B sandbox quota tracking / budget enforcement (separate brief — quotas are tenant-level, reaping is per-API-key cleanup); template-aware reaping (one threshold for all templates; per-template thresholds is v2 if operators ask); reaping based on cost rather than time (v2; cost requires E2B billing-API integration which is a different surface); auto-reaping during normal CU execution (the existing CU `Drop` path stays the in-band cleanup; this is the out-of-band leak-recovery path); cross-cloud sandbox reaping (E2B only — Docker / local-sandbox cleanup is its own surface).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-executor-computer-use/src/e2b.rs` (EXTEND, ≤ 80 LOC delta)
- `crates/argo-executor-computer-use/src/e2b_reap_tests.rs` (NEW, ≤ 130 LOC)
- `crates/argo-executor-computer-use/src/testing.rs` (EXTEND, ≤ 30 LOC — add `list_sandboxes` + `reap_idle` to MockE2bClient)
- `crates/argo-executor-computer-use/src/lib.rs` (EXTEND, ≤ 5 LOC — re-export new types)
- `crates/argo-cli/src/cli_command_enum.rs` (EXTEND, ≤ 15 LOC)
- `crates/argo-cli/src/handlers/sandbox_e2b.rs` (NEW, ≤ 130 LOC)
- `crates/argo-cli/src/handlers/mod.rs` (EXTEND, ≤ 3 LOC)
- `crates/argo-cli/tests/sandbox_e2b_smoke.rs` (NEW, ≤ 100 LOC — env-gated end-to-end)
- `crates/argo-runtime/src/cron_tick.rs` (EXTEND, ≤ 25 LOC — registration; verify file at claim time)
- `docs/skycot/runbook/e2b-reap-idle.md` (NEW, ≤ 120 LOC) — operator runbook

## Files you must NOT touch

- `crates/argo-executor-computer-use/src/executor.rs` (orthogonal — main execution path unchanged)
- `crates/argo-executor-computer-use/src/pool.rs` (orthogonal — pool keeps its own lifecycle)
- `crates/argo-sandbox/**` (different sandbox abstraction; this brief is E2B-specific)
- `crates/argo-store/migrations/**` (no migration)

## Acceptance criteria

- [ ] `cargo build -p argo-executor-computer-use -p argo-cli` succeeds
- [ ] `cargo test -p argo-executor-computer-use e2b_reap` passes all 6 unit tests
- [ ] `argo sandbox e2b list` returns JSON-line array of sandboxes
- [ ] `argo sandbox e2b reap-idle --max-age 600 --dry-run` lists candidates without closing them
- [ ] `argo sandbox e2b reap-idle --max-age 600` closes all sandboxes idle > 600s and emits a report
- [ ] Reaping continues past individual close-errors and reports them in the `errors` field
- [ ] Cron-registered version no-ops when `ARGO_E2B_API_KEY` is unset (no errors, no logs)
- [ ] Smoke test (env-gated) creates a sandbox via existing E2B client, waits 1s, runs reap with `--max-age 0` → sandbox closed
- [ ] All new files < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-e2b-reap-idle`
- PR title: `argo-executor-computer-use: periodic E2B idle-sandbox reaper + sandbox e2b list/reap-idle CLI + cron job (mission-e2b-reap-idle)`

## Notes for the implementer

- E2B's `GET /sandboxes` endpoint returns all sandboxes the API key has touched in roughly the last 24h. The exact `last_activity_at` semantics may have edge cases — check E2B's API docs at claim time. If the field is missing or null, treat the sandbox as "no activity ever recorded" and use `started_at` as the fallback.
- Pagination: E2B's list endpoint paginates at 100 sandboxes per page. Implement the cursor-follow loop in `list_sandboxes`. A long-running operator with many concurrent sandboxes is the expected case.
- Boundary semantics: the `--max-age` is **idle time**, not total age. A sandbox that's been alive 2 hours but had activity 30 seconds ago is NOT idle. A sandbox alive 5 min with no activity since creation IS idle (passes the threshold once `started_at` is the only fallback timestamp).
- DO NOT close sandboxes from other workspaces / tenants accidentally. The E2B API-key is the tenancy boundary — operators with multiple Argo workspaces sharing one E2B key WILL reap each other's sandboxes. Document this loudly in the rustdoc + runbook: "the reaper closes ALL idle sandboxes under this API key, including those from other workspaces or applications."
- DO NOT introduce a `--workspace-filter` flag in v1. E2B doesn't tag sandboxes with workspace identity; filtering would require reading every sandbox's metadata which is a different API surface. v2 if operators ask.
- The cron integration must be gated by `ARGO_E2B_API_KEY` presence — operators without E2B configured shouldn't get spurious "E2B reap skipped" logs every 15 min. Use `tracing::trace!` for the no-op skip path.
- DO NOT add `reap_idle` to the existing `E2bCall` trait if doing so breaks downstream impls — the parallel `MockE2bClient` in `testing.rs` MUST stay in sync. Add the method with a default impl that returns a no-op `E2bReapReport` so existing test mocks compile unchanged; production `E2bClient` overrides.
- The CLI's `list` output must be JSON-line (one sandbox per line) to match existing CLI conventions; the `reap-idle` output is a single JSON object summarizing the run.
- The runbook should cover: when to run manually (post-crash recovery; operator forgot to close a long session); what `--max-age` is reasonable (default 600s = 10 min; aggressive 60s for cost-sensitive operators); how to confirm a sandbox was closed via E2B dashboard.
- Coordinate w/ `mission-cron-tick-maintenance-jobs` (in flight): that brief sets up the cron scaffolding; this brief registers one job into it. If the cron-tick brief hasn't landed yet, prefer a NEW file `crates/argo-runtime/src/e2b_reap_cron.rs` (≤ 60 LOC) standalone — it can be folded into the cron scaffolding later.
- DO NOT implement a "reap all sandboxes regardless of age" path. That's footgun-class. Operators who want full cleanup can call `argo sandbox e2b list --json | jq -r '.[].id' | xargs -I{} curl -X DELETE ...` outside Argo.
