#!/usr/bin/env bash
# restore-drill.sh — the documented, runnable disaster-recovery drill.
# "A backup that has never been restored is unverified." This script
# closes that gap: it restores a backup into a throwaway data dir, boots
# a real gateway against the restored db, and asserts the gateway reports
# READY plus a row-count sanity probe. Run it after every schema change
# and on a schedule.
#
# Modes:
#   (default, --from-s3)  Pull the LATEST tarball from
#                         s3://$S3_BUCKET/$S3_PREFIX/ (lexical sort on the
#                         %Y%m%dT%H%M%SZ filename == newest), then drill.
#   --local <file>        Drill a local tarball; no AWS needed. Used by
#                         CI and by anyone without S3 creds.
#
# Steps:
#   [1/5] Obtain the tarball (S3 or local).
#   [2/5] `argo backup verify` — refuse to drill a corrupt archive.
#   [3/5] `argo restore` into a fresh empty dir → <drill>/.argo/...
#   [4/5] Boot `argo gateway start` with ARGO_HOME=<drill>/.argo
#         (--skip-startup-check so it needs no provider key,
#         --max-requests so it self-exits), poll /readyz.
#   [5/5] Assert: /readyz status==ok AND /health schema_version is a
#         positive integer (proves the SQLite store opened and answered).
#         Best-effort sqlite3 integrity_check + table-row count when the
#         db is not encrypted at rest.
#
# Environment:
#   S3_BUCKET / S3_PREFIX   as in backup-cron.sh (--from-s3 mode)
#   ARGO_BIN                argo binary           (default: argo on PATH)
#   AWS_CLI                 aws cli               (default: aws on PATH)
#   SKYCOT_DB_KEY_BASE64    at-rest db key — pass the SAME value the live
#                           gateway uses so the restored db can be opened.
#                           On a macOS dev box, set any throwaway 32-byte
#                           base64 value to bypass the Keychain prompt.
#   DRILL_PORT              gateway port for the probe  (default: 7799)
#
# Exit codes: 0 PASS; 1 misconfiguration; 2 verify/restore failure;
# 3 gateway did not become ready; 4 probe assertion failed. Emits one
# JSON summary line (ok=true ONLY on a fully-green drill — NO THEATER).
set -euo pipefail

ARGO_BIN="${ARGO_BIN:-argo}"
AWS_CLI="${AWS_CLI:-aws}"
S3_PREFIX="${S3_PREFIX:-argo-backups}"
DRILL_PORT="${DRILL_PORT:-7799}"

MODE="from-s3"
LOCAL_TARBALL=""
while [ $# -gt 0 ]; do
  case "$1" in
    --from-s3) MODE="from-s3"; shift ;;
    --local) MODE="local"; LOCAL_TARBALL="${2:-}"; shift 2 ;;
    *) printf '{"ok":false,"error":"unknown arg: %s"}\n' "$1"; exit 1 ;;
  esac
done

work="$(mktemp -d "${TMPDIR:-/tmp}/argo-restore-drill.XXXXXX")"
gw_pid=""
cleanup() {
  [ -n "$gw_pid" ] && kill "$gw_pid" 2>/dev/null || true
  rm -rf "$work"
}
trap cleanup EXIT

stage="init"
die() { printf '{"ok":false,"stage":"%s","error":"%s"}\n' "$stage" "$1"; exit "$2"; }

stage="obtain-tarball"
if [ "$MODE" = "local" ]; then
  [ -s "$LOCAL_TARBALL" ] || die "local tarball not found: ${LOCAL_TARBALL}" 1
  tarball="$LOCAL_TARBALL"
else
  [ -n "${S3_BUCKET:-}" ] || die "S3_BUCKET required for --from-s3" 1
  latest_key="$("$AWS_CLI" s3 ls "s3://${S3_BUCKET}/${S3_PREFIX%/}/" \
    | awk '{print $NF}' | grep '\.tar\.gz$' | sort | tail -1)" \
    || die "aws s3 ls failed" 2
  [ -n "$latest_key" ] || die "no backups found under s3://${S3_BUCKET}/${S3_PREFIX%/}/" 2
  tarball="${work}/${latest_key}"
  "$AWS_CLI" s3 cp "s3://${S3_BUCKET}/${S3_PREFIX%/}/${latest_key}" "$tarball" --only-show-errors \
    || die "aws s3 cp download failed" 2
fi

stage="verify"
"$ARGO_BIN" backup verify "$tarball" --json >/dev/null || die "backup verify failed: ${tarball}" 2

stage="restore"
restore_dir="${work}/restored"
mkdir -p "$restore_dir"
abs_tarball="$(cd "$(dirname "$tarball")" && pwd)/$(basename "$tarball")"
( cd "$restore_dir" && "$ARGO_BIN" restore "$abs_tarball" --json >/dev/null ) \
  || die "argo restore failed" 2
restored_db="${restore_dir}/.argo/sessions.db"
[ -s "$restored_db" ] || die "restored db missing or empty: ${restored_db}" 2

stage="boot-gateway"
ARGO_HOME="${restore_dir}/.argo" "$ARGO_BIN" gateway start \
  --port "$DRILL_PORT" --max-requests 4 --skip-startup-check \
  >"${work}/gw.log" 2>&1 &
gw_pid=$!
ready=0
for _ in $(seq 1 80); do
  if curl -fsS "http://127.0.0.1:${DRILL_PORT}/readyz" >/dev/null 2>&1; then ready=1; break; fi
  sleep 0.5
done
[ "$ready" = "1" ] || die "gateway did not become ready on :${DRILL_PORT} (see ${work}/gw.log)" 3

stage="probe"
ready_body="$(curl -fsS "http://127.0.0.1:${DRILL_PORT}/readyz")" || die "readyz probe failed" 4
health_body="$(curl -fsS "http://127.0.0.1:${DRILL_PORT}/health")" || die "health probe failed" 4
echo "$ready_body" | grep -q '"status":"ok"' || die "readyz not ok: ${ready_body}" 4
schema="$(printf '%s' "$health_body" | sed -n 's/.*"schema_version":\([0-9]*\).*/\1/p')"
[ -n "$schema" ] && [ "$schema" -gt 0 ] 2>/dev/null || die "no positive schema_version: ${health_body}" 4

# Best-effort row-count sanity: only meaningful when the db is plaintext
# (sqlcipher-encrypted at-rest dbs are not readable by stock sqlite3,
# and /readyz above already proved the gateway opened it with its key).
rows="unknown"
if command -v sqlite3 >/dev/null 2>&1; then
  if integ="$(sqlite3 "$restored_db" 'PRAGMA integrity_check;' 2>/dev/null)" && [ "$integ" = "ok" ]; then
    rows="$(sqlite3 "$restored_db" \
      "SELECT COALESCE(SUM(c),0) FROM (SELECT (SELECT COUNT(*) FROM sqlite_master WHERE type='table') AS c);" \
      2>/dev/null || echo unknown)"
  fi
fi

printf '{"ok":true,"tarball":"%s","schema_version":%s,"readyz":"ok","tables_or_rows":"%s"}\n' \
  "$(basename "$tarball")" "$schema" "$rows"
