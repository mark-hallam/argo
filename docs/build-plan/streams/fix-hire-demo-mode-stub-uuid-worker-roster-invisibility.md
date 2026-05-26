# fix-hire-demo-mode-stub-uuid-worker-roster-invisibility

**Owner**: claude.
**Authored against**: origin/main `2cc2ec42b` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-hire-demo-mode-stub-uuid-worker-roster-invisibility`.
**Source**: ux-e2e-audit-T2309MEL `onboarding-hire-an-ai-teammate.md` F3 [P0].

🔴 **P0 DEMO-GATE — In the recording configuration (auth-bypass on), `hireWorker()` short-circuits to `stub-<crypto.randomUUID()>` regardless of input. The redirect lands on `/workers/stub-xxx/overview` where the browser tab title is literally `stub-<uuid> · Overview`, the personality card 404s, and `/team` keeps showing only the 5 SAMPLE_TEAM personas — the just-hired worker is invisible. The "meet your new teammate" payoff is theater the camera DRILLS INTO. Violates `feedback_no_theater_in_demo`.**

## Root cause

`web/lib/api/workers.ts:331`:

```ts
export async function hireWorker(req: HireWorkerRequest): Promise<HireWorkerResult> {
  if (isAuthBypassed()) {
    return { workerId: stubWorkerId(), success: true };
  }
  // ...real POST to /api/v1/workers
}
```

`stubWorkerId()` at line 219 returns `stub-<crypto.randomUUID()>`. With `.claude/launch.json:9` setting `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1` (the recording configuration), every Hire-button click in `/hire` returns a brand-new stub regardless of buyer input.

Cascading consequences:

1. **`/team` does not refresh.** `useTeam()` (`web/lib/hooks/use-team.ts`) refetches `fetchTeam()` which in demo mode returns the static `SAMPLE_TEAM` (Mariah / Peta / Priya / Jamal / Nori) from `web/lib/sample-team.ts`. No optimistic insert path. The just-hired worker never appears.

2. **Worker-detail page title is the UUID.** `web/app/(app)/workers/[workerId]/overview/layout.tsx:10` returns `{ title: \`${workerId} · Overview\` }` — for a stub ID, this renders as `stub-018f67d0-... · Overview` in the browser tab.

3. **Personality card 404s.** `fetchWorkerOverview('stub-xxx')` throws `WorkerOverviewNotFoundError` (`web/lib/api/worker-overview.ts:114`); the page renders an error state, not a charter.

Per `feedback_no_theater_in_demo` + `v1-demo-positioning-poc-vs-public.md`: function-side theater the camera SEES is forbidden. This is the headline case.

## Findings this brief closes

- onboarding-hire-an-ai-teammate F3 [P0] — Hire commit returns stub UUID; new worker invisible

## Fix recipe

Two viable approaches; ship one:

### Option A (preferred — session-local mock store)

When `isAuthBypassed()` is true, `hireWorker()` writes the new worker into a session-scoped local mock store that `fetchTeam()` reads:

1. Add `web/lib/api/demo-roster-store.ts`:
   - `appendDemoWorker(worker: WorkerOverview): void` — appends to a Map keyed by tenant
   - `listDemoWorkers(): WorkerOverview[]` — returns appended workers
   - Storage: `sessionStorage` (clears on tab close — fine for recording)

2. `hireWorker()` in auth-bypass branch:
   - Generate a proper UUIDv7 (NOT prefixed `stub-`)
   - Synthesize a `WorkerOverview` from `HireWorkerRequest` (name, role, cohort) and the operator's narrated traits
   - Call `appendDemoWorker(...)`
   - Return `{ workerId: <real-shaped-uuid>, success: true }`

3. `fetchTeam()` in auth-bypass branch:
   - Return `[...SAMPLE_TEAM, ...listDemoWorkers()]`

4. `fetchWorkerOverview(id)` in auth-bypass branch:
   - Check `listDemoWorkers()` first, fall back to `SAMPLE_WORKER_OVERVIEWS`, otherwise 404

5. `/workers/[workerId]/overview/layout.tsx` — call `generateMetadata` async, resolve to `worker.name ?? workerId` for the title.

### Option B (recording-only — operator narrates around it)

Document the recording technique in the runbook: don't run the wizard live during the take; instead drill into an existing SAMPLE_TEAM persona (e.g. open `/team` → click Mariah). The hire-button beat moves to a separate b-roll segment.

This is what the audit's mitigation note recommends as the short-term fix, but it leaves the function-side theater present on any unscripted demo (live walkthroughs to customers, etc.). Option A is the load-bearing fix.

## Acceptance criteria

- [ ] Filling `/hire` step 1-4 + clicking the final Hire button results in a real-shaped UUIDv7 (not `stub-` prefixed) in the URL
- [ ] `/workers/<that-id>/overview` renders the just-hired persona's name in the browser tab title (not the UUID)
- [ ] `/team` deck shows 6 cards after the hire (5 SAMPLE_TEAM + 1 new), not 5
- [ ] Closing the tab + reopening the SPA cleans the demo roster back to 5 (sessionStorage scoping)
- [ ] `pnpm --dir web exec vitest run lib/api/workers` green
- [ ] `pnpm --dir web exec tsc --noEmit` green

## May-touch

- `web/lib/api/workers.ts` (~30 LOC — replace stub branch with append-to-store)
- `web/lib/api/team.ts` (~10 LOC — read appended workers)
- `web/lib/api/worker-overview.ts` (~10 LOC — read appended workers)
- `web/lib/api/demo-roster-store.ts` (~60 LOC new file)
- `web/app/(app)/workers/[workerId]/overview/layout.tsx` (~15 LOC — async metadata; can fold into `fix-worker-detail-page-title-and-server-client-tab-helper` if landing simultaneously)
- `web/lib/api/__tests__/workers.test.ts` (~40 LOC test)

## Out of scope

- Real backend persistence (Option B above; operator-decisional — outside POC demo scope)
- Refactoring `SAMPLE_TEAM` into the new store (leave both; new hires append-only)
- Auth-bypass on/off recording-prep toggle (covered by `fix-demo-recording-disable-auth-bypass-prep`)

## Effort

M. ~125 LOC + ~40 LOC test. ~2-3 hours. P0 because the headline "hire a teammate" demo beat lands on a broken page in the recording configuration.
