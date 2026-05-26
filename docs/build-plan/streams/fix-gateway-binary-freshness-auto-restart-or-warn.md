# fix-gateway-binary-freshness-auto-restart-or-warn

**Owner**: claude.
**Authored against**: origin/main `ddf44dfb4` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-gateway-binary-freshness-auto-restart-or-warn`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-2007/onboarding-hire-an-ai-teammate.md` F2 + F3 + whole-site-mvp-claims-coverage F11.

🟠 **P1 — The live gateway binary was 5.5h older than two load-bearing PRs at the time of audit (PR #2400 Genesis wire + PR #9ea693db1 workers-collection alias). Source-fixed code was not running. Investor demo would have hit pre-Genesis templated charter + 404 on `/api/v1/workers`. Operator must remember to restart the gateway after every relevant merge — this is a recurring foot-gun.**

## Root cause

The gateway is a long-lived dev process (5h45m uptime at audit time). It does not auto-rebuild or auto-restart when the source code changes. Operators (and AI agents) routinely forget to run `./scripts/restart-live-gateway.sh` after merging gateway-touching PRs. There is no visible signal from the gateway itself ("you're running build SHA X, current HEAD is SHA Y, you're N commits behind"). The freshness gate that exists (`scripts/check-gateway-freshness.sh`) only checks schema version, not binary age vs HEAD.

The audit flagged this as a P0 (Onboarding F2) because for cold demo recording, the operator hits the pre-Genesis code path without realising it.

Two leverage points:

1. **Gateway self-reports its build commit on `/health`**, and a periodic check warns when N commits behind HEAD.
2. **The dev launch script auto-detects gateway staleness** and offers to restart.

## Findings this brief closes

- onboarding-hire-an-ai-teammate F2 [P0] — Live gateway binary predates PR #2400 (Genesis wire); demo records pre-Genesis charter
- onboarding-hire-an-ai-teammate F3 [P0] — Live gateway binary predates PR #9ea693db1 (workers-collection alias); `/api/v1/workers` 404s
- whole-site-mvp-claims-coverage F11 [P1] — Running gateway binary stale vs HEAD; collection alias not live

## Fix recipe

1. **Embed the gateway's build commit SHA into the binary at compile time** via a build script:
   - `crates/argo-cli/build.rs`: shell out to `git rev-parse HEAD` and emit `cargo:rustc-env=ARGO_BUILD_COMMIT=...`
   - In the gateway `main.rs` or `/health` handler, read `env!("ARGO_BUILD_COMMIT")` and include it in the `/health` JSON: `{ "ok": true, "schema_version": 83, "build_commit": "ddf44dfb4", "uptime_seconds": ... }`

2. **Extend `scripts/check-gateway-freshness.sh`** to compare gateway build_commit against current `git rev-parse origin/main`:
   - If gateway is N commits behind, print a warning with the count + the first 3 commit subjects
   - If N > 50 OR any of the recent commits touched `crates/argo-cli/src/handlers/gateway/`, escalate the warning to a failure (exit code 1)
   - Output is grep-able for CI / scheduled-task conductor

3. **Add a `make gateway-warn-if-stale` target** that runs the check and prints a single-line operator-actionable warning at the top of every developer session:
   ```
   ⚠️  gateway running build abc1234 (5h45m old, 12 commits behind origin/main; 3 touch gateway code).
   ⚠️  Restart: ./scripts/restart-live-gateway.sh
   ```

4. **Optionally wire into the conductor tick** so the scheduled-task conductor refuses to recommend "record the demo" if the gateway is gateway-touching-code-stale.

5. **Documentation**: update `docs/skycot/operator/migrations.md` (or a new `gateway-freshness.md`) explaining the freshness contract and operator's responsibility.

## Acceptance criteria

- [ ] `curl http://127.0.0.1:7400/health` returns a JSON with a `build_commit` field
- [ ] `scripts/check-gateway-freshness.sh` exits non-zero when gateway is >5 commits behind on gateway-touching code
- [ ] `make gateway-warn-if-stale` prints actionable copy to terminal
- [ ] A scheduled-tick conductor heartbeat captures `gateway_build_age=N commits behind` and surfaces it in SHARED.md when N is non-trivial

## May-touch

- `crates/argo-cli/build.rs` (NEW or extended)
- `crates/argo-cli/src/handlers/gateway/health.rs`
- `scripts/check-gateway-freshness.sh`
- `Makefile` (add `gateway-warn-if-stale` target)
- `docs/skycot/operator/migrations.md` or `docs/skycot/operator/gateway-freshness.md` (NEW)

## Out of scope

- Auto-restarting the gateway (operator decision; would disrupt long-running work)
- Hot-reloading the binary (Rust doesn't support that out of the box)
- The existing schema-version check (already shipped; extend it, don't replace)

## Effort

~150-250 LOC across build.rs + handler + script + Makefile + docs. ~3-5 hours. P1 because the foot-gun has bitten the demo recording at least twice (per ledger history + this audit) and will keep recurring.
