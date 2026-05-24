# Stream fix-web-workers-api-envelope-and-id-format — unwrap /api/v1/workers response + fix demo worker ID

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `f3d1c66a6` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-web-workers-api-envelope-and-id-format`.
🔴 **P0 DEMO-BLOCKER.** `/workers/wkr_01J0MARIAH/overview` stuck on "Loading worker..." spinner forever. Two underlying issues: (1) the gateway rejects the demo worker ID with `"invalid worker_id 'wkr_01J0MARIAH': invalid character: found 'w' at 1"` — the `wkr_` prefix isn't a valid ULID, and the gateway parses worker IDs as strict ULIDs. (2) The frontend may also have the same envelope-vs-bare-array issue that bit `/team` (`fetchTeam` was typing the response as bare `Worker[]` while the gateway returned `{workers, total_count}` envelope — see commit `1d1085aab`).

## Scope

Two coordinated fixes:

**(a) Worker ID format.** Either:
- Update the demo voiceover script + page reference to use a valid ULID-format worker ID for the demo (e.g. `01J0M_VALID_ULID_HERE`), AND
- Update SAMPLE_PERSONAS in `web/lib/sample-personas.ts` so its worker IDs match (the demo navigates to `${workerId}/overview` — the link target needs to be navigable).

Or:
- Loosen the gateway's worker_id parser to accept `wkr_` prefix (NOT recommended — gateway invariants matter).

Pick option 1 (frontend-side fix). The demo script can use whatever ID we pick.

**(b) Envelope unwrap.** Apply the same fix shape as commit `1d1085aab` (which fixed `/team`): inspect what `/api/v1/workers/{id}` returns, and if it's wrapped in an envelope (e.g. `{worker: Worker, ...}`), unwrap it in `fetchWorker`. If it's a bare `Worker`, leave alone.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `web/lib/api/workers.ts` — fix the `fetchWorker` (or whatever the worker-fetch helper is named) shape parsing. Pattern template: `web/lib/api/team.ts` after the envelope-unwrap fix at commit `1d1085aab`.
- `web/lib/sample-personas.ts` — replace `wkr_01J0MARIAH` (and any other `wkr_`-prefixed IDs) with valid ULID strings. Use the operator's discretion or generate fresh ULIDs.
- `docs/skycot/operator/demo-runbook.md` — update the voiceover script's `wkr_01J0MARIAH` reference to the new ID.
- `web/app/(app)/workers/[workerId]/overview/page.tsx` — only if the page itself hardcodes the demo ID; otherwise leave.
- `web/lib/sample-worker-overview.ts` — if it references the same ID.
- Any `tests/e2e/*.spec.ts` that navigates to `/workers/wkr_01J0MARIAH`.

## Files you must NOT touch

- Any `crates/argo-*/**` file — the gateway parsing is correct; don't loosen invariants
- Other sample-*.ts files unless they reference the renamed ID
- Other route handlers under `web/app/(app)/`

## Acceptance criteria

- [ ] `/workers/{new-valid-id}/overview` loads in the browser within 3s — shows Mariah's personality card + responsibilities + KPIs per the voiceover
- [ ] No "Loading worker..." spinner that doesn't resolve
- [ ] `pnpm typecheck` clean
- [ ] `pnpm test` — no regressions; sample-personas-related tests updated
- [ ] PR body includes a curl test against the gateway showing the new ID resolves
- [ ] Demo runbook's voiceover script updated (line ~74-76 of `docs/skycot/operator/demo-runbook.md`)
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-fix-web-workers-api-envelope-and-id-format`
- PR title: `web(workers): unwrap envelope + use valid ULID for demo worker (fix-web-workers-api-envelope-and-id-format)`

## Notes for the implementer

- Diagnosed during the 2026-05-24 demo walkthrough. Gateway returns: `{"error":{"code":"bad_request","message":"invalid worker_id 'wkr_01J0MARIAH': invalid character: found 'w' at 1"}}` on `GET /api/v1/workers/wkr_01J0MARIAH`. Use a fresh ULID (26 chars, Crockford base32) for the demo persona.
- The voiceover refers to "Mariah" by name, not by ID — so the name stays, only the underlying ID changes. The persona's display_name, role, traits, all stay the same.
- This is sibling to `fix-web-mock-bypass-env-var-rename`. Together with mock-bypass working, this brief unblocks the worker-profile scene of the demo (~2:30-4:00 in the voiceover script).
