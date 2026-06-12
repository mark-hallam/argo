# launch-backup-cron-s3-offsite-and-restore-drill

**Owner**: claude or codex — data-durability P1 → conductor review before merge.
**Depends on**: none.
**Authored against**: origin/main `7274fb673` on `2026-06-12`.
**Validate before claim**: bash scripts/check-brief-freshness.sh launch-backup-cron-s3-offsite-and-restore-drill

## Problem (launch audit 2026-06-12)

`argo backup create/verify` is tested CLI but NEVER SCHEDULED anywhere:
DEPLOYMENT.md (the EC2 compose path) has zero backup steps; single EC2 +
SQLite on one volume with no automated offsite copy = one bad volume loses
every paying customer's data. Restore has never been executed once. Also
flagged: unknown whether `argo backup create` snapshots consistently while
the gateway writes (WAL/online-backup handling).

## Fix

(1) Verify/ensure `argo backup create` takes a consistent snapshot under
concurrent writes (SQLite online-backup API or WAL checkpoint — document
which; add a concurrent-write test). (2) `scripts/backup-cron.sh`: create →
verify → push to S3 (aws cli, bucket+prefix from env) → prune local; ship a
systemd timer unit + a compose sidecar variant; add the setup steps to
DEPLOYMENT.md + AWS_DEPLOY_RUNBOOK. (3) `scripts/restore-drill.sh`:
restore latest from S3 into a fresh data dir, boot a gateway against it,
assert /health/ready + a row-count sanity probe — the documented,
runnable drill. Replace/delete the stale pg_dump recipe the audit found.

## Acceptance
1. Concurrent-write backup test green (or the documented safe mechanism).
2. Drill script runs end-to-end against a local MinIO/S3-stub or real S3
   env in a test (seeded db → backup → restore → probe passes).
3. Docs updated in the same commit; clippy/shellcheck clean.

~150-250 LOC. **🔴 P1 — backups that have never restored are unverified.**
