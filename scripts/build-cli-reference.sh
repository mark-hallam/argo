#!/usr/bin/env bash
# Auto-generate the CLI reference page in book/reference/cli.md
# (or wherever the first arg points). Walks `argo --help`
# recursively, then writes the result between the
# BEGIN/END AUTO-GENERATED CLI REFERENCE markers.
#
# Usage:
#   scripts/build-cli-reference.sh [output-file]
#
# If no output-file is given, defaults to book/reference/cli.md.
#
# Requires the `argo` binary to be on PATH (cargo build first if
# you don't have one). Exits non-zero on any error so CI fails
# loudly.
set -euo pipefail

OUT_FILE="${1:-book/reference/cli.md}"
ARGO="${ARGO:-argo}"

if ! command -v "$ARGO" >/dev/null 2>&1; then
  echo "build-cli-reference: '$ARGO' not on PATH" >&2
  echo "  hint: cargo build --release -p argo-cli && export PATH=\$PWD/target/release:\$PATH" >&2
  exit 1
fi

if [ ! -f "$OUT_FILE" ]; then
  echo "build-cli-reference: $OUT_FILE not found" >&2
  exit 1
fi

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

GEN_FILE="$TMP_DIR/cli.md"

# Capture the top-level help. Most subcommands surface in the
# "Commands:" section; their names sit at the start of indented
# lines.
TOP_HELP="$("$ARGO" --help 2>&1)"

{
  echo "_Generated $(date -u +%Y-%m-%dT%H:%M:%SZ) from \`$ARGO --help\`._"
  echo
  echo "## Top-level"
  echo
  echo '```text'
  printf '%s\n' "$TOP_HELP"
  echo '```'
  echo
} > "$GEN_FILE"

# Extract subcommand names from the "Commands:" block. Lines that
# begin with two spaces, a non-space char, then more chars are the
# command listings; the first whitespace-separated token is the
# name.
SUBCOMMANDS="$(printf '%s\n' "$TOP_HELP" \
  | awk '
      /^Commands:/      { in_cmds=1; next }
      /^[A-Z][a-z]+:/   { in_cmds=0 }
      in_cmds && /^  [a-z]/ {
        # First non-whitespace token only.
        gsub(/^ +/, "")
        split($0, parts, /[ \t]+/)
        print parts[1]
      }
    ' \
  | grep -vE '^(help)$' \
  | sort -u)"

if [ -z "$SUBCOMMANDS" ]; then
  echo "build-cli-reference: no subcommands parsed from top-level help" >&2
  echo "  (this often means '$ARGO --help' format changed; update the awk parser)" >&2
  exit 1
fi

echo "## Subcommands" >> "$GEN_FILE"
echo >> "$GEN_FILE"

for cmd in $SUBCOMMANDS; do
  HELP_OUT="$("$ARGO" "$cmd" --help 2>&1 || true)"
  {
    echo "### \`argo $cmd\`"
    echo
    echo '```text'
    printf '%s\n' "$HELP_OUT"
    echo '```'
    echo
  } >> "$GEN_FILE"
done

# Splice the generated block into OUT_FILE between the markers.
# If markers exist, replace the content between them. If they
# don't, append a new block at the bottom.
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
