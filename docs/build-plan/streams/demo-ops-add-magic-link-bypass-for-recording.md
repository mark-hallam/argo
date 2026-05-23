# demo-ops-add-magic-link-bypass-for-recording

**Owner**: claude.
**Depends on**: nothing.
**Authored against**: origin/main `5a626ef98` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh demo-ops-add-magic-link-bypass-for-recording`.

🔴 **P0 — v1.0 demo cannot be recorded end-to-end through the browser today; operator-recording is blocked.**

## Why this is P0

Round 30 of the evaluator drove `docs/skycot/demo_script.md` (the 5-7 minute v1.0 mission demo) live through Chrome MCP against `localhost:3000` (SPA) + `127.0.0.1:7400` (gateway). Verdict: the demo runs cleanly through Scenes 0 + 1 + 2.1 + 2.2 (cold-open marketing landing → signup form → onboarding welcome → workspace step) — then BLOCKS at Scene 2 step 3 (provider key) when "Continue" on the workspace step POSTs `/api/v1/onboarding/provision` to the gateway. The gateway correctly returns:

```
HTTP 401
X-Argo-Principal header must carry a verified `human:<email>` value
```

This is **correct safety behavior** — an unauthenticated provision call would be a tenant-creation DoS vector. The 401 is the safety net working.

But it's **also the demo barrier**: the demo script's Scene 1 narration says "cut to inbox, the magic-link email arrives, click the link." Real-machine recording requires either:

- (a) an actual SMTP / email-delivery loop wired up, throwaway address visible on screen during recording, OR
- (b) a dev-mode flag that mints a verified principal session locally WITHOUT a real email round-trip.

Option (b) is what every recording session needs. Without it, the demo cannot be filmed as a single take per the operator-locked "single take, no edits" hidden constraint in `demo_script.md`.

The full evaluator-round-30 report is at `docs/skycot/evals/scheduled/2026-05-23/round-30-demo-e2e-chrome/findings.md`.

## Scope

Add an env-var-gated demo-mode signup path that mints a verified `human:<email>` principal session without an SMTP round-trip. Gated by `ARGO_DEMO_MODE=1` so it cannot be active in a production gateway.

## Hard constraints (operator-locked)

1. **MUST be env-var-gated.** The bypass MUST be inactive unless `ARGO_DEMO_MODE=1` is set in the gateway's startup environment. The flag is read once at startup and cached; no runtime flip.
2. **MUST log a warning on every use.** Each time the bypass mints a principal, emit `tracing::warn!(target: "argo_cli::demo_mode", "demo-mode magic-link bypass minted principal for {email}")` so an operator running the gateway with the flag accidentally sees obvious noise.
3. **MUST also write an audit row.** The minted-principal call writes an `auth.succeeded` (existing `AuditKind::AuthSucceeded`) audit row with `payload.demo_mode_bypass: true` and `payload.email: <email>`. This is the forensic record that distinguishes a demo-recording session from a real production sign-in.
4. **MUST NOT bypass cross-tenant scoping.** The minted principal is bound to the tenant the gateway booted under (or a fresh tenant if no workspace exists yet); it does NOT inherit any other tenant's data.
5. **MUST fail loud if `ARGO_DEMO_MODE=1` is set in a release build with bearer-auth enabled.** Production deployments use bearer-auth — if both `ARGO_DEMO_MODE=1` AND `ARGO_GATEWAY_AUTH_BYPASS` is NOT set, the gateway refuses to start with a clear message: `"ARGO_DEMO_MODE=1 cannot be used in production; set ARGO_GATEWAY_AUTH_BYPASS=1 or unset ARGO_DEMO_MODE"`.

## Scope (what to ship)

### Backend

1. New gateway route `POST /api/v1/demo/mint-principal` (only mounted when `ARGO_DEMO_MODE=1`). Request body: `{"email": "demo@throwaway.cc"}`. Response: `{"ok": true, "principal": "human:demo@throwaway.cc", "session_token": "<opaque>"}`. The session-token is a real signed cookie-equivalent value the SPA's BFF can stamp into `X-Argo-Principal` for subsequent calls.
2. New `tracing::warn!` on every mint (constraint #2).
3. New `AuditKind::AuthSucceeded` row write with `demo_mode_bypass: true` (constraint #3).
4. Startup check in `crates/argo-cli/src/handlers/gateway/startup_check.rs`: if `ARGO_DEMO_MODE=1` is set AND bearer-auth is enabled (i.e., `ARGO_GATEWAY_AUTH_BYPASS` is NOT set), refuse to bind with the clear message (constraint #5).

### SPA

5. In `web/middleware.ts` or equivalent, when `process.env.NEXT_PUBLIC_ARGO_DEMO_MODE === "1"`, the SPA's signup form's "Send magic link" button calls the new `POST /api/v1/demo/mint-principal` instead of the real magic-link flow. The verified email round-trip is bypassed; the user lands directly on `/onboarding/welcome`.
6. Demo-mode visual indicator: a small "DEMO MODE — auth bypass active" banner at the top of every page when the BFF detects the env var. So the operator (and anyone watching the recording, if it ships) can SEE the demo-mode toggle.

### Documentation

7. Add a new `docs/skycot/runbook/v1-demo-recording.md` §3.5 that documents the `ARGO_DEMO_MODE=1` flag, what it does, what it logs, and how to verify it's working. Operator copies the flag into their demo-recording terminal alongside `SKYCOT_DB_KEY_BASE64` and `ANTHROPIC_API_KEY`.

### Regression test (within evaluator's existing tests/)

8. New file `crates/argo-cli/tests/demo_mode_mint_principal_smoke.rs` proving: (a) without `ARGO_DEMO_MODE=1`, the route returns 404; (b) with `ARGO_DEMO_MODE=1` AND `ARGO_GATEWAY_AUTH_BYPASS=1`, the route mints a session + writes the audit row with `demo_mode_bypass: true`; (c) with `ARGO_DEMO_MODE=1` AND bearer-auth enabled, the gateway refuses to bind.

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/` — new route, new startup-check branch.
- `web/middleware.ts` + `web/app/auth/signup/page.tsx` — demo-mode signup branch + banner.
- `docs/skycot/runbook/v1-demo-recording.md` — new §3.5.
- `crates/argo-cli/tests/demo_mode_mint_principal_smoke.rs` — new test file.

## Files you must NOT touch

- `crates/argo-cli/src/handlers/gateway/routes_onboarding.rs` — the real auth path stays as it is; demo-mode is a separate route.
- `crates/argo-channels/src/email_*.rs` — the real magic-link mail path stays unchanged.
- Any production middleware. Demo-mode is purely an env-var feature on top of the existing auth surface.

## Done means

- `ARGO_DEMO_MODE=1 ARGO_GATEWAY_AUTH_BYPASS=1 SKYCOT_DB_KEY_BASE64=... ANTHROPIC_API_KEY=sk-ant-placeholder argo gateway start` boots cleanly.
- Driving the SPA via `npm run dev` against this gateway lets the operator click through `/landing → /auth/signup → "Send magic link" → /onboarding/welcome → /onboarding/workspace → /onboarding/connect-provider` without a single 401 or 503.
- The new test passes.
- The new runbook §3.5 explains the flag in plain language with a copy-pasteable env block.
- `ARGO_DEMO_MODE=1` without `ARGO_GATEWAY_AUTH_BYPASS=1` makes the gateway refuse to start with the clear error.

## Why this matters NOW

The v1.0 demo is the ship gate. Operator needs to record the 5-7 minute demo before 2026-05-30. Without this flag, the operator either has to wire up a real SMTP server (hours of work, fragile) or invent a workaround on the night of the recording. P0.

## Authoring trail

This brief was authored by the evaluator after round 30 (live Chrome MCP walkthrough of the demo). Findings + screenshots: `docs/skycot/evals/scheduled/2026-05-23/round-30-demo-e2e-chrome/findings.md`. Companion brief: `docs-demo-runbook-add-skycot-db-key-section` (P0/P1 docs-only fix for the other recording-blocker).
