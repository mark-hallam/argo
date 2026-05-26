# fix-spa-demo-live-mode-route-real-gateway-on-camera

**Owner**: claude.
**Authored against**: origin/main `56f4ea0b4d84ebf52c14e387d95c4e99e1cca0b7` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-spa-demo-live-mode-route-real-gateway-on-camera`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-27-0415/scene-6-claim-6-trust-and-approvals.md` F1 [P0] + F9 [P2] + F14 [P3].

🔴 **P0 NARRATIVE-BLOCKER — In demo + auth-bypass config (which the v1.0 MP4 records in), the entire audit / replay / approval-decision path NEVER reaches the gateway. The SPA short-circuits to `mockPage()` / `sampleOrFailed()` / `{ok: true, source: "mock-auth-bypass"}` before any HTTP call fires. The "Real provenance" narration of Scene 6 step 7 is theatre the camera sees if the operator says "this hits the real backend".**

## Root cause

`.claude/launch.json` sets both `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1` AND `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1`. In this state:

- `web/lib/api/audit.ts:346-348` short-circuits to `mockPage()` — no `GET /api/v1/audit`.
- `web/lib/api/audit-detail.ts:99-101` short-circuits to `mockDetail()` — no `GET /api/v1/audit/{id}`.
- `web/lib/api/replay.ts:259-261` short-circuits to `sampleOrFailed(actionId)` — no `POST /api/v1/audit/{id}/replay`.
- `web/lib/api/approvals.ts:207-209, 249-251` returns `{ok:true, source:"mock-auth-bypass"}` instantly — no `POST /api/v1/approvals/{id}/decide`.

Per the function-vs-dressing rule (`docs/skycot/demos/v1-demo-positioning-poc-vs-public.md`), the audit-row drill-in is function-side (it claims to do cryptographic provenance lookup against the real audit chain). Showing canonical seed JSON here is a function-side fabrication when the narration claims "this is the real audit row".

The gateway IS working end-to-end — curl-tested against the live demo tenant returns real `decision_chain`, `provenance.hash_chain_anchor`, `replay verdict:"same"`, cross-tenant 404 enforcement. The data path exists; the SPA just doesn't use it in demo+bypass mode.

Three coupled findings (F1, F9, F14) all share this root cause:

- F1: audit/replay/approve all short-circuit
- F9: replay client never reaches gateway; the well-written `adaptGatewayReplay` is dead code in demo mode
- F14: approve button shows green tick before any backend call (because it returns synchronously from the mock branch)

## Findings this brief closes

- scene-6-claim-6-trust-and-approvals F1 [P0] — In demo + auth-bypass config the entire audit/replay/approve path bypasses the gateway
- scene-6-claim-6-trust-and-approvals F9 [P2] — Replay client never reaches gateway in auth-bypass
- scene-6-claim-6-trust-and-approvals F14 [P3] — Approve button shows green tick before any audit row exists

## Fix recipe

**Introduce a third mode `NEXT_PUBLIC_SKYCOT_DEMO_LIVE=1`** that keeps the seeded copy + persona avatars but routes audit / replay / approve through the real gateway. The mode triplet then becomes:

| Mode | Purpose | API path |
|---|---|---|
| auth-bypass + demo-mode (today) | local UI iteration | mock-only, no gateway |
| demo-live + demo-mode (NEW) | recording-mode | real gateway calls, seeded responses |
| signed-in + no-demo | production | real gateway, real user data |

1. **Add `isDemoLive()` helper** at `web/lib/onboarding/demo-mode.ts` (or sibling). Returns true when `NEXT_PUBLIC_SKYCOT_DEMO_LIVE=1`.
2. **Update the 4 short-circuit branches**:
   - `audit.ts:346` — `if (isAuthBypassed() && !isDemoLive())` → `mockPage()`
   - `audit-detail.ts:99` — same guard
   - `replay.ts:259` — same guard
   - `approvals.ts:207, 249` — same guard
3. **Wire the gateway path** under `isDemoLive()`. The existing `apiGet` / `apiPost` calls already exist; the short-circuit just stops firing.
4. **Add `NEXT_PUBLIC_SKYCOT_DEMO_LIVE=1` to `.claude/launch.json`** as a separate launch profile (or the recording-day profile) — operator-controlled toggle.
5. **Update the demo runbook** `docs/skycot/demos/v1-mission-mvp-runbook.md` pre-record checklist: "verify `DEMO_LIVE=1` if recording with real-gateway narration; verify `DEMO_LIVE=0` for fast local iteration".
6. **Pre-record dependency**: `argo demo seed --tenant <demo>` MUST have run against the live gateway (covered by `fix-demo-seed-cli-operator-action-runbook`) so the gateway returns real audit / approval rows that match the seeded persona names.

## Acceptance criteria

- [ ] With `DEMO_LIVE=1` set, opening `/audit` fires a real `GET /api/v1/audit?range=7d` (visible in browser network panel).
- [ ] Clicking a row fires `GET /api/v1/audit/<id>`.
- [ ] Clicking Replay fires `POST /api/v1/audit/<id>/replay` and the verdict comes from `adaptGatewayReplay`.
- [ ] Clicking Approve fires `POST /api/v1/approvals/<id>/decide` and the audit feed shows a new row after the decision.
- [ ] With `DEMO_LIVE=0`, behaviour is unchanged from today (mock-only, fast local iteration).
- [ ] Playwright spec at `web/playwright/demo-live-mode/audit-fires-gateway.spec.ts` asserts the network panel shape under each mode.

## May-touch

- `web/lib/onboarding/demo-mode.ts` (add `isDemoLive()`)
- `web/lib/api/audit.ts:346-348` (guard update)
- `web/lib/api/audit-detail.ts:99-101` (guard update)
- `web/lib/api/replay.ts:259-261` (guard update)
- `web/lib/api/approvals.ts:207-209, 249-251` (guard updates)
- `.claude/launch.json` (add recording-mode env-var)
- `docs/skycot/demos/v1-mission-mvp-runbook.md` (pre-record checklist)
- `web/playwright/demo-live-mode/audit-fires-gateway.spec.ts` (NEW)

## Out of scope

- Gateway-side audit row seeding (covered by `fix-demo-seed-emit-worker-hired-audit-rows`)
- Cross-tenant header forwarding (covered by `fix-spa-client-forward-x-argo-tenant-id-header`)
- Audit fallback on 200-empty (covered by `fix-approvals-audit-fetchers-fallback-on-200-empty-in-demo-mode`)
- Replay endpoint contract alignment (covered by `fix-replay-endpoint-contract-spa-vs-gateway-mismatch`)

## Effort

~80-150 LOC + 1 Playwright spec + 1 doc update. ~3-5 hours. P0 because the operator cannot truthfully narrate "real backend" during Scene 6 today.
