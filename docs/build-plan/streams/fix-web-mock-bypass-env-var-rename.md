# Stream fix-web-mock-bypass-env-var-rename — make ARGO_GATEWAY_AUTH_BYPASS visible to client code

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `f3d1c66a6` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-web-mock-bypass-env-var-rename`.
🔴 **P0 DEMO-BLOCKER.** Mock-bypass mode is currently un-callable from the browser. Several `lib/api/*.ts` `fetch*` functions check `process.env.ARGO_GATEWAY_AUTH_BYPASS === "1"` for the bypass branch, but Next.js only exposes `NEXT_PUBLIC_*`-prefixed env vars to client code. So `useTeam`, `useWorkers`, `useAudit` etc. that run in the browser never see the bypass flag, and the SAMPLE_TEAM / sample-personas / sample-* mock fallback never fires. This is the load-bearing blocker for the v1.0 demo recording (operator surfaced 2026-05-24): without mock-bypass, the demo's voiceover assumes 6 teammates / Mariah / pending approvals etc. but the workspace is empty and the gateway returns nothing to populate the pages.

## Scope

Two options; implementer picks based on which is cleaner:

**Option A (recommended)** — rename the env var. Add `NEXT_PUBLIC_` prefix everywhere:
- `ARGO_GATEWAY_AUTH_BYPASS` → `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS`
- Update all `process.env.ARGO_GATEWAY_AUTH_BYPASS` reads in `web/lib/api/*.ts` and any other client-side callers
- Update `.env.example`, `vercel.json`, `auth.config.ts`, `middleware.ts`, README docs
- Update test setup that sets the var (`playwright.config.ts`, e2e specs)
- Documentation: a one-liner in the demo runbook about which env var to set

**Option B** — move the bypass-decision server-side via Next.js route handlers. More refactor, less clean.

Pick A.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `web/lib/api/team.ts`, `web/lib/api/workers.ts`, `web/lib/api/audit.ts`, `web/lib/api/approvals.ts`, `web/lib/api/workflows.ts` — and any other `web/lib/api/*.ts` that checks the env var. Read each `isAuthBypassed()` helper and rename.
- `web/lib/skycot/demo-mode.ts` (if it has similar logic)
- `web/middleware.ts`, `web/auth.config.ts`, `web/playwright.config.ts`, `web/vercel.json` — config touchpoints
- `web/.env.example` — add the renamed var with a comment
- `web/README.md` — update docs
- `web/tests/e2e/*.spec.ts` — any test setup that sets the var
- `web/middleware.test.ts` — update test that asserts on the var
- `docs/skycot/operator/demo-runbook.md` — add a "Set this env var" pre-flight bullet
- `scripts/demo-dry-run.sh` — update the line that sets `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1` and `ARGO_GATEWAY_AUTH_BYPASS` to be consistent

## Files you must NOT touch

- Any `crates/argo-*/**` Rust file — this is a web-only fix
- The bypass LOGIC in `isAuthBypassed()` — only rename the env-var lookup, keep the `NODE_ENV !== "production"` guard so prod is never bypassed
- `web/lib/sample-*.ts` — fixtures unchanged

## Acceptance criteria

- [ ] `grep -rn 'process\.env\.ARGO_GATEWAY_AUTH_BYPASS' web/` returns 0 results in `lib/api/*.ts` (browser code); any remaining server-side references documented in PR
- [ ] `grep -rn 'NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS' web/` returns the new references
- [ ] Setting `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1` and starting `pnpm dev` → `/team` shows SAMPLE_TEAM (Mariah + 5 others) rather than empty state
- [ ] `pnpm typecheck` clean
- [ ] `pnpm test` — no regressions (any failures are pre-existing per `KNOWN_FLAKES.md`)
- [ ] `pnpm lint` clean
- [ ] `.env.example` documents the new var with a one-line comment
- [ ] PR body lists every file touched + cites the symptom (mock-bypass invisible to browser)
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-fix-web-mock-bypass-env-var-rename`
- PR title: `web: rename ARGO_GATEWAY_AUTH_BYPASS to NEXT_PUBLIC_ for client visibility (fix-web-mock-bypass-env-var-rename)`

## Notes for the implementer

- The bug was diagnosed live during the 2026-05-24 demo-recording walkthrough (operator + claude). Server-side code can still read `ARGO_GATEWAY_AUTH_BYPASS` if needed — only the browser-side reads need the rename.
- If you find non-API callers that legitimately need to be server-side only, leave those alone. The rename is specifically for the `isAuthBypassed()` helpers in `lib/api/*.ts`.
- Don't add a backwards-compat shim — clean break, prod was always supposed to bypass via `NEXT_PUBLIC_` semantics.
