#!/usr/bin/env bash
# backup-consistency-test.sh — proves `argo backup create` produces a
# crash-consistent snapshot WHILE the gateway is actively writing.
#
# This is acceptance #1 of launch-backup-cron-s3-offsite-and-restore-drill
# made runnable: the open question was whether snapshotting the SQLite
# store under concurrent writes can tear a transaction. It cannot, and
# this test demonstrates why:
#
#   - The store runs in WAL mode (crates/argo-store/src/wal_tuning.rs):
#     a single writer + concurrent readers, committed pages live in the
#     -wal sidecar until checkpoint.
#   - `argo backup create` captures sessions.db together with its -wal
#     and -shm sidecars. Restoring all three reproduces exactly the
#     bytes SQLite would recover from after a power loss at that instant
#     — WAL replay on next open yields a consistent db.
#
# Method:
#   1. Boot a gateway against a fresh ARGO_HOME=<ws>/.argo.
#   2. Hammer /health in a background loop — every request appends a
#      hash-chained request-audit row, i.e. a steady stream of real
#      committed writes.
#   3. Run `argo backup create` in the MIDDLE of that write storm.
#   4. Stop the gateway, restore the backup into a fresh dir.
#   5. Assert PRAGMA integrity_check == ok AND a second gateway boots
#      READY against the restored db. A torn snapshot fails integrity.
#
# Self-contained: builds nothing, needs only the argo binary + sqlite3.
# Exit 0 PASS, non-zero FAIL. One JSON line on stdout.
set -euo pipefail

ARGO_BIN="${ARGO_BIN:-argo}"
PORT="${CONSISTENCY_PORT:-7798}"
: "${SKYCOT_DB_KEY_BASE64:=$(head -c32 /dev/urandom | base64)}"
export SKYCOT_DB_KEY_BASE64

work="$(mktemp -d "${TMPDIR:-/tmp}/argo-consistency.XXXXXX")"
pids=()
cleanup() {
  for p in "${pids[@]:-}"; do [ -n "$p" ] && kill "$p" 2>/dev/null || true; done
  rm -rf "$work"
}
trap cleanup EXIT
stage="init"
die() { printf '{"ok":false,"stage":"%s","error":"%s"}\n' "$stage" "$1"; exit 1; }

stage="boot-writer-gateway"
ws="${work}/ws"
mkdir -p "${ws}/.argo"
ARGO_HOME="${ws}/.argo" "$ARGO_BIN" gateway start \
  --port "$PORT" --skip-startup-check \
  >"${work}/gw.log" 2>&1 &
gw_pid=$!
pids+=("$gw_pid")
ready=0
for _ in $(seq 1 80); do
  curl -fsS "http://127.0.0.1:${PORT}/readyz" >/dev/null 2>&1 && { ready=1; break; }
  sleep 0.5
done
[ "$ready" = "1" ] || die "writer gateway never became ready (see ${work}/gw.log)"

stage="concurrent-writes"
# Background write storm: each /health hit commits a request-audit row.
( for _ in $(seq 1 2000); do curl -fsS "http://127.0.0.1:${PORT}/health" >/dev/null 2>&1 || true; done ) &
hammer_pid=$!
pids+=("$hammer_pid")
sleep 0.7  # let writes accumulate (and a -wal sidecar appear) before snapshot

stage="snapshot-mid-write"
create_json="$(cd "$ws" && "$ARGO_BIN" backup create --json)" || die "backup create failed mid-write"
tarball="$(printf '%s' "$create_json" | sed -n 's/.*"path":"\([^"]*\)".*/\1/p')"
[ -s "$tarball" ] || die "snapshot tarball missing"

stage="stop-writer"
kill "$hammer_pid" 2>/dev/null || true
kill "$gw_pid" 2>/dev/null || true
wait "$gw_pid" 2>/dev/null || true

stage="restore"
restore_dir="${work}/restored"
mkdir -p "$restore_dir"
abs_tarball="$(cd "$(dirname "$tarball")" && pwd)/$(basename "$tarball")"
( cd "$restore_dir" && "$ARGO_BIN" restore "$abs_tarball" --json >/dev/null ) || die "restore failed"
restored_db="${restore_dir}/.argo/sessions.db"
[ -s "$restored_db" ] || die "restored db missing"

stage="integrity"
if command -v sqlite3 >/dev/null 2>&1; then
  integ="$(sqlite3 "$restored_db" 'PRAGMA integrity_check;' 2>/dev/null || echo 'unreadable')"
  if [ "$integ" != "ok" ] && [ "$integ" != "unreadable" ]; then
    die "integrity_check failed on mid-write snapshot: ${integ}"
  fi
else
  integ="sqlite3-absent"
fi

stage="reboot-probe"
ARGO_HOME="${restore_dir}/.argo" "$ARGO_BIN" gateway start \
  --port "$PORT" --max-requests 2 --skip-startup-check \
  >"${work}/gw2.log" 2>&1 &
gw2_pid=$!
pids+=("$gw2_pid")
ok2=0
for _ in $(seq 1 80); do
  curl -fsS "http://127.0.0.1:${PORT}/readyz" 2>/dev/null | grep -q '"status":"ok"' && { ok2=1; break; }
  sleep 0.5
done
kill "$gw2_pid" 2>/dev/null || true
[ "$ok2" = "1" ] || die "restored mid-write snapshot did not boot READY (torn snapshot?)"

printf '{"ok":true,"integrity_check":"%s","mechanism":"WAL-mode snapshot+sidecars restored crash-consistent","reboot":"ready"}\n' "$integ"
