#!/usr/bin/env bash
# gateway-entrypoint.sh — durable-DB wrapper for the production gateway (DEC-056, "B′").
#
# The gateway's SQLCipher-encrypted SQLite DB lives on the ECS task's EPHEMERAL
# local disk (WAL on — fastest, and the config SQLite is designed for). Local
# disk is wiped on every task replacement, so without this wrapper the DB resets
# and every account is lost (it did, during the 2026-06-16 go-live). Litestream —
# the usual SQLite→S3 streamer — CANNOT be used here: it must parse the WAL,
# which SQLCipher encrypts (litestream issue #177). So durability is achieved by
# periodic WHOLE-FILE snapshots (`argo backup create`, which copies the encrypted
# db + its WAL/SHM sidecars — no key needed, crash-consistent) shipped to S3, plus
# restore-on-boot.
#
# This wrapper, run as the container ENTRYPOINT (then `exec`s the real gateway):
#   1. RESTORE-ON-BOOT, FAIL-CLOSED — the load-bearing safety. If a snapshot
#      SHOULD exist but cannot be restored, REFUSE TO BOOT (exit non-zero) rather
#      than start against an empty DB and masquerade as a fresh install. ECS then
#      retries; an alarm fires. Booting empty is the exact failure we are killing.
#   2. BACKGROUND SNAPSHOT LOOP — every DB_SNAPSHOT_INTERVAL seconds, snapshot the
#      live local DB to S3 (reusing scripts/backup-cron.sh). The data-loss window
#      on a hard crash is therefore at most one interval.
#   3. exec the gateway (so it is PID 1 and receives SIGTERM for graceful stop).
#
# Disabled cleanly when DB_SNAPSHOT_S3_BUCKET is unset (dev / local / single-box):
# the wrapper just exec's the gateway, so the same image runs everywhere.
#
# Env (all optional except the bucket, which gates the whole feature):
#   DB_SNAPSHOT_S3_BUCKET   target bucket (e.g. skycot-prod-backups). Unset => OFF.
#   DB_SNAPSHOT_S3_PREFIX   key prefix for live snapshots   (default: gateway-live)
#   DB_SNAPSHOT_INTERVAL    seconds between snapshots        (default: 300)
#   ARGO_WORKSPACE          dir whose .argo/ holds the db    (default: /data)
#   ARGO_BIN / AWS_CLI      binaries                          (defaults on PATH)
# Exit codes: 70 = snapshot exists but restore failed (fail-closed);
#             71 = "already initialized" sentinel present but no snapshot listable
#                  (fail-closed — S3 misconfig / wrong bucket, never boot empty).
set -euo pipefail

WORKSPACE="${ARGO_WORKSPACE:-/data}"
DATA_DIR="${WORKSPACE%/}/.argo"
DB="${DATA_DIR}/sessions.db"
BUCKET="${DB_SNAPSHOT_S3_BUCKET:-}"
PREFIX="${DB_SNAPSHOT_S3_PREFIX:-gateway-live}"
INTERVAL="${DB_SNAPSHOT_INTERVAL:-300}"
SENTINEL_KEY="${PREFIX%/}/.initialized"
ARGO_BIN="${ARGO_BIN:-/usr/local/bin/argo}"
AWS_CLI="${AWS_CLI:-aws}"
BACKUP_CRON="${BACKUP_CRON:-/app/scripts/backup-cron.sh}"

log() { printf '{"src":"gateway-entrypoint","ts":"%s","level":"%s","msg":"%s"}\n' "$(date -u +%FT%TZ)" "${2:-info}" "$1"; }

# Feature OFF: no bucket => run the gateway directly (dev/local/single-box parity).
if [ -z "$BUCKET" ]; then
  log "DB_SNAPSHOT_S3_BUCKET unset — durable snapshot/restore DISABLED; starting gateway directly"
  exec "$ARGO_BIN" "$@"
fi

# ---------- 1. restore-on-boot (FAIL-CLOSED) ----------
if [ -s "$DB" ]; then
  log "local db present (${DB}) — same-task restart; local copy is authoritative, skipping restore"
else
  log "no local db — consulting s3://${BUCKET}/${PREFIX%/}/ for the latest snapshot"
  latest="$("$AWS_CLI" s3 ls "s3://${BUCKET}/${PREFIX%/}/" 2>/dev/null | awk '{print $NF}' | grep '\.tar\.gz$' | sort | tail -1 || true)"
  sentinel_exists=1
  "$AWS_CLI" s3api head-object --bucket "$BUCKET" --key "$SENTINEL_KEY" >/dev/null 2>&1 || sentinel_exists=0

  if [ -n "$latest" ]; then
    log "restoring latest snapshot: ${latest}"
    mkdir -p "$WORKSPACE"
    tmp="$(mktemp -d "${TMPDIR:-/tmp}/gw-restore.XXXXXX")"
    err="${tmp}/err.log"
    # `argo restore` REFUSES a non-empty target (ensure_fresh_workspace), and the
    # image pre-creates ${DATA_DIR} (Dockerfile `mkdir -p /data/.argo`), so we must
    # NOT restore straight into ${WORKSPACE}. Restore into a fresh STAGING dir (on
    # the same filesystem as ${DATA_DIR} for an atomic rename), then move the
    # restored .argo into place. Capture each step's stderr so a failure says WHY
    # (the prior >/dev/null hid the real cause — it cost a live drill to find).
    stage="$(mktemp -d "${WORKSPACE%/}/.restore-stage.XXXXXX")"
    if "$AWS_CLI" s3 cp "s3://${BUCKET}/${PREFIX%/}/${latest}" "${tmp}/snap.tar.gz" --only-show-errors 2>"$err" \
        && "$ARGO_BIN" backup verify "${tmp}/snap.tar.gz" --json >/dev/null 2>>"$err" \
        && ( cd "$stage" && "$ARGO_BIN" restore "${tmp}/snap.tar.gz" --json >/dev/null 2>>"$err" ) \
        && [ -s "${stage}/.argo/sessions.db" ]; then
      rm -rf "$DATA_DIR"
      mv "${stage}/.argo" "$DATA_DIR"
      # Strip RUNTIME/transient state that must NOT survive a restore:
      #  - gateway.pid: the gateway's PID lock. The snapshot captured the live
      #    gateway's pid (="1" — it's PID 1 in its container), and EVERY container's
      #    gateway is PID 1, so a restored lock always looks "alive" → the gateway
      #    refuses to boot ("already running", boot_serve.rs) → crash-loop. The live
      #    kill-task drill hit exactly this.
      #  - sessions.db-shm: SQLite's WAL shared-memory index — rebuilt from the
      #    db + -wal on open, so dropping it is safe (no data loss) and avoids any
      #    stale-mmap surprise. The committed data lives in sessions.db (+ -wal).
      rm -f "${DATA_DIR}/gateway.pid" "${DATA_DIR}/sessions.db-shm"
      rm -rf "$tmp" "$stage"
      log "restore OK — db present at ${DB} (stale runtime locks stripped)"
    else
      reason="$(tr '\n"' '  ' <"$err" 2>/dev/null | head -c 300)"
      rm -rf "$stage" 2>/dev/null || true
      log "snapshot ${latest} exists but download/verify/restore FAILED [${reason}] — refusing to boot empty (fail-closed); ECS will retry" "fatal"
      exit 70
    fi
  elif [ "$sentinel_exists" = "1" ]; then
    log "init-sentinel present (we have snapshotted before) but NO snapshot is listable — refusing to boot empty (fail-closed)" "fatal"
    exit 71
  else
    log "no snapshot and no init-sentinel — first-ever boot; starting with a fresh empty db (the only safe empty-boot)"
  fi
fi

# ---------- 2. background snapshot loop ----------
(
  # First successful snapshot writes the init-sentinel, which arms the
  # fail-closed guard above for all FUTURE boots.
  while true; do
    sleep "$INTERVAL"
    if S3_BUCKET="$BUCKET" S3_PREFIX="$PREFIX" ARGO_WORKSPACE="$WORKSPACE" \
       ARGO_BIN="$ARGO_BIN" AWS_CLI="$AWS_CLI" \
       bash "$BACKUP_CRON" >/tmp/last-snapshot.json 2>&1; then
      if ! "$AWS_CLI" s3api head-object --bucket "$BUCKET" --key "$SENTINEL_KEY" >/dev/null 2>&1; then
        printf 'initialized %s\n' "$(date -u +%FT%TZ)" \
          | "$AWS_CLI" s3 cp - "s3://${BUCKET}/${SENTINEL_KEY}" --only-show-errors || true
      fi
    else
      log "periodic snapshot FAILED (see /tmp/last-snapshot.json) — retrying next interval" "warn"
    fi
  done
) &

log "snapshot loop armed (interval=${INTERVAL}s → s3://${BUCKET}/${PREFIX%/}/); starting gateway"

# ---------- 3. hand off to the gateway (PID 1, owns SIGTERM) ----------
exec "$ARGO_BIN" "$@"
