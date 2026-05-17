# mission-deployed-real-e2e-lane — production-path E2E lane against real deployed gateway (P1 audit-finding-11)

**Owner**: claude. **Depends on**: `claude-magic-link-auth-backend` MERGED (#1796) + `claude-tenant-provisioning-endpoint` MERGED (#1791).
**Authored against**: origin/main `6b432ec4` on `2026-05-17`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-deployed-real-e2e-lane`.
**Source**: 2026-05-17 audit finding #11 (lines 213-229). Today's Playwright E2E is mock-fallback-heavy: most specs point `NEXT_PUBLIC_GATEWAY_URL` at `http://127.0.0.1:1` and prove fallback rather than backend integration.

## Mission alignment

- **Reliability** (CLAUDE.md priority #1) — proving graceful fallback is necessary but not sufficient; mission proof needs the real-backend path.
- **Repeatability** (#3) — production-path runs identically in CI + on operator machines.

## Scope

P1. Add a separate Playwright E2E lane that targets a real deployed gateway (Fly app, staging, or operator-hosted) and exercises the full activation path end-to-end. Keep existing mock-fallback specs — they prove different things.

### New lane: `web/tests/e2e/deployed/`

```
web/tests/e2e/deployed/
├── onboarding.spec.ts       # signup → magic-link → wizard → first hire
├── approval-roundtrip.spec.ts # action → ApprovalAsked → operator approve → ActionExecuted
├── audit-replay.spec.ts      # audit chain verifiable end-to-end
└── hire-flow-real.spec.ts    # Genesis classify → draft → workers POST (no stubs)
```

### Trigger mechanism

```bash
# Local lane (existing): mock-fallback, fast
pnpm exec playwright test web/tests/e2e/

# Deployed lane (NEW): real backend, slower
ARGO_E2E_TARGET=deploy \
ARGO_E2E_GATEWAY_URL=https://api.skycot.com \
ARGO_E2E_SPA_URL=https://app.skycot.com \
ARGO_E2E_MARKETING_URL=https://skycot.com \
ARGO_E2E_MAIL_CATCHER_URL=https://mailcatcher.example.com \
pnpm exec playwright test web/tests/e2e/deployed/
```

### Test fixtures

- **Throwaway email**: per-run unique address (e.g. `e2e-${TIMESTAMP}-${RANDOM}@skycot-test.com`).
- **Mail catcher**: real inbox webhook for magic-link interception. For now: poll a mailcatcher.io tenant or a self-hosted smtp4dev. Operator-resume fallback if no email in 60s.
- **Per-run cleanup**: after each spec, DELETE the test tenant + user_account + magic_link_token rows via admin endpoint.

### Reporting

- Lane runs report SEPARATELY from local lane in CI summary. "29 local-fallback passed; 4 deployed-real passed" is the green state.
- A red deployed-real run does NOT block PR merges automatically — it blocks the `deploy-smoke.yml` workflow (run after fly deploy). Operator decides whether to roll back.

### Scope-explicit non-goals

- Don't replace existing local lane.
- Don't try to make deployed lane runnable on every PR (cost + slowness).
- Don't add deployed lane to default `pnpm test` — it's `pnpm test:e2e:deploy`.

LOC: ~400.

## Files you may touch

- `web/tests/e2e/deployed/onboarding.spec.ts` (NEW) — signup → magic-link → wizard → first hire. ≤180 LOC.
- `web/tests/e2e/deployed/approval-roundtrip.spec.ts` (NEW). ≤120 LOC.
- `web/tests/e2e/deployed/audit-replay.spec.ts` (NEW). ≤100 LOC.
- `web/tests/e2e/deployed/hire-flow-real.spec.ts` (NEW). ≤140 LOC.
- `web/tests/e2e/deployed/fixtures/mail-catcher-deploy.ts` (NEW) — real mail-poll. ≤100 LOC.
- `web/tests/e2e/deployed/fixtures/tenant-cleanup.ts` (NEW) — per-run teardown via admin endpoint. ≤80 LOC.
- `web/playwright.config.ts` — branch behavior on `ARGO_E2E_TARGET=deploy`. ≤40 modified LOC.
- `web/package.json` — add `test:e2e:deploy` script. ≤3 modified LOC.
- `.github/workflows/deploy-smoke.yml` (existing) — add deployed E2E step gated on `ARGO_E2E_DEPLOYED_URL`. ≤30 modified LOC.
- `docs/skycot/operator/deployed-e2e.md` (NEW) — operator runbook. ≤80 LOC.

## Files you must NOT touch

- Existing `web/tests/e2e/*.spec.ts` (local-fallback lane) — additive only.
- Backend code — deployed lane only consumes the existing HTTP surface.
- Production tenant data — every test uses throwaway email + cleanup teardown.

## Acceptance criteria

- [ ] `pnpm test:e2e` (local) runs existing 30+ specs without regression.
- [ ] `pnpm test:e2e:deploy` runs 4 deployed specs against a real gateway.
- [ ] Each deployed spec creates a per-run tenant + cleans it up on completion (verify no leaked test rows in DB after suite).
- [ ] Magic-link is intercepted via real mail (operator-resume fallback if delivery >60s).
- [ ] Deployed lane runs in CI on `deploy-smoke.yml` AFTER `fly deploy` succeeds.
- [ ] Deployed lane RED → workflow fails but doesn't block PR merges (operator-decision-only).
- [ ] Audit chain after a deployed spec → `argo audit verify-chain --json` returns clean.
- [ ] Operator runbook documents: how to run locally vs deploy, throwaway email setup, mail-catcher options (mailcatcher.io / smtp4dev / Postmark inbox endpoint).
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `mission-deployed-real-e2e-lane`
- Title: `test(e2e): deployed-real lane against real gateway (mission-deployed-real-e2e-lane)`
- Worker verification: `pnpm typecheck && pnpm test && pnpm exec playwright test web/tests/e2e/` (local lane only — deployed lane needs operator-side deploy) per SWARM_PROTOCOL.md.

## Hidden constraints

- **Pre-claim check: confirm #1796 magic-link MERGED.** Worker `git log origin/main -- crates/argo-cli/src/handlers/gateway/routes_auth.rs | grep magic_link` should return matches.
- **`E2E_TENANT_PROVISION_READY` flag** — local lane gates steps 9/10 on this. Deployed lane assumes it's TRUE (provisioning #1791 is MERGED).
- **Mail catcher selection** — for v1, smtp4dev in a Docker container is simplest. Postmark's inbox endpoint is cleaner if operator runs Postmark in test mode. Document both.
- **No production data writes.** All test tenants use a prefix (e.g. `e2e-test-<ts>-`) so admin-side cleanup is unambiguous.
- **Slack/voice approval scenarios** require real Slack workspace OR a stub. Phase the slack test behind `ARGO_E2E_SLACK_AVAILABLE=1` env so the lane runs without Slack.
- **Closes audit finding #11.** When this lands, the mock-fallback dominance flagged in the audit's E2E section is resolved.
