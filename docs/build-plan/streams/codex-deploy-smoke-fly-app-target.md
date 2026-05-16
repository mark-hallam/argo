# codex-deploy-smoke-fly-app-target — extend smoke-skycot.sh to target deployed Fly app (P2)

**Owner**: codex. **Depends on**: `codex-hosted-deployment-flyio` MERGED (#1757, 543bffa).
**Authored against**: origin/main `2b984b94` on `2026-05-16`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-deploy-smoke-fly-app-target`.
**Source**: PR #1757 follow-up — fly.toml + Dockerfile shipped, but `scripts/smoke-skycot.sh` only smokes against `localhost`. Need post-deploy validation.

## Mission alignment

- **Reliability** (CLAUDE.md priority #1) — deploy without smoke validation is faith-based.
- **Observability** (#2) — smoke output captures specific failures (5xx, 4xx, latency) so deploys fail fast.

## Scope

P2. Extend `scripts/smoke-skycot.sh` with an opt-in `--target-url <URL>` flag (default unset → localhost behavior unchanged). Add 8 new scenarios that exercise the deployed surface:

| # | Scenario | Method | Path | Expected |
|---:|---|---|---|---|
| 30 | health-deployed | GET | `/healthz` | 200, `{"ok":true,...}` |
| 31 | meta-deployed | GET | `/meta` | 200, includes `version` field |
| 32 | meta-tools-deployed | GET | `/meta/tools` | 200, includes at least 8 in-core tools |
| 33 | auth-signup-rate-limit | POST × 11 | `/api/v1/auth/signup` | 11th request 429 |
| 34 | auth-signup-happy | POST | `/api/v1/auth/signup` | 200 + `{ status: "sent" }` (uses throwaway email) |
| 35 | cors-preflight | OPTIONS | `/api/v1/dispatch` | 200 with CORS headers (`Access-Control-Allow-Origin`) |
| 36 | tls-cert | (curl -k disabled) | `<URL>` | TLS handshake succeeds, cert not expired |
| 37 | latency-budget | 100×GET | `/healthz` | p95 < 500ms (network-dependent) |

LOC: ~150.

## Files you may touch

- `scripts/smoke-skycot.sh` — add `--target-url <URL>` flag, add 8 scenarios gated on `$TARGET_URL` env. ≤140 modified LOC.
- `scripts/smoke-skycot-tests.sh` (existing or NEW) — golden output assertions for the local-only path. ≤30 modified LOC.
- `docs/skycot/operator/deploy-smoke.md` (NEW) — operator runbook for running smoke against a Fly deploy. ≤40 LOC.
- `.github/workflows/deploy-smoke.yml` (existing — added by #1732 ci re-enable) — extend to run against the deploy target after `flyctl deploy`, gated on `FLY_DEPLOYED_URL` env or workflow input. ≤20 modified LOC.

## Files you must NOT touch

- The CLI itself. Smoke is pure curl + jq.
- `deploy/flyio/*` config files added by #1757. Read-only.
- `argo-store` migrations — smoke is read-only against deployed instance.

## Acceptance criteria

- [ ] `bash scripts/smoke-skycot.sh` without `--target-url` behaves identically to today (local-only scenarios pass).
- [ ] `bash scripts/smoke-skycot.sh --target-url https://api.skycot.com` runs 30/30 scenarios (22 existing + 8 new).
- [ ] Each new scenario emits OK / SKIP / FAIL with reason line.
- [ ] Rate-limit scenario (#33) uses throwaway emails (e.g. `test-${RANDOM}@smoketest.local`) to avoid polluting real signup table.
- [ ] TLS cert scenario reports certificate expiry date (operator visibility).
- [ ] Latency scenario uses `curl -w '%{time_total}'` 100 times in parallel; emits p50/p95/p99 + budget pass/fail.
- [ ] Operator runbook documents 3 cases: fresh-deploy smoke, scheduled smoke (cron), failed-deploy diagnosis.
- [ ] `bash scripts/check-shellcheck.sh` (or `shellcheck scripts/smoke-skycot.sh`) clean.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `codex-deploy-smoke-fly-app-target`
- Title: `test(smoke): --target-url <URL> with 8 post-deploy scenarios (codex-deploy-smoke-fly-app-target)`
- Worker verification: shellcheck + dry-run against localhost + workspace clippy per SWARM_PROTOCOL.md.

## Hidden constraints

- **Throwaway emails matter.** Burst-rate-limit test fires 11 signup requests; if these aren't throwaway addresses, you flood the real magic-link queue with junk.
- **Latency budget is informational.** p95 < 500ms is a "yellow flag", not a deploy block — network conditions vary. Emit warning but don't fail unless p95 > 2000ms.
- **TLS cert check** — use `openssl s_client -showcerts < /dev/null 2>/dev/null | openssl x509 -noout -dates`. Don't use curl's built-in cert validation; it's hard to extract dates from.
- **No write operations.** Smoke is read-only against a production-ish deploy. The signup endpoint test uses a path that's idempotent (creates a magic-link token; doesn't actually onboard a tenant).
- **CORS preflight scenario** — the OPTIONS response must include `Access-Control-Allow-Origin: app.skycot.com` (or `*` for now); regression catch if a security middleware over-tightens.
