# uf-spa-teammate-output-surface

**Owner**: claude or codex — product surface; conductor review before merge.
**Depends on**: cu1f (#2820) + cu1g (#2823) merged (work emission + tenant-aware approvals).
**Authored against**: origin/main `5e73d548b` on `2026-06-11`.
**Validate before claim**: bash scripts/check-brief-freshness.sh uf-spa-teammate-output-surface

## Problem (found live in the 2026-06-11 E2E stranger walkthrough)

A teammate's actual work product is invisible in the SPA. When a teammate runs,
its output lands in an on-disk artifact bundle
(`.skycot/outputs/<job_id>/{assistant_output.md,execution_plan.md,actions.json,
result.json,transcript.md}`) on the gateway host, plus human-handoff bundles
and files under the worker's `fs_root`. The only thing a SPA user ever sees of
the work is an `/audit` row (a `chars` count + `outcome_status`) and the
approvals-card text. There is **no "here is what your teammate produced"
surface**. The operator explicitly named this as the highest-leverage remaining
addition to make a minted teammate turnkey for a stranger. Evidence:
`docs/skycot/audit/2026-06-11-e2e-stranger-walkthrough.md` (pipeline-trace,
Stage 5 = WALL).

## Goal

A SPA surface that shows, per teammate run / per held action, the teammate's
real output: the assistant's reasoning/plan (`assistant_output.md`), the
proposed actions (`actions.json`), and — when present — the human-handoff
instructions and any files written under `fs_root`. Reachable from the worker
detail page ("Output" / "Recent runs" tab) and from the approval card ("view
what they produced").

## Scope / approach (smallest honest slice first)

1. **Gateway read route** (new, tenant-scoped, owner/User RBAC like the other
   worker routes): serve a run's artifact bundle by `job_id` — e.g.
   `GET /api/v1/workers/<id>/runs/<job_id>/output` returning the parsed
   `result.json` + `assistant_output.md` text + `actions.json` summary. MUST be
   tenant-scoped (a caller can only read its own tenant's bundles — mirror the
   `routes_worker_run` tenant resolution; reject cross-tenant with 404). Read
   only from the bundle dir; never execute. New file under
   `crates/argo-cli/src/handlers/gateway/routes_worker_outputs*.rs`.
2. **SPA**: an "Output" tab on the worker overview listing recent runs
   (the run-now response already returns `session_id`/`job_id`; the audit feed
   has the run rows) and rendering the selected run's assistant text + proposed
   actions. A "View output" affordance on the approval card. Use the locked
   design tokens; honest empty state ("no runs yet — use Run now").

## Out of scope (follow-ups)

- Rendering computer-use screenshots from the bundle (separate surface).
- Diffing fs_root file changes (later).

## Acceptance

- A run's `assistant_output.md` + proposed actions are visible in the SPA
  without touching the disk, tenant-scoped (cross-tenant read → 404), behind
  the same RBAC as worker detail.
- Tests: gateway route returns the bundle for the owning tenant + 404
  cross-tenant; SPA renders the output + empty state (vitest).
- Files < 500 LOC; no mock data on the real-session path (hasRealSession gate).

## Parallel-surface check

`rg 'routes_worker_outputs|worker.*output' crates/` — none today; the artifact
bundle writer is `work_artifacts` + `work_emission`. No parallel route exists.
