# DEC-056 — Durable gateway DB: local-disk + snapshot-to-S3 + fail-closed restore-on-boot ("B′")

**Date**: 2026-06-17 · **Status**: active · **Operator-locked**: yes ("B best long term performance")

## Decision

The production gateway's SQLCipher-encrypted SQLite database stays on the ECS
task's **fast local (ephemeral) disk with WAL mode ON** — the configuration
SQLite is designed for and the one that preserves write performance. Durability
across task replacement is provided by **periodic whole-file snapshots to S3 plus
fail-closed restore-on-boot**, NOT by moving the DB onto network storage.

Concretely:
- DB remains at `/data/.argo/sessions.db` on local disk (`gateway_db_on_efs=false`
  stays). No journal-mode change.
- The gateway container ENTRYPOINT becomes `scripts/gateway-entrypoint.sh`, which
  (1) restores the latest S3 snapshot before boot **fail-closed** (if a snapshot
  *should* exist but can't be restored, it exits non-zero rather than boot empty),
  (2) runs a background loop that snapshots the live DB to S3 every
  `DB_SNAPSHOT_INTERVAL` seconds (default 300) via the existing
  `scripts/backup-cron.sh` (`argo backup create` → verify → `aws s3 cp`), then
  (3) `exec`s the gateway.
- The gateway task IAM role gains S3 (List/Get/Put on the backups bucket) + the
  bucket's KMS data-key ops — scoped to that one bucket ARN, no wildcards.

## Context / why B′

The go-live set `gateway_db_on_efs=false` because SQLite **WAL crash-loops on
EFS/NFS** (WAL needs a `-shm` mmap that NFS can't provide). Local disk fixed the
crash but is ephemeral → the DB resets on every task restart, losing all accounts
(proven during the 2026-06-16 go-live; we had to re-pay to re-test). A
design-workflow scored three options:

- **A — EFS + rollback journal_mode.** Durable + simplest, but pays a network
  round-trip + fsync on every commit; with frequent hash-chained audit writes
  that's a permanent, worsening write tax. Rejected for the perf ceiling.
- **B — Litestream → S3 (continuous WAL streaming).** The workflow's first pick,
  but **eliminated on verification**: Litestream must parse the WAL, which
  SQLCipher encrypts — it does not support encrypted DBs (litestream issue #177,
  open since 2021). We cannot drop SQLCipher (the prod boot gate refuses to start
  without it). The workflow's confident "it streams the encrypted bytes" claim
  was wrong; the web-research sub-agent had failed (529) so the gap went unchecked
  until re-verified.
- **C — Fargate EBS volume.** Fast, but a single disk in a single availability
  zone (still needs an S3 backup on top — i.e. B′'s work anyway) plus
  attach/detach-race fragility on every restart. Dominated.

**B′** is the SQLCipher-compatible realization of B's principle (data local + fast,
durability moved off the hot path) using whole-file snapshots (which work with
SQLCipher — they copy the encrypted bytes, no key needed) instead of WAL
streaming. It keeps best-in-class write performance and converges on SQLite's
designed configuration; the cost is a bounded data-loss window of one snapshot
interval on a hard crash (vs today's "lose everything"), tunable down.

## Critical finding (drove the design)

The existing daily backup ECS task (`modules/backup/schedule.tf`) mounts the
gateway's **EFS** access point to read the DB. With `gateway_db_on_efs=false` the
live DB is on **local disk**, invisible to a separate task — so the daily backup
has been silently snapshotting an empty EFS volume. We currently have **neither
durability nor a working backup.** Therefore the snapshot must run **inside the
gateway container** (only it can see the local-disk DB); the separate EFS-based
daily backup task is superseded and will be disabled.

## Consequences / risks

- **The one risk that matters: booting empty after a restore failure** — the very
  symptom we're killing. Neutralised by the fail-closed boot (exit non-zero unless
  a real DB is in place when an init-sentinel says one should be) + a CloudWatch
  alarm on `RunningTaskCount < 1` so a fail-closed crash-loop pages, never wipes
  silently. An S3 `gateway-live/.initialized` sentinel distinguishes the only safe
  empty-boot (first-ever) from "S3 unreachable / wrong bucket".
- Data-loss window = one `DB_SNAPSHOT_INTERVAL` (default 5 min) on a hard crash.
  Planned deploys can snapshot-before-stop to approach zero (refinement).
- Verified by `scripts/restore-drill.sh` (restore → boot → `/readyz` ok →
  `schema_version > 0`) run in CI + the live kill-the-task durability test.
- Scale-up path if snapshot I/O ever dominates: shorter interval, or revisit once
  multi-node makes S3-backed replication the substrate anyway.

## Links

- Eliminated option: [Litestream SQLCipher limitation #177](https://github.com/benbjohnson/litestream/issues/177)
- Builds on: `scripts/backup-cron.sh`, `scripts/restore-drill.sh`, `argo backup create/verify/restore`.
- Supersedes the local-disk "ephemeral, durable fix tracked" note from the go-live (spawned task_dbc79d4c).
