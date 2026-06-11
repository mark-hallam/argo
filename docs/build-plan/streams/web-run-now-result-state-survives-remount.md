# web-run-now-result-state-survives-remount

**Owner**: claude or codex — web-only, DEC-006 fast lane (vitest green required).
**Depends on**: none (#2830 + day-1 activation package both MERGED).
**Authored against**: origin/main `7f5f86c7c` on `2026-06-12`.
**Validate before claim**: bash scripts/check-brief-freshness.sh web-run-now-result-state-survives-remount

## Problem (observed live in the 2026-06-12 DEC-044 browser pass)

On worker overview, clicking **Run now** completes a REAL run (engine: turn
ran, `probation.hold` audit row written, toast renders "Read ran, but their
tools are held by the probation window — Approve the held tools below") — but
the page remounts after the run and the run's response state (`job_id`,
`held_tools`, `probation`) is held only in component memory, so after the
remount:

- the **Output card** (#2830 `RunOutputCard`) shows "No runs yet. Use Run
  now…" immediately after a successful run (the route itself is VERIFIED
  fine — fetching `GET /api/v1/workers/<id>/runs/<job_id>/output` with the
  session cookie returns the parsed bundle for that exact run);
- the **held-tools approval card** (`ProbationApprovalCard`) never renders,
  even though the toast tells the operator to "approve the held tools
  below" — the operator's only path is to run AGAIN and not lose the page.

The two flagship day-1 surfaces are unreachable exactly when the operator
was just told to use them.

## Fix (root: persist the last-run summary, don't pin component state)

Persist the Run-now response summary (`job_id`, `held_tools`, `probation`,
timestamp) per worker — `sessionStorage` keyed by worker id is sufficient
(no backend change), or hydrate from an existing backend list endpoint if
one already returns recent runs (grep first; do NOT add a new backend route
in this brief). On mount, `RunOutputCard` + the probation-approval host
rehydrate from that summary so both cards show the latest run after any
remount/reload. Honest empty state stays for workers never run.

## May touch

- `web/components/skycot/worker/` (RunOutputCard host, ProbationApprovalCard
  host, WorkerOverviewContent)
- `web/lib/api/workers.ts` (run-now response handling) + a small storage
  helper
- vitest files for the above

## Acceptance / verifier

1. vitest: after a mocked run-now response, simulated remount still renders
   the output card (with job link) + the approval card (held tools listed).
2. vitest: a worker with no stored summary keeps today's empty states.
3. `pnpm --dir web test` green; `tsc` clean; every file <500 LOC.

~60–120 LOC. **🟠 P2 — day-1 activation UX integrity (the cards exist and
work; they're just unreachable after the remount).**
