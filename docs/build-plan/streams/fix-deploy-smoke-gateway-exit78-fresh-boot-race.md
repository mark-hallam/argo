# fix-deploy-smoke-gateway-exit78-fresh-boot-race

**Owner**: claude or codex — **P2 RELIABILITY (priority #1 domain).**
**Depends on**: #2862 (deploy-smoke first-green) — landed the bootable image this builds on.
**Authored against**: origin/main `abb020896` on `2026-06-12`.
**Validate before claim**: bash scripts/check-brief-freshness.sh fix-deploy-smoke-gateway-exit78-fresh-boot-race

## Problem (observed on the deploy-smoke first-green run 2026-06-12)

With the SQLCipher/libdbus/key fixes from #2862 the gateway boots and serves,
but it **intermittently `exit 78` (EX_CONFIG) on the FIRST fresh-volume
`docker compose up -d`** after a `down -v`, then comes up Healthy on a
subsequent boot. Observed ~50% across runs:
- scripted `run.sh` (fresh `down -v` → `up -d` of postgres+gateway+web): two
  separate runs each `exit 78` on first gateway boot.
- manual `up -d postgres gateway` (no web) + the eventual scripted retry:
  booted clean (`startup_check ok in 9 ms`, `listening on 0.0.0.0:8080`).

A non-deterministic boot failure on a fresh deploy is a reliability defect
(priority #1) — a real customer's first `docker compose up` could land on the
failing side.

## Likely cause (to confirm, not assume)

`exit 78` is a `startup_check` config failure. The clean boots logged
`startup_check ok`, so the check *passes* once something is ready. Leading
hypotheses, in order:
1. **A startup_check races a dependency's first-init.** `depends_on:
   condition: service_healthy` gates on postgres's *healthcheck*, but the
   gateway may probe something (a DB/connection/migration) that isn't ready
   at the exact first-boot instant. Note the compose mismatch worth checking:
   postgres creates `POSTGRES_DB: argo` while `.env.smoke` points
   `DATABASE_URL` at `argo_smoke` — confirm whether the gateway connects to
   postgres at boot at all (it serves /health off the SQLite store), and if
   so, fix the DB-name mismatch.
2. **First-boot /data SQLCipher store init** races the encrypted-store open
   on a brand-new volume.

## Fix

1. Reproduce deterministically: loop `down -v` → `up -d` and capture the
   `exit 78` gateway log (the failing `startup_check` line names the check).
2. Root-cause the specific check; fix the actual ordering/readiness gap
   (e.g. correct the postgres DB name, add the missing readiness wait, or
   make the check retry with bounded backoff instead of fail-fast at boot).
3. Do NOT paper over it with a blind `restart: on-failure` — identify why the
   check fails and close that, per DEC-029 (a green-on-retry is not a fix).

## Acceptance
- 10 consecutive `down -v` → `up -d` cycles boot the gateway Healthy with
  zero `exit 78` (a checked-in loop or a documented manual run with evidence).
- `deploy/smoke/LAST-PASS.md` still green; checks.ts 4/4.

~40-120 LOC (mostly compose/env/config + maybe a small readiness gate).
**🟡 P2 — reliability hardening before the gated-cohort deploy goes wide.**
