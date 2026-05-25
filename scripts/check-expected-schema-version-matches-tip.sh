#!/usr/bin/env bash
set -euo pipefail

ROOT="${ARGO_SCHEMA_CHECK_REPO_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || true)}"
if [[ -z "$ROOT" ]]; then
  echo "PARSE_ERROR: not inside an argo git checkout" >&2
  exit 2
fi

MIGRATIONS_DIR="$ROOT/crates/argo-store/src/migrations"
LIB_FILE="$ROOT/crates/argo-store/src/lib.rs"

if [[ ! -d "$MIGRATIONS_DIR" ]]; then
  echo "PARSE_ERROR: migrations dir missing: $MIGRATIONS_DIR" >&2
  exit 2
fi
if [[ ! -f "$LIB_FILE" ]]; then
  echo "PARSE_ERROR: argo-store lib.rs missing: $LIB_FILE" >&2
  exit 2
fi

# shellcheck disable=SC2012
TIP_RAW="$(
  ls "$MIGRATIONS_DIR"/v*.rs 2>/dev/null \
    | sort -V \
    | tail -1 \
    | grep -oE 'v[0-9]+' \
    | head -1 || true
)"
if [[ -z "$TIP_RAW" ]]; then
  echo "PARSE_ERROR: no v*.rs migration files under $MIGRATIONS_DIR" >&2
  exit 2
fi
TIP_N="${TIP_RAW#v}"

EXPECTED_LINE="$(
  grep -nE 'pub[[:space:]]+const[[:space:]]+EXPECTED_SCHEMA_VERSION[[:space:]]*:[[:space:]]*u32[[:space:]]*=[[:space:]]*[0-9]+[[:space:]]*;' "$LIB_FILE" \
    | head -1 || true
)"
if [[ -z "$EXPECTED_LINE" ]]; then
  echo "PARSE_ERROR: EXPECTED_SCHEMA_VERSION constant not found in $LIB_FILE" >&2
  exit 2
fi
EXPECTED_LINE_N="${EXPECTED_LINE%%:*}"
EXPECTED_N="$(printf '%s' "$EXPECTED_LINE" | sed -E 's/.*=[[:space:]]*([0-9]+)[[:space:]]*;.*/\1/')"

if [[ "$EXPECTED_N" != "$TIP_N" ]]; then
  echo "❌ EXPECTED_SCHEMA_VERSION=$EXPECTED_N lags behind tip migration v$TIP_N. Bump crates/argo-store/src/lib.rs:$EXPECTED_LINE_N to $TIP_N. (This is the v83 pattern from round-67/68/69 — see docs/skycot/evals/scheduled/2026-05-25/round-67-pr-2317-bump-expected-schema-version-post-merge.md.)" >&2
  exit 1
fi

echo "✅ EXPECTED_SCHEMA_VERSION=$EXPECTED_N matches tip migration v$TIP_N."
