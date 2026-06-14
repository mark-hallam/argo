# fix-backup-image-bake-scripts-awscli-and-argo-home

**Authored against:** origin/main `7f98b30cf` on 2026-06-15
**Validate before claim:** `bash scripts/check-brief-freshness.sh fix-backup-image-bake-scripts-awscli-and-argo-home`
**Priority:** 🔴 P1 / HIGH — go-live DR gap. The AUTOMATED daily offsite backup does NOT
run on a fresh apply. A manual workaround exists (run backup-cron.sh from an EFS-mounting
host), so it is NOT a hard apply-blocker, but close it before a broad paying launch.

## Problem (VERIFIED 2026-06-15 AWS go-live audit — confirmed against current main)

The offsite-backup module (`infra/terraform/modules/backup/`) provisions a daily
EventBridge → ECS RunTask (`schedule.tf:40` `command = ["scripts/backup-cron.sh"]`;
`schedule.tf:46-47` `ARGO_WORKSPACE=/data`, `ARGO_HOME=/data/.argo`) that runs
`scripts/backup-cron.sh` (exists, 111 lines; `aws s3 cp` at line 96) and uploads to S3.
The runtime image does NOT satisfy it — and the module's OWN docs flag this as a
never-closed follow-up:

- Root `Dockerfile` COPYs only manifests/verticals/policies/skill-library (lines 73-76)
  — **no `COPY scripts`**; apt (line 57) installs ca-certificates/libssl/libdbus only
  — **no AWS CLI**. `deploy/Dockerfile.gateway` (lines 54-60) has the same gaps.
- `modules/backup/README.md` "⚠️ Image prerequisite" + `schedule.tf:11-26` both state
  the scheduled RunTask "will fail at the script-not-found / aws-not-found step" until a
  follow-up bakes `scripts/` + `awscli` into the image (the Dockerfile was outside that
  brief's may-touch).
- **ARGO_HOME mismatch:** the gateway image defaults `ARGO_HOME=/data` (`Dockerfile:84`,
  `Dockerfile.gateway:60`) → db at `/data/sessions.db`. But backup-cron.sh snapshots
  `$ARGO_WORKSPACE/.argo` (`/data/.argo/sessions.db`) → the empty-db guard
  (`backup-cron.sh:73`) fails LOUD daily even once scripts + awscli are present.

Result: bucket/lifecycle/IAM are live-on-apply, but the daily AUTOMATED offsite backup
fails on every run on a fresh apply → no working automated DR.

## Fix
1. **Root `Dockerfile` runtime stage** (keep `deploy/Dockerfile.gateway` at PARITY):
   `COPY scripts ./scripts` + install **AWS CLI v2** in the runtime layer. Keep the image
   lean (multi-stage copy from `amazon/aws-cli`, or a slim install) — do NOT pull the
   full Rust toolchain into the runtime.
2. **Reconcile ARGO_HOME** (⚠️ BLAST RADIUS — changes the gateway's data dir; safe on a
   FRESH apply, needs a migration note for any existing deploy): set the gateway image
   `ARGO_HOME=/data/.argo` (the documented runbook §13b convention) so the gateway writes
   `/data/.argo/sessions.db` where the backup looks. Ensure `/data/.argo` is
   creatable + writable by the non-root uid 10001 (extend the existing `/data` chmod at
   `Dockerfile:90`). Verify the gateway still BOOTS and finds its db with the new ARGO_HOME.

## Verifier (RUN it — NO THEATER)
- `docker build -f Dockerfile .` AND `docker build -f deploy/Dockerfile.gateway .` succeed.
- `docker run --entrypoint /bin/bash <img> -lc 'ls scripts/backup-cron.sh && aws --version'`
  → both succeed.
- Gateway boots cleanly with `ARGO_HOME=/data/.argo` (db created/found there; the
  fail-closed boot still passes with the DB key).
- (stretch) a local `scripts/backup-cron.sh` dry-run against a seeded `/data/.argo` db
  reaches the `aws s3 cp` step (mock/skip the upload) without tripping the empty-db guard.

## Out of scope
- Bloating the runtime image (AWS CLI must be a slim install). Migrating existing prod
  data (this is pre-go-live; document the migration for completeness only).

**LOC budget:** ~30–80 (Dockerfile COPY + apt/awscli layer + ARGO_HOME ENV + chmod, ×2 files).
