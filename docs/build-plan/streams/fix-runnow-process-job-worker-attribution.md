# fix-runnow-process-job-worker-attribution

**Owner**: claude.
**Authored against**: origin/main `f1ea8b09a` on `2026-06-09`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-runnow-process-job-worker-attribution`.

## Why (e2e finding R2-F1, VERIFIED live 2026-06-09)
The product's core promise is "every teammate action is auditable to that teammate." But LIVE work executed through `process_job` is NOT worker-attributed in the audit feed. Repro: `POST /api/v1/workers/<Maya-id>/run` → 200 completed (1734 chars real output); the emitted turn rows (`turn_start`/`assistant_message`/`tool_call`/`tool_result`/`turn_end`) appear in `GET /api/v1/audit` with `worker={"id":"","name":"unknown"}`. **Maya-attributed rows: 0.** Seeded demo workers (Mariah/Jamal/etc.) ARE attributed incl. turn-level rows, so turn rows CAN carry a worker — only the live runtime path doesn't stamp it. A paying customer running a teammate (Run-now / scheduled) sees their work logged as "unknown".

## Grep-before-write
- `rg -n 'resolve_page_worker_names|lookup_worker_name|fn audit_row_json' crates/argo-cli/src/handlers/gateway/routes_audit_feed*.rs` — the feed resolves `.worker` from a worker id ON the row.
- `rg -n 'process_job|worker_id|session|AuditSink|emit|worker' crates/argo-cli/src/handlers/work_dispatch.rs crates/argo-cli/src/handlers/gateway/routes_worker_run_exec.rs` — does `process_job` carry the worker id into the per-turn audit context? Where is the run session created (`run_worker_now`) and is the worker id threaded to the turn audit rows?
- Check whether the audit row has a `worker_id` column the runner populates, OR whether the feed resolves worker via a session→worker join (and whether the live run creates that session→worker link). The fix differs:
  - If turn rows take `worker_id` from the audit context → thread the worker id from `run_worker_now` into `process_job`'s audit sink/session so each turn row is stamped.
  - If the feed joins session→worker → ensure the run session row records the worker id the feed can join on.

## Fix (first-principles)
Make a hired worker's LIVE work attribute to that worker in the audit feed — the same way seeded rows do. Stamp the `worker_id` onto the turn audit rows emitted under a worker-run session (preferred: thread it through `process_job`'s audit context from `run_worker_now`, which already has the worker). Do NOT weaken tenant scoping. Decide + document whether `pitch.*` rows (Portia, a tenant-level agent) should attribute to the Portia worker or stay tenant-level (separate call; not necessarily part of this fix).

## Acceptance (truth-bounded)
1. New test: a worker-run turn (driven by the `ARGO_WORKER_RUN_PROVIDER=mock` seam used by `routes_worker_run_exec_tests.rs`) emits audit rows whose resolved worker == the run's worker (NOT "unknown").
2. Live (gateway :7400): `POST /api/v1/workers/<id>/run` then `GET /api/v1/audit` shows the new turn rows attributed to that worker id/name (paste the rows).
3. `cargo test -p argo-cli` green; clippy clean on touched files (boundary-note the pre-existing routes_settings_credentials await_holding_lock).

## On ship (same commit)
Update FEATURE_MATRIX audit row + regen HTML. Note it closes e2e finding R2-F1.

## Effort
M (touches the work_dispatch/process_job audit context or the feed's worker resolution + a round-trip test).
