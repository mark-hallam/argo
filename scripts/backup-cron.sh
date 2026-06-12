#!/usr/bin/env bash
# backup-cron.sh — scheduled, offsite-replicated SQLite backup for a
# production argo gateway. The runnable counterpart of the
# launch-backup-cron-s3-offsite-and-restore-drill brief.
#
# Flow (each invocation):
#   [1/5] Locate the live data dir and assert it actually holds a db.
#   [2/5] `argo backup create --json` — a consistent tar.gz snapshot of
#         $ARGO_WORKSPACE/.argo (sessions.db + WAL/SHM if present +
#         bootstrap token; credentials only with --include-credentials).
#   [3/5] `argo backup verify` — re-hash the tarball against its manifest
#         so a corrupt write is caught BEFORE it is the only copy.
#   [4/5] `aws s3 cp` the verified tarball to s3://$S3_BUCKET/$S3_PREFIX/.
#         This is the offsite copy — a lost EC2 volume no longer loses
#         every customer's data.
#   [5/5] Prune local tarballs older than $RETENTION_DAYS (S3 lifecycle
#         rules handle remote retention; see AWS_DEPLOY_RUNBOOK.md).
#
# WHY THE .argo CONVENTION MATTERS (read before deploying):
#   `argo backup create` snapshots `<cwd>/.argo/`. The gateway stores its
#   db at `$ARGO_HOME/sessions.db`. These align ONLY when
#   `ARGO_HOME = $ARGO_WORKSPACE/.argo`. If your gateway runs with
#   `ARGO_HOME=/data` (db at /data/sessions.db, no .argo subdir) this
#   script would otherwise snapshot an EMPTY dir and ship a useless
#   backup. Step [1/5] FAILS LOUD in that case rather than silently
#   archiving nothing — that is the whole point of a P1 backup. The
#   one-time migration is printed on failure.
#
# Consistency under concurrent writes: SQLite runs in WAL mode
# (crates/argo-store/src/wal_tuning.rs). `argo backup create` captures
# sessions.db together with its -wal/-shm sidecars, so the restored set
# is crash-consistent — SQLite replays the WAL on next open exactly as
# it would after a power loss. scripts/backup-consistency-test.sh proves
# this end-to-end against a live, actively-writing gateway.
#
# Environment (all overridable):
#   S3_BUCKET           (required) target bucket, e.g. skycot-prod-backups
#   S3_PREFIX           key prefix          (default: argo-backups)
#   ARGO_WORKSPACE      dir whose .argo/ holds the db  (default: /var/lib/argo)
#   ARGO_BIN            path to the argo binary         (default: argo on PATH)
#   RETENTION_DAYS      prune local tarballs older than (default: 14)
#   INCLUDE_CREDENTIALS set to 1 to back up credentials.toml (default: 0)
#   AWS_CLI             aws cli path                    (default: aws on PATH)
#   DRY_RUN             set to 1 to skip the S3 upload   (default: 0)
#
# Exit codes: 0 ok; 1 misconfiguration / empty-db guard; 2 backup or
# verify failure; 3 S3 upload failure. Always emits one JSON summary
# line on stdout (truth-bounded: ok=true only when the offsite copy
# landed and verified).
set -euo pipefail

S3_PREFIX="${S3_PREFIX:-argo-backups}"
ARGO_WORKSPACE="${ARGO_WORKSPACE:-/var/lib/argo}"
ARGO_BIN="${ARGO_BIN:-argo}"
RETENTION_DAYS="${RETENTION_DAYS:-14}"
INCLUDE_CREDENTIALS="${INCLUDE_CREDENTIALS:-0}"
AWS_CLI="${AWS_CLI:-aws}"
DRY_RUN="${DRY_RUN:-0}"

die() {
  # $1 exit code, $2 reason
  printf '{"ok":false,"stage":"%s","error":"%s"}\n' "${stage:-init}" "$2"
  exit "$1"
}

stage="config"
[ -n "${S3_BUCKET:-}" ] || die 1 "S3_BUCKET is required"

stage="locate-db"
data_dir="${ARGO_WORKSPACE%/}/.argo"
db_file="${data_dir}/sessions.db"
if [ ! -s "$db_file" ]; then
  die 1 "no db at ${db_file}; gateway must run with ARGO_HOME=${ARGO_WORKSPACE%/}/.argo (one-time migration: mkdir -p ${data_dir} && mv ${ARGO_WORKSPACE%/}/sessions.db ${ARGO_WORKSPACE%/}/.bootstrap-token ${data_dir}/ 2>/dev/null; restart gateway)"
fi

stage="backup-create"
if [ "$INCLUDE_CREDENTIALS" = "1" ]; then
  create_json="$(cd "$ARGO_WORKSPACE" && "$ARGO_BIN" backup create --json --include-credentials)" \
    || die 2 "argo backup create failed"
else
  create_json="$(cd "$ARGO_WORKSPACE" && "$ARGO_BIN" backup create --json)" \
    || die 2 "argo backup create failed"
fi
tarball="$(printf '%s' "$create_json" | sed -n 's/.*"path":"\([^"]*\)".*/\1/p')"
[ -n "$tarball" ] && [ -s "$tarball" ] || die 2 "backup tarball path not found in: ${create_json}"

stage="backup-verify"
"$ARGO_BIN" backup verify "$tarball" --json >/dev/null || die 2 "argo backup verify failed for ${tarball}"

stage="s3-upload"
key="${S3_PREFIX%/}/$(basename "$tarball")"
if [ "$DRY_RUN" = "1" ]; then
  uploaded="s3://${S3_BUCKET}/${key} (dry-run, not uploaded)"
else
  "$AWS_CLI" s3 cp "$tarball" "s3://${S3_BUCKET}/${key}" --only-show-errors \
    || die 3 "aws s3 cp failed to s3://${S3_BUCKET}/${key}"
  uploaded="s3://${S3_BUCKET}/${key}"
fi

stage="prune-local"
pruned=0
backups_dir="${data_dir}/backups"
if [ -d "$backups_dir" ]; then
  while IFS= read -r old; do
    rm -f "$old" && pruned=$((pruned + 1))
  done < <(find "$backups_dir" -name '*.tar.gz' -type f -mtime "+${RETENTION_DAYS}" 2>/dev/null)
fi

bytes="$(printf '%s' "$create_json" | sed -n 's/.*"bytes":\([0-9]*\).*/\1/p')"
printf '{"ok":true,"tarball":"%s","bytes":%s,"uploaded":"%s","pruned_local":%d,"retention_days":%s}\n' \
  "$tarball" "${bytes:-0}" "$uploaded" "$pruned" "$RETENTION_DAYS"
