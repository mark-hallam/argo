#!/usr/bin/env bash
# Auto-generate the CLI reference page in book/reference/cli.md
# (or wherever the first arg points). Walks `argo --help` depth-first
# through every subcommand level, then splices the result between the
# BEGIN/END AUTO-GENERATED CLI REFERENCE markers.
#
# Usage:
#   scripts/build-cli-reference.sh [output-file]
#
# Defaults: output → book/reference/cli.md. Resolves the `argo` binary
# from $ARGO if set, else $PATH, else $CARGO_TARGET_DIR/release/argo,
# else target/release/argo.
#
# Output is byte-stable across runs at the same git revision (no
# timestamp, no host data). The drift CI relies on that.
set -euo pipefail

OUT_FILE="${1:-book/reference/cli.md}"

# Resolve the argo binary deterministically.
if [ -z "${ARGO:-}" ]; then
  if command -v argo >/dev/null 2>&1; then
    ARGO="argo"
  elif [ -n "${CARGO_TARGET_DIR:-}" ] && [ -x "$CARGO_TARGET_DIR/release/argo" ]; then
    ARGO="$CARGO_TARGET_DIR/release/argo"
  elif [ -x "target/release/argo" ]; then
    ARGO="target/release/argo"
  else
    echo "build-cli-reference: 'argo' not on PATH and no release binary found" >&2
    echo "  hint: cargo build --release -p argo-cli" >&2
    exit 1
  fi
fi

if ! "$ARGO" --help >/dev/null 2>&1; then
  echo "build-cli-reference: '$ARGO --help' failed" >&2
  exit 1
fi

if [ ! -f "$OUT_FILE" ]; then
  echo "build-cli-reference: $OUT_FILE not found" >&2
  exit 1
fi

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT
GEN_FILE="$TMP_DIR/cli.md"

# Strip ANSI color escapes defensively. clap normally writes plain
# text when stdout is not a TTY, but `--color=always` or some shells
# can leak escapes; the drift CI catches us if it ever happens.
strip_ansi() {
  sed -E $'s/\x1b\\[[0-9;]*[a-zA-Z]//g'
}

# Parse subcommand names from a help output passed on stdin. Reads
# the lines under `Commands:` until the next `Section:` header.
parse_subcommands() {
  awk '
    /^Commands:/                 { in_cmds=1; next }
    in_cmds && /^[A-Z][a-z]+:/   { in_cmds=0 }
    in_cmds && /^  [a-z]/ {
      gsub(/^ +/, "")
      split($0, parts, /[ \t]+/)
      print parts[1]
    }
  ' | { grep -vE '^(help)$' || true; } | sort -u
}

# Recursively emit one command's help block + every descendant.
# $1 = subcommand chain (empty for root, else space-separated, e.g.
#      "worker actions"). $2 = markdown heading depth (clamped to 6).
emit_command() {
  local cmd_path="$1"
  local depth="$2"
  if [ "$depth" -gt 6 ]; then depth=6; fi
  local hashes
  hashes="$(printf '%*s' "$depth" '' | tr ' ' '#')"

  local heading help_out
  if [ -z "$cmd_path" ]; then
    heading="\`argo\`"
    help_out="$("$ARGO" --help 2>&1 | strip_ansi || true)"
  else
    heading="\`argo $cmd_path\`"
    # shellcheck disable=SC2086
    help_out="$($ARGO $cmd_path --help 2>&1 | strip_ansi || true)"
  fi

  {
    echo "$hashes $heading"
    echo
    echo '```text'
    printf '%s\n' "$help_out"
    echo '```'
    echo
  } >> "$GEN_FILE"

  local subs sub
  subs="$(printf '%s\n' "$help_out" | parse_subcommands)"
  for sub in $subs; do
    if [ -z "$cmd_path" ]; then
      emit_command "$sub" $((depth + 1))
    else
      emit_command "$cmd_path $sub" $((depth + 1))
    fi
  done
}

{
  echo "_Auto-generated from \`argo --help\` recursively. Do not edit by"
  echo "hand — re-run \`scripts/build-cli-reference.sh\` to regenerate._"
  echo
} > "$GEN_FILE"

emit_command "" 2

# Splice generated content into OUT_FILE between BEGIN/END markers.
# If markers exist, replace the content between them. If not, append
# a new block at the bottom.
BEGIN_MARK="<!-- BEGIN AUTO-GENERATED CLI REFERENCE -->"
END_MARK="<!-- END AUTO-GENERATED CLI REFERENCE -->"

if grep -qF "$BEGIN_MARK" "$OUT_FILE" && grep -qF "$END_MARK" "$OUT_FILE"; then
  awk -v begin="$BEGIN_MARK" -v end="$END_MARK" -v gen_file="$GEN_FILE" '
    $0 == begin {
      print
      while ((getline line < gen_file) > 0) print line
      close(gen_file)
      in_block = 1
      next
    }
    $0 == end {
      print
      in_block = 0
      next
    }
    !in_block { print }
  ' "$OUT_FILE" > "$TMP_DIR/out.md"
  mv "$TMP_DIR/out.md" "$OUT_FILE"
else
  {
    echo
    echo "$BEGIN_MARK"
    cat "$GEN_FILE"
    echo "$END_MARK"
  } >> "$OUT_FILE"
fi

echo "build-cli-reference: wrote $(wc -l < "$OUT_FILE") lines to $OUT_FILE"
