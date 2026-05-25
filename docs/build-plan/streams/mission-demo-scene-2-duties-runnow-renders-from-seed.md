# mission-demo-scene-2-duties-runnow-renders-from-seed

**Owner**: claude.
**Authored against**: origin/main `e8f7e67e6` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-demo-scene-2-duties-runnow-renders-from-seed`.
**Depends on**: `mission-demo-seed-data-bootstrap` MERGED (delivers seeded workers + duty rows) + `mission-demo-seed-real-llm-driver` MERGED (delivers the audit-row history for Run-now drill-ins).

🟡 **P1 DEMO-GATE — Scene 2 rendering verification (FUNCTION-SIDE per `feedback_no_theater_in_demo` clarified 2026-05-25T08:10Z).**

`v1-mission-mvp-runbook.md` Scene 2 demonstrates duties (the operator-facing word for what code calls "schedules" per memory `project_vocabulary_duty_vs_schedule`) + Run-now. Operator clicks a worker's "Schedules" tab, sees the cron-shaped duties Mariah has (morning_kickoff cron at 9, listing_health_check every 6h, weekly_summary Fridays 17:00), clicks "Run now" on one, watches a real agent turn fire + audit row land. Per function-vs-dressing: the duty execution must be REAL (real `AgentRunner::run_turn` call → real `WorkerAction` row → real `audit_record`); duty cron-schedule labels can be theatrical strings.

## Scope

1. **Pre-flight**: confirm `mission-demo-seed-data-bootstrap` + `mission-demo-seed-real-llm-driver` both MERGED. If not, do NOT claim — wait.

2. **Run the seed**: `argo demo seed --tenant <demo-tenant-id>`. Verify:
   - `curl http://localhost:7400/api/v1/workers/<mariah-id>/duties` (or whatever the gateway endpoint is — check `routes_worker_duties.rs` or similar) returns ≥3 duty rows.
   - Each duty has a `name`, `cron_schedule`, `enabled` field.

3. **Render check** — open `/workers/{mariah-id}` in a headless browser (or curl + HTML grep):
   - Worker-detail page renders C2-personality design per memory `project_worker_detail_design_locked`.
   - "Schedules" tab is present (NOT "Duties" — per memory `project_vocabulary_duty_vs_schedule`, operator-facing copy uses "Schedule" not "Duty"; code/SQL/audit keeps "duty").
   - Schedules tab content shows the 3 duty rows with cron-shaped labels.
   - Each duty has a "Run now" button.

4. **Run-now drill-in test** — click "Run now" on one duty (e.g. listing_health_check):
   - Backend fires `AgentRunner::run_turn` with the duty's payload.
   - Audit row appears in `/audit` with kind `worker_duty.fired` (NOT theatrical — real agent turn execution).
   - The duty's payload includes real worker_id + real tenant_id + a sensible action description.
   - Latency check: `Run now` → audit-row-visible in `/audit` ≤ 5s p95.

5. **Cross-tenant check** — log in as foreign tenant; `/workers/<mariah-id>` returns 404 (not someone else's worker visible).

6. **Fix any rendering gaps** — if the Schedules tab doesn't render, check the gateway endpoint shape + the SPA's fetch flow.

## Acceptance criteria

- [ ] `curl http://localhost:7400/api/v1/workers/<mariah-id>/duties` returns ≥3 real duty rows.
- [ ] `/workers/{mariah-id}` renders the Schedules tab with the 3 duty rows.
- [ ] "Run now" on a duty creates a real audit row (verified via `curl http://localhost:7400/api/v1/audit?kind=worker_duty.fired&worker_id=<mariah-id>` returning a new row).
- [ ] p95 latency Run-now-click → audit-row-visible ≤ 5s.
- [ ] Cross-tenant 404 verified.
- [ ] PR body has screenshots: Schedules tab + post-Run-now audit row.

## May-touch

- `web/app/workers/[id]/page.tsx` (if Schedules tab rendering needs gaps fixed)
- `web/components/skycot/worker-detail/schedules-tab.tsx` (if it exists; create if not)
- `web/lib/api/duties.ts` or wherever the fetch flow lives (envelope mismatches)
- This brief file (status updates)

## Out of scope

- Re-implementing the cron-scheduler runtime (consume existing surface)
- Adding new duty types (consume what the seed produces)
- Renaming "Duty" → "Schedule" in code (per memory: vocabulary stays internal; UI-only rename)

## Effort

~30-60 min if rendering correct; +30-90 min per UI gap.
