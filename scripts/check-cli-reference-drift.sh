#!/usr/bin/env bash
# Fail when book/reference/cli.md differs from the current `argo --help`
# recursive walk. Build the binary first, or set ARGO to the binary path.
set -euo pipefail

CANONICAL="book/reference/cli.md"

if [ ! -f "$CANONICAL" ]; then
  echo "check-cli-reference-drift: $CANONICAL not found" >&2
  exit 1
fi

TMP_FILE="$(mktemp)"
trap 'rm -f "$TMP_FILE"' EXIT

bash scripts/build-cli-reference.sh > "$TMP_FILE"

if ! diff -u "$CANONICAL" "$TMP_FILE"; then
  echo "check-cli-reference-drift: $CANONICAL is out of date" >&2
  echo "  regenerate with: ARGO=<path-to-argo> scripts/build-cli-reference.sh $CANONICAL" >&2
  exit 1
fi

echo "check-cli-reference-drift: $CANONICAL is current"
