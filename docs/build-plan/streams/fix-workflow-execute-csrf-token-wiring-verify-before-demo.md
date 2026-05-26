# fix-workflow-execute-csrf-token-wiring-verify-before-demo

**Owner**: claude.
**Authored against**: origin/main `4b3aa443f` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-workflow-execute-csrf-token-wiring-verify-before-demo`.
**Source**: UX-AUDIT-T0519 2026-05-27 — `docs/skycot/ux-audits/2026-05-27-0519/scene-4-and-5-claim-4-and-5.md` F9.

🟡 **P1 — Workflow detail "Run now" button POSTs `/api/v1/verticals/<v>/workflows/<n>/execute`. Direct fetch in preview yielded `{"error":"csrf_missing","reason":"X-CSRF-Token header required for cookie-auth requests"}` (HTTP 403). SPA's `apiPost` is supposed to attach the token automatically — but the auditor couldn't verify the browser path end-to-end. Scene 5 narrates "click Run now → audit row lands in 5s"; if CSRF wiring is broken, this beat fails on camera.**

## Root cause

`web/lib/api/client.ts` (the `apiPost` shared client) is supposed to:
1. Read the CSRF token from a cookie or `meta` tag on page load.
2. Attach `X-CSRF-Token: <value>` to every mutating request (POST/PATCH/DELETE).

The auditor's direct curl from a preview eval (not the in-browser button) returned 403 csrf_missing. This DOES NOT mean the browser button is broken — the curl bypasses the SPA's client. But the auditor couldn't confirm the in-browser path lands (Preview MCP wedged repeatedly during the audit run; some calls succeeded earlier sessions).

Risk: a regression in `apiPost` CSRF attachment (or token expiry mid-session) silently breaks the Run-now button. Demo MP4 captures the click + the next scene assuming success.

Per `feedback_no_theater_in_demo`: this is function-side — Run now must actually fire and write an audit row, the next scene depends on it.

Filing as a P1 verification + hardening brief.

## Findings this brief closes

- scene-4-and-5-claim-4-and-5 F9 [P1] — Run-now POST gateway rejects with 403 csrf_missing on direct call; browser-path verification incomplete

## Fix recipe

1. **Verify the browser path end-to-end.**
   - Run `pnpm dev` against the live gateway.
   - Open `/workflows/sales/qualify_lead` (or any seeded workflow).
   - DevTools → Network → click Run now.
   - Confirm request carries `X-CSRF-Token: <value>` header.
   - Confirm response is 200 with `{ "execution_id": "...", ... }`.
   - Confirm `/audit?range=5m` shows the new execution row within 5s.

2. **If the header is missing in browser path**, fix `web/lib/api/client.ts`:
   - Confirm token source (cookie name or meta tag).
   - Ensure the `apiPost` / `apiPatch` / `apiDelete` helpers attach the header on every request.
   - Add a defensive log if the token is missing at request time.

3. **If the header is present and gateway still 403s**, the gateway-side CSRF validation may be misconfigured for the route. Check `crates/argo-cli/src/handlers/gateway/middleware/csrf.rs` (or wherever validation lives) — verify the workflow-execute route is on the protected list AND the validation accepts the token shape the SPA sends.

4. **Vitest pin** at `web/lib/api/__tests__/client.test.ts`:
   - Calling `apiPost('/foo', body)` reads the token from the configured source and attaches the header.
   - Missing token raises a clear error (not a silent 403).

5. **Cargo test** at `crates/argo-cli/tests/workflow_execute_csrf.rs`:
   - POST `/api/v1/verticals/<v>/workflows/<n>/execute` WITHOUT CSRF token → 403.
   - POST WITH valid CSRF token → 200 + audit row written.

6. **Pre-record checklist update**: add to `docs/skycot/demos/v1-mission-mvp-runbook.md` Scene 5 — "Smoke-test Run now button in a fresh tab before recording. Confirm 200 + audit row within 5s."

## Acceptance criteria

- [ ] Documented evidence (DevTools network screenshot OR test transcript) showing the browser path POSTs with `X-CSRF-Token` header and gets 200.
- [ ] Vitest pin: `apiPost` attaches the token.
- [ ] Cargo test: gateway rejects no-token, accepts valid-token.
- [ ] Pre-record checklist updated.

## May-touch

- `web/lib/api/client.ts` (verify + harden CSRF attachment)
- `web/lib/api/__tests__/client.test.ts` (pin)
- `crates/argo-cli/src/handlers/gateway/middleware/csrf.rs` (if gateway side needs touch)
- `crates/argo-cli/tests/workflow_execute_csrf.rs` (NEW)
- `docs/skycot/demos/v1-mission-mvp-runbook.md` (checklist)

## Out of scope

- New workflow seeds (covered by `fix-scene-5-customer-research-followup-workflow-and-mcp-cu-step-kinds.md`)
- Workflow-builder canvas changes
- Run-now button visual design (covered by `fix-workflow-detail-run-now-buried-among-five-ctas.md`)

## Effort

S — ~30-80 LOC across pins + minor client hardening + checklist. ~1-2 hours.
