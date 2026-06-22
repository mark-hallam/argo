#!/usr/bin/env bash
# overage-cron.sh — scheduled caller for DEC-053 Pro metered-overage billing.
#
# Wave-3 ③: the overage job (`argo billing report-overage`) + the 10% markup
# shipped (#2996/#2999), but nothing CALLED it on a schedule, so a paying
# customer's metered usage never billed unless an operator ran the CLI by hand.
# This script is the body of the daily EventBridge-scheduled ECS RunTask
# (infra/terraform/modules/overage/schedule.tf): it makes the live DB readable,
# then runs the idempotent report-overage command.
#
# WHY A RESTORE STEP. In production the gateway runs DEC-056 "B′" — its
# SQLCipher SQLite DB lives on the gateway TASK's ephemeral local disk and is
# snapshotted to S3 every interval (scripts/gateway-entrypoint.sh). A separate
# Fargate overage task has its OWN empty local disk and does NOT share the
# gateway's, so to read live tenant usage it must first restore the latest
# snapshot from the same S3 bucket/prefix the gateway snapshots to. This mirrors
# the gateway entrypoint's restore-on-boot (same `aws s3 cp` + `argo restore`
# pattern), but READ-ONLY: this task never snapshots back, never writes the
# init-sentinel, and a MISSING snapshot is NOT fatal (it just means no Pro
# tenant has any usage yet → nothing to bill → exit 0).
#
# When DB_SNAPSHOT_S3_BUCKET is unset (the DB is on EFS, or a dev/single-box
# run), the restore is skipped and the command runs directly against whatever DB
# is already at $ARGO_WORKSPACE/.argo (EFS mount or local file) — same script
# everywhere.
#
# ⚠️ DB-PATH BRIDGE (the one non-obvious bit). The rest of the platform writes
# its DB at `$ARGO_HOME/sessions.db` and snapshots `<cwd>/.argo/` — the canonical
# `.argo` convention. But the `argo billing report-overage` handler
# (crates/argo-cli/src/handlers/billing_overage.rs open_store()) hardcodes
# `<cwd>/.skycot/sessions.db` and does NOT honour ARGO_HOME — a real
# convention mismatch in that one handler. This is INFRA-ONLY work, so rather
# than touch Rust we BRIDGE it here: after restore (which yields
# `$ARGO_WORKSPACE/.argo/sessions.db`) we expose the same DB at
# `<cwd>/.skycot/sessions.db` and run the command from cwd=$ARGO_WORKSPACE. A
# follow-up should make billing_overage.rs use home.rs resolve_home/db_path like
# its sibling handlers (worker_budget.rs etc.) so this bridge can be deleted.
#
# IDEMPOTENCY (cron-safe). `argo billing report-overage` pushes each Pro
# tenant's CUMULATIVE period-to-date overage with Stripe `action=set` keyed on a
# stable (tenant_id, period_start) idempotency key (crates/argo-billing
# overage.rs). Re-running RE-ASSERTS the same totals rather than adding to them,
# so any cadence is safe — a missed run is caught by the next, a double run does
# not double-bill.
#
# Env:
#   DB_SNAPSHOT_S3_BUCKET   S3 bucket holding the gateway's live snapshots.
#                           Unset => skip restore (EFS / dev). Same var the
#                           gateway entrypoint uses, so they stay in lock-step.
#   DB_SNAPSHOT_S3_PREFIX   key prefix for live snapshots   (default: gateway-live)
#   ARGO_WORKSPACE          dir whose .argo/ holds the db   (default: /data)
#   STRIPE_PRICE_PRO_OVERAGE  the si_… metered subscription item; UNSET => the
#                           command plans-only (no Stripe push) and exits 0.
#   REPORT_OVERAGE_ARGS     extra flags to pass through (e.g. --dry-run --json).
#   ARGO_BIN / AWS_CLI      binaries (defaults on PATH / /usr/local/bin/argo)
# Exit codes: propagates `argo billing report-overage`'s exit code.
set -euo pipefail

WORKSPACE="${ARGO_WORKSPACE:-/data}"
DATA_DIR="${WORKSPACE%/}/.argo"
DB="${DATA_DIR}/sessions.db"
BUCKET="${DB_SNAPSHOT_S3_BUCKET:-}"
PREFIX="${DB_SNAPSHOT_S3_PREFIX:-gateway-live}"
ARGO_BIN="${ARGO_BIN:-/usr/local/bin/argo}"
AWS_CLI="${AWS_CLI:-aws}"
REPORT_OVERAGE_ARGS="${REPORT_OVERAGE_ARGS:-}"

log() { printf '{"src":"overage-cron","ts":"%s","level":"%s","msg":"%s"}\n' "$(date -u +%FT%TZ)" "${2:-info}" "$1"; }

# ---------- 1. make the live DB readable (read-only restore) ----------
if [ -z "$BUCKET" ]; then
  log "DB_SNAPSHOT_S3_BUCKET unset — reading DB in place at ${DB} (EFS / dev), no restore"
elif [ -s "$DB" ]; then
  log "local db already present at ${DB} — using it as-is (no restore)"
else
  log "no local db — consulting s3://${BUCKET}/${PREFIX%/}/ for the latest snapshot"
  latest="$("$AWS_CLI" s3 ls "s3://${BUCKET}/${PREFIX%/}/" 2>/dev/null | awk '{print $NF}' | grep '\.tar\.gz$' | sort | tail -1 || true)"
  if [ -z "$latest" ]; then
    # No snapshot yet = no Pro tenant has any usage to bill. NOT fatal: report
    # against the fresh empty DB the image pre-creates and exit cleanly.
    log "no snapshot listable — nothing has been billed yet; reporting against empty db (no-op expected)" "warn"
  else
    log "restoring latest snapshot: ${latest}"
    mkdir -p "$WORKSPACE"
    tmp="$(mktemp -d "${TMPDIR:-/tmp}/ov-restore.XXXXXX")"
    err="${tmp}/err.log"
    # `argo restore` refuses a non-empty target, and the image pre-creates
    # ${DATA_DIR}; restore into a fresh STAGING dir on the same filesystem then
    # swap it into place (atomic rename) — identical to the gateway entrypoint.
    stage="$(mktemp -d "${WORKSPACE%/}/.restore-stage.XXXXXX")"
    if "$AWS_CLI" s3 cp "s3://${BUCKET}/${PREFIX%/}/${latest}" "${tmp}/snap.tar.gz" --only-show-errors 2>"$err" \
        && "$ARGO_BIN" backup verify "${tmp}/snap.tar.gz" --json >/dev/null 2>>"$err" \
        && ( cd "$stage" && "$ARGO_BIN" restore "${tmp}/snap.tar.gz" --json >/dev/null 2>>"$err" ) \
        && [ -s "${stage}/.argo/sessions.db" ]; then
      rm -rf "$DATA_DIR"
      mv "${stage}/.argo" "$DATA_DIR"
      # Strip runtime locks (see gateway-entrypoint.sh) so the read-only argo CLI
      # opens cleanly. -shm is rebuilt from db + -wal on open.
      rm -f "${DATA_DIR}/gateway.pid" "${DATA_DIR}/sessions.db-shm"
      rm -rf "$tmp" "$stage"
      log "restore OK — db present at ${DB}"
    else
      reason="$(tr '\n"' '  ' <"$err" 2>/dev/null | head -c 300)"
      rm -rf "$stage" 2>/dev/null || true
      # A snapshot EXISTS but could not be restored => do NOT bill against an
      # empty db (that would under-report every Pro tenant). Fail loud; the next
      # daily run is the recovery, and idempotency means no double-bill later.
      log "snapshot ${latest} exists but download/verify/restore FAILED [${reason}] — refusing to bill against empty db (fail-closed)" "fatal"
      exit 70
    fi
  fi
fi

# ---------- 2. bridge the .argo db to where report-overage looks (.skycot) ----------
# See the DB-PATH BRIDGE note above. The handler opens `<cwd>/.skycot/sessions.db`
# (+ migrate(), which writes — and SQLite's create_if_missing creates the FILE but
# NOT its parent dir), so expose the restored/live .argo db at .skycot under
# $ARGO_WORKSPACE and run from there.
SKYCOT_DIR="${WORKSPACE%/}/.skycot"
if [ -e "$DB" ]; then
  # We have a real db at ${DATA_DIR}/sessions.db (restored or in-place). Symlink
  # .skycot -> .argo so it stays a single source of truth (no stale copy).
  if [ ! -e "${SKYCOT_DIR}/sessions.db" ]; then
    ln -sfn "$DATA_DIR" "$SKYCOT_DIR"
    log "bridged ${SKYCOT_DIR} -> ${DATA_DIR} so report-overage finds the db"
  fi
else
  # Fresh-empty case (no snapshot existed): no db to bridge. Just ensure the
  # parent dir exists so the CLI's create_if_missing can make an empty db
  # (which migrates to zero Pro tenants → a clean no-op run).
  mkdir -p "$SKYCOT_DIR"
  log "no db to bridge — created empty ${SKYCOT_DIR} for a clean no-op run"
fi
cd "$WORKSPACE"

# ---------- 3. run the idempotent overage report ----------
log "running (cwd=$(pwd)): argo billing report-overage ${REPORT_OVERAGE_ARGS}"
# shellcheck disable=SC2086  # REPORT_OVERAGE_ARGS is an intentional word-split flag list.
exec "$ARGO_BIN" billing report-overage ${REPORT_OVERAGE_ARGS}
