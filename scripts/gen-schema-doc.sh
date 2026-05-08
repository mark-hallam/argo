#!/usr/bin/env bash
# Auto-generate docs/skycot/SCHEMA.md from the argo-store migrations
# (`crates/argo-store/migrations/*.sql` plus the SQL strings embedded
# in `crates/argo-store/src/migrations/v*.rs`) and the
# `sqlx::FromRow` row structs across `crates/argo-store/src/`.
#
#   scripts/gen-schema-doc.sh           # rewrite docs/skycot/SCHEMA.md
#   scripts/gen-schema-doc.sh --check   # exit 1 on drift vs. on-disk
#   scripts/gen-schema-doc.sh --stdout  # emit generated doc to stdout
#
# Heuristic-only. CREATE TRIGGER bodies are stripped; statements that
# don't match CREATE TABLE / ALTER TABLE / CREATE INDEX / CREATE
# VIRTUAL TABLE shapes are dropped silently. Output is sorted by
# table name for stable diffs.

set -euo pipefail
REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

OUT_FILE="docs/skycot/SCHEMA.md"
MODE="write"
case "${1:-}" in
  --check)  MODE="check" ;;
  --stdout) MODE="stdout" ;;
  "") ;;
  *) echo "gen-schema-doc.sh: unknown arg: $1" >&2; exit 2 ;;
esac

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# Step 1 — assemble a single SQL stream.
SQL="$WORK/all.sql"
{
  for f in $(ls crates/argo-store/migrations/*.sql 2>/dev/null | sort); do
    cat "$f"; echo
  done
  for f in $(ls crates/argo-store/src/migrations/v*.rs 2>/dev/null | sort -V); do
    awk '
      /const V[0-9]+_UP: &\[&str\] = &\[/ { in_block = 1; next }
      in_block && /^\];/                  { in_block = 0; next }
      in_block                            { print }
    ' "$f" | sed -E -e 's/^    "//' -e 's/",$/;/' -e 's/"$/;/' -e 's/\\$//'
    echo
  done
} > "$SQL"

# Step 2 — parse the SQL stream into TSV records.
RECORDS="$WORK/records.tsv"
awk '
  function emit(s,   upper, rest, idx, on, name) {
    sub(/^[[:space:]]+/, "", s); sub(/[[:space:]]+$/, "", s)
    gsub(/[[:space:]]+/, " ", s)
    sub(/;$/, "", s); sub(/[[:space:]]+$/, "", s)
    if (s == "") return
    upper = toupper(s)
    if (substr(upper, 1, 13) == "CREATE TABLE ") {
      rest = substr(s, 14)
      if (toupper(substr(rest, 1, 14)) == "IF NOT EXISTS ") rest = substr(rest, 15)
      if (match(rest, /^[A-Za-z_][A-Za-z0-9_]*/) > 0) {
        name = substr(rest, RSTART, RLENGTH)
        printf "TABLE\t%s\t%s\n", name, s
      }
    } else if (substr(upper, 1, 21) == "CREATE VIRTUAL TABLE ") {
      rest = substr(s, 22)
      if (toupper(substr(rest, 1, 14)) == "IF NOT EXISTS ") rest = substr(rest, 15)
      if (match(rest, /^[A-Za-z_][A-Za-z0-9_]*/) > 0) {
        name = substr(rest, RSTART, RLENGTH)
        printf "VTABLE\t%s\t%s\n", name, s
      }
    } else if (substr(upper, 1, 12) == "ALTER TABLE ") {
      rest = substr(s, 13)
      if (match(rest, /^[A-Za-z_][A-Za-z0-9_]*/) > 0) {
        name = substr(rest, RSTART, RLENGTH)
        printf "ALTER\t%s\t%s\n", name, s
      }
    } else if (match(upper, /^CREATE( UNIQUE)? INDEX( IF NOT EXISTS)? /) > 0) {
      rest = substr(s, RLENGTH + 1)
      if (match(rest, /^[A-Za-z_][A-Za-z0-9_]*/) > 0) {
        idx = substr(rest, RSTART, RLENGTH)
        rest = substr(rest, RLENGTH + 1)
        sub(/^ /, "", rest)
        if (toupper(substr(rest, 1, 3)) == "ON ") rest = substr(rest, 4)
        if (match(rest, /^[A-Za-z_][A-Za-z0-9_]*/) > 0) {
          on = substr(rest, RSTART, RLENGTH)
          printf "INDEX\t%s\t%s\t%s\n", on, idx, s
        }
      }
    }
  }
  { sub(/--.*/, ""); sub(/\/\/.*/, "") }
  /^[[:space:]]*$/                { next }
  /CREATE[[:space:]]+TRIGGER/     { in_trigger = 1 }
  in_trigger                      { if (/END[[:space:]]*;/) in_trigger = 0; next }
  {
    n = length($0)
    for (i = 1; i <= n; i++) {
      ch = substr($0, i, 1)
      buf = buf ch
      if      (ch == "(") depth++
      else if (ch == ")") depth--
      else if (ch == ";" && depth == 0) { emit(buf); buf = "" }
    }
    buf = buf " "
  }
  END { emit(buf) }
' "$SQL" > "$RECORDS"

# Step 3 — collect FromRow structs into TSV: struct_name \t fields_pipe_sep.
# Fields are pipe-separated (not newline) so each struct fits on one line for
# the per-table grep below.
STRUCTS="$WORK/structs.tsv"
awk '
  function flush() {
    if (sname != "") print sname "\t" fields
    sname = ""; fields = ""; capture = 0
  }
  /#\[derive\(.*FromRow.*\)\]/ { saw_derive = 1; next }
  saw_derive && /struct[[:space:]]+[A-Za-z_][A-Za-z0-9_]*/ {
    flush(); saw_derive = 0
    match($0, /struct[[:space:]]+[A-Za-z_][A-Za-z0-9_]*/)
    sname = substr($0, RSTART + 7, RLENGTH - 7)
    sub(/^[[:space:]]+/, "", sname)
    capture = 1; next
  }
  capture && /^}/ { flush(); next }
  capture && /^[[:space:]]*(pub[[:space:]]+)?[A-Za-z_][A-Za-z0-9_]*[[:space:]]*:/ {
    line = $0
    sub(/^[[:space:]]+/, "", line)
    sub(/^pub[[:space:]]+(\([^)]+\)[[:space:]]+)?/, "", line)
    sub(/,.*$/, "", line); sub(/[[:space:]]*$/, "", line)
    if (line != "") fields = fields (fields == "" ? "" : "|") line
  }
  END { flush() }
' $(find crates/argo-store/src -name "*.rs" | sort) > "$STRUCTS"

# Step 4 — render Markdown.
DOC="$WORK/SCHEMA.md"
{
  cat <<'HEADER'
# argo-store schema reference

<!-- DO NOT EDIT BY HAND. Auto-generated by scripts/gen-schema-doc.sh.
     Regenerate via `bash scripts/gen-schema-doc.sh`. CI fails on drift. -->

This document is generated from
`crates/argo-store/migrations/*.sql` and the SQL embedded in
`crates/argo-store/src/migrations/v*.rs`. Each section lists one
table's CREATE statement, accumulated `ALTER TABLE` additions, and
the indexes attached to it. `sqlx::FromRow` row structs that map to
each table are listed when the struct or table name matches.

## Tables

HEADER

  TABLES="$WORK/tables.txt"
  awk -F'\t' '$1 == "TABLE" || $1 == "VTABLE" { print $2 }' "$RECORDS" \
    | sort -u > "$TABLES"

  while IFS= read -r table; do
    echo "### \`$table\`"
    echo
    awk -F'\t' -v t="$table" '
      $1 == "TABLE"  && $2 == t { print "**Created**:\n\n```sql\n" $3 ";\n```\n" }
      $1 == "VTABLE" && $2 == t { print "**Created (virtual)**:\n\n```sql\n" $3 ";\n```\n" }
    ' "$RECORDS"

    alters="$(awk -F'\t' -v t="$table" '$1 == "ALTER" && $2 == t { print $3 ";" }' "$RECORDS")"
    if [ -n "$alters" ]; then
      echo "**Alterations**:"
      echo
      echo '```sql'
      printf '%s\n' "$alters"
      echo '```'
      echo
    fi

    indexes="$(awk -F'\t' -v t="$table" '$1 == "INDEX" && $2 == t { print $4 ";" }' "$RECORDS" | sort -u)"
    if [ -n "$indexes" ]; then
      echo "**Indexes**:"
      echo
      echo '```sql'
      printf '%s\n' "$indexes"
      echo '```'
      echo
    fi

    structs="$(awk -F'\t' -v t="$table" '
      {
        lc = tolower($1); lt = tolower(t)
        if (index(lc, lt) > 0 || index(lt, lc) > 0) print $0
      }
    ' "$STRUCTS")"
    if [ -n "$structs" ]; then
      echo "**Rust row structs**:"
      echo
      printf '%s\n' "$structs" | while IFS=$'\t' read -r sname srows; do
        [ -z "$sname" ] && continue
        echo "- \`$sname\`"
        printf '%s\n' "$srows" | tr '|' '\n' | sed 's/^/    - /'
      done
      echo
    fi
  done < "$TABLES"

  echo "## Limitations"
  echo
  cat <<'LIMITS'
- `CREATE TRIGGER` bodies are stripped from the parse stream — triggers
  exist in the schema but are not documented here.
- Heuristic regex parsing: statements that don't match
  `CREATE TABLE` / `ALTER TABLE` / `CREATE INDEX` /
  `CREATE VIRTUAL TABLE` are silently dropped. If you add a new DDL
  shape, extend `scripts/gen-schema-doc.sh`.
- `sqlx::FromRow` ↔ table mapping is a case-insensitive substring
  match between struct name and table name. Misses are common;
  treat the listing as advisory.
LIMITS
} > "$DOC"

# Step 5 — write / check / stdout.
case "$MODE" in
  stdout) cat "$DOC" ;;
  check)
    if [ ! -f "$OUT_FILE" ] || ! diff -q "$DOC" "$OUT_FILE" >/dev/null 2>&1; then
      echo "gen-schema-doc.sh: drift detected — run scripts/gen-schema-doc.sh and commit." >&2
      diff -u "$OUT_FILE" "$DOC" || true
      exit 1
    fi
    ;;
  write)
    mkdir -p "$(dirname "$OUT_FILE")"
    cp "$DOC" "$OUT_FILE"
    echo "gen-schema-doc.sh: wrote $(wc -l < "$OUT_FILE") lines to $OUT_FILE"
    ;;
esac
