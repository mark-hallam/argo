# fix-gateway-per-vertical-workflows-401-auth-mask

**Owner**: codex.
**Authored against**: origin/main `6f5d5c49da2e` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-gateway-per-vertical-workflows-401-auth-mask`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-1441/scene-4-and-5-claim-4-and-5.md` F9.

🟠 **P1 — `GET /api/v1/verticals/<v>/workflows` returns 401 unauthorized even when `ARGO_GATEWAY_AUTH_BYPASS=1` is set server-side. SPA catches 401 as `isRecoverable` and silently falls back to `SAMPLE_WORKFLOW_SUMMARIES`. Camera-visible workflow cards on /workflows are fixtures, not real data — and the "mock-offline" source label is the only indication the operator never sees.**

## Root cause

The auth middleware that gates `/api/v1/verticals/*` (`crates/argo-cli/src/handlers/gateway/routes_verticals*.rs`) does NOT honor the gateway-side `ARGO_GATEWAY_AUTH_BYPASS=1` flag the same way `/api/v1/team` and `/api/v1/skills` do. The principal-required check fires unconditionally.

The SPA client (`web/lib/api/workflows.ts:113-128`, `fetchAllWorkflows`) treats 401 as recoverable per `isRecoverable(error)`:
```ts
catch (error) {
  if (isRecoverable(error)) return { workflows: [], source: "mock-offline" };
  throw error;
}
```

Net result: gateway returns 401, SPA swallows it, falls to `SAMPLE_WORKFLOW_SUMMARIES`, renders pretty workflow cards (with substrate chips, version pills, last-edited timestamps) that LOOK real. The function-side claim "every workflow you see is from your tenant" is undermined.

Curl evidence:
```
curl http://127.0.0.1:7400/api/v1/verticals/sales/workflows
→ {"error":{"code":"unauthorized","message":"X-Argo-Principal header is required"}}

curl :3000/workflows | grep workflow-card
→ 7 sample cards from SAMPLE_WORKFLOW_SUMMARIES, source = "mock-offline"
```

## Findings this brief closes

- scene-4-and-5-claim-4-and-5 F9 [P1] — Per-vertical workflow endpoint 401s; SPA silently masks to fixtures

## Fix recipe

1. **Gateway side — apply auth-bypass to verticals routes**:
   - Identify the handler for `/api/v1/verticals/<v>/workflows` (likely `routes_verticals_workflows.rs` or sibling under `crates/argo-cli/src/handlers/gateway/`).
   - Apply the same `ARGO_GATEWAY_AUTH_BYPASS=1` short-circuit that `/api/v1/team` uses: when set, inject a synthetic principal derived from the workspace's demo-mint record (mirroring the pattern in `crates/argo-cli/src/handlers/gateway/routes_team_list.rs`).
   - In production (`ARGO_GATEWAY_AUTH_BYPASS` unset/0), behavior is unchanged: require `X-Argo-Principal`.

2. **SPA side — surface the failure, don't mask it**:
   - In `web/lib/api/workflows.ts:113-128`, distinguish 401 (auth misconfig) from 503/network (gateway down). 401 should NOT be `isRecoverable` → should bubble to a banner: "Couldn't fetch workflows — sign in to see your workspace's."
   - 503/network → fall to mock with `source: "mock-offline"` as today.
   - This makes the failure mode VISIBLE so future audits don't silently sit on fixtures.

3. **Apply same pattern to siblings** that have the same 401-mask risk: `/api/v1/verticals/<v>/skills`, `/api/v1/verticals/<v>/duties`, `/api/v1/verticals/<v>/runs`. Audit grep:
   ```
   rg "isRecoverable\(error\)" web/lib/api/ -A 3
   ```

4. **Integration test**: `crates/argo-cli/tests/gateway_verticals_auth_bypass.rs`:
   - With `ARGO_GATEWAY_AUTH_BYPASS=1` env, `GET /api/v1/verticals/sales/workflows` returns 200 + workflows list.
   - Without the flag, returns 401.

5. **SPA test**: `web/lib/api/workflows.test.ts` (new or extend):
   - 401 response → throws, banner can render.
   - 503 response → returns `{workflows: [], source: "mock-offline"}`.

## Acceptance criteria

- [ ] `curl :7400/api/v1/verticals/sales/workflows` with `ARGO_GATEWAY_AUTH_BYPASS=1` returns 200 + the workflows array.
- [ ] Same without the env flag returns 401 (production unchanged).
- [ ] `/workflows` page in dev (bypass on) renders real gateway data, NOT fixtures.
- [ ] 401 in SPA surfaces a banner; 503 falls to mock as today.
- [ ] Sibling routes (`/skills`, `/duties`, `/runs` per vertical) get the same treatment OR have follow-up tickets opened.
- [ ] New integration test + SPA test pass cold.

## May-touch

- `crates/argo-cli/src/handlers/gateway/routes_verticals_workflows.rs` (auth-bypass branch)
- `crates/argo-cli/src/handlers/gateway/routes_verticals_skills.rs` (sibling — same pattern)
- `crates/argo-cli/src/handlers/gateway/routes_verticals_duties.rs` (sibling)
- `crates/argo-cli/src/handlers/gateway/routes_verticals_runs.rs` (sibling)
- `crates/argo-cli/tests/gateway_verticals_auth_bypass.rs` (NEW)
- `web/lib/api/workflows.ts` (distinguish 401 from 503)
- `web/lib/api/workflows.test.ts` (extend)
- `web/lib/api/skills.ts` + `web/lib/api/duties.ts` (mirror)

## Out of scope

- `.claude/launch.json` env var typo (covered by `fix-claude-launch-json-env-var-naming-mismatch`)
- CORS port-pin issue (covered by `fix-gateway-cors-localhost-127-and-preview-proxy-origins`)
- The demo seed itself (`fix-demo-seed-cli-for-real-roster`)

## Effort

~150-220 LOC across 4-5 gateway handlers + 2 tests + 2 SPA files. ~4-5 hours.
