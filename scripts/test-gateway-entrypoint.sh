#!/usr/bin/env bash
# test-gateway-entrypoint.sh — unit test for the DEC-056 durable-DB entrypoint
# wrapper (scripts/gateway-entrypoint.sh). Focuses on the LOAD-BEARING safety:
# the fail-closed "never boot empty" branches. Uses on-PATH stubs for `aws` and
# `argo` so it needs no AWS and no real binary; the happy restore path is proven
# for real by the live kill-task durability drill (DEC-056).
#
# Asserts:
#   1. snapshot exists but restore FAILS            -> exit 70 (fail-closed)
#   2. init-sentinel present but no snapshot listed -> exit 71 (fail-closed)
#   3. no bucket configured                         -> passes through to gateway
#   4. first-ever boot (no snapshot, no sentinel)   -> passes through (empty OK)
#   5. local db already present                     -> skips restore, passes through
# Exit 0 = all pass; 1 = a failure (prints which).
set -uo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
WRAPPER="${HERE}/gateway-entrypoint.sh"
[ -x "$WRAPPER" ] || { echo "FAIL: wrapper not executable at ${WRAPPER}"; exit 1; }

work="$(mktemp -d "${TMPDIR:-/tmp}/gw-entrypoint-test.XXXXXX")"
cleanup() { pkill -f 'sleep 4242' 2>/dev/null || true; rm -rf "$work"; }
trap cleanup EXIT

# --- stub bin/ ---
mkdir -p "${work}/bin"

cat > "${work}/bin/aws" <<'AWS'
#!/usr/bin/env bash
# Stubbed aws. Behaviour driven by env: FAKE_SNAPSHOT, FAKE_SENTINEL, FAKE_DL_OK.
case "$1 $2" in
  "s3 ls")
    if [ "${FAKE_SNAPSHOT:-0}" = "1" ]; then echo "2026-06-17 00:00:00  123  snap-20260617T000000Z.tar.gz"; fi
    exit 0 ;;
  "s3api head-object")
    [ "${FAKE_SENTINEL:-0}" = "1" ] && exit 0 || exit 255 ;;
  "s3 cp")
    # args: s3 cp <src> <dst> ...   create dst when the download is meant to succeed
    if [ "${FAKE_DL_OK:-1}" = "1" ]; then dst="$4"; [ -n "$dst" ] && : > "$dst"; exit 0; fi
    exit 1 ;;
  *) exit 0 ;;
esac
AWS

cat > "${work}/bin/argo" <<'ARGO'
#!/usr/bin/env bash
# Stubbed argo. backup verify -> FAKE_VERIFY_OK; restore -> FAKE_RESTORE_OK (also
# creates the restored db when ok); gateway start -> print the exec marker.
case "$1 $2" in
  "backup verify") [ "${FAKE_VERIFY_OK:-1}" = "1" ] && exit 0 || exit 1 ;;
  restore*)
    if [ "${FAKE_RESTORE_OK:-1}" = "1" ]; then
      # Real `argo restore` writes <cwd>/.argo/... and REFUSES a non-empty cwd
      # (ensure_fresh_workspace). The wrapper now restores into a fresh STAGING
      # dir (cd'd into here) — so write .argo in the CWD, and FAIL if the cwd is
      # non-empty (mirrors ensure_fresh_workspace, so a regression to restoring
      # into the pre-created /data/.argo would be caught). Non-empty content
      # because the wrapper requires `[ -s db ]`.
      if [ -n "$(ls -A . 2>/dev/null)" ]; then echo "restore target is not empty: $PWD" >&2; exit 1; fi
      mkdir -p ".argo" && printf 'stub-sqlite-db' > ".argo/sessions.db"
      # Simulate the runtime/transient files a real snapshot captures from a LIVE
      # gateway — the wrapper must strip these on restore (gateway.pid="1" else the
      # restored gateway refuses to boot "already running"; -shm is rebuildable).
      echo 1 > ".argo/gateway.pid"; : > ".argo/sessions.db-shm"
      exit 0
    fi
    exit 1 ;;
  "gateway start") echo "EXEC_GATEWAY_MARKER"; exit 0 ;;
  *) exit 0 ;;
esac
ARGO
chmod +x "${work}/bin/aws" "${work}/bin/argo"

PASS=0; FAIL=0
run_case() {
  # $1 name; $2 expected_exit; $3 expect_marker(yes/no); rest = env assignments
  local name="$1" exp_exit="$2" exp_marker="$3"; shift 3
  local ws; ws="$(mktemp -d "${work}/ws.XXXXXX")"
  # Simulate the production image: the Dockerfile pre-creates ${WORKSPACE}/.argo
  # (RUN mkdir -p /data/.argo). This is what made the live drill's restore fail —
  # `argo restore` into a non-empty target is refused — so the test env must match.
  mkdir -p "${ws}/.argo"
  local out rc outfile="${ws}/out.log"
  # Redirect to a FILE, never $(...): the wrapper backgrounds a snapshot loop that
  # inherits stdout and sleeps for the interval, so command-substitution would
  # block until that sleep ends. With a file, the foreground command returns as
  # soon as the exec'd gateway stub exits; the orphaned sleep is reaped by the trap.
  env -i PATH="${work}/bin:/usr/bin:/bin" \
    ARGO_WORKSPACE="$ws" ARGO_BIN="${work}/bin/argo" AWS_CLI="${work}/bin/aws" \
    BACKUP_CRON="/bin/true" DB_SNAPSHOT_INTERVAL=4242 \
    "$@" \
    bash "$WRAPPER" gateway start --port 8080 >"$outfile" 2>&1
  rc=$?
  out="$(cat "$outfile" 2>/dev/null)"
  local ok=1
  [ "$rc" = "$exp_exit" ] || { ok=0; echo "  FAIL[$name]: exit $rc, expected $exp_exit"; echo "$out" | sed 's/^/      /'; }
  if [ "$exp_marker" = "yes" ]; then echo "$out" | grep -q EXEC_GATEWAY_MARKER || { ok=0; echo "  FAIL[$name]: gateway exec marker missing"; }; fi
  if [ "$exp_marker" = "no" ]; then echo "$out" | grep -q EXEC_GATEWAY_MARKER && { ok=0; echo "  FAIL[$name]: gateway was exec'd but should NOT have been (would boot empty!)"; }; fi
  if [ "$ok" = "1" ]; then PASS=$((PASS+1)); echo "  ok: $name"; else FAIL=$((FAIL+1)); fi
}

echo "== gateway-entrypoint fail-closed tests =="
# 1. snapshot exists but restore fails -> exit 70, must NOT exec the gateway.
run_case "snapshot-exists-restore-fails-exits-70" 70 no \
  DB_SNAPSHOT_S3_BUCKET=b FAKE_SNAPSHOT=1 FAKE_RESTORE_OK=0
# 2. sentinel says we snapshotted before, but no snapshot listed -> exit 71, no exec.
run_case "sentinel-but-no-snapshot-exits-71" 71 no \
  DB_SNAPSHOT_S3_BUCKET=b FAKE_SNAPSHOT=0 FAKE_SENTINEL=1
# 3. no bucket -> feature off, pass straight through to the gateway.
run_case "no-bucket-passes-through" 0 yes
# 4. first-ever boot (no snapshot, no sentinel) -> the ONLY safe empty-boot.
run_case "first-boot-no-snapshot-no-sentinel-passes-through" 0 yes \
  DB_SNAPSHOT_S3_BUCKET=b FAKE_SNAPSHOT=0 FAKE_SENTINEL=0
# 5. snapshot exists and restore succeeds -> restores then passes through.
run_case "snapshot-exists-restore-ok-passes-through" 0 yes \
  DB_SNAPSHOT_S3_BUCKET=b FAKE_SNAPSHOT=1 FAKE_RESTORE_OK=1

# 6. restore strips the stale runtime lock — the bug the live drill found: a
# restored gateway.pid (="1") makes the gateway refuse to boot "already running".
ws6="$(mktemp -d "${work}/ws6.XXXXXX")"; mkdir -p "${ws6}/.argo"
env -i PATH="${work}/bin:/usr/bin:/bin" ARGO_WORKSPACE="$ws6" ARGO_BIN="${work}/bin/argo" \
  AWS_CLI="${work}/bin/aws" BACKUP_CRON="/bin/true" DB_SNAPSHOT_INTERVAL=4242 \
  DB_SNAPSHOT_S3_BUCKET=b FAKE_SNAPSHOT=1 FAKE_RESTORE_OK=1 \
  bash "$WRAPPER" gateway start >"${ws6}/out" 2>&1 || true
if [ -s "${ws6}/.argo/sessions.db" ] && [ ! -e "${ws6}/.argo/gateway.pid" ] && [ ! -e "${ws6}/.argo/sessions.db-shm" ]; then
  PASS=$((PASS+1)); echo "  ok: restore-strips-stale-gateway.pid-and-shm"
else
  FAIL=$((FAIL+1)); echo "  FAIL: stale gateway.pid / -shm NOT stripped after restore (would crash-loop 'already running')"
fi

echo "== ${PASS} passed, ${FAIL} failed =="
[ "$FAIL" = "0" ]
