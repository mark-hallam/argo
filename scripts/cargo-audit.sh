#!/usr/bin/env bash
# Local vulnerability scan for the workspace.
#
# Idempotent install of cargo-audit, then runs `cargo audit` honoring
# `.cargo-audit-ignore.toml` at the repo root. Exits non-zero on
# findings. Pass `--fix` to forward to `cargo update`; the operator
# is expected to review the resulting Cargo.lock diff manually.
#
# Usage:
#   scripts/cargo-audit.sh           # scan; exit non-zero on findings
#   scripts/cargo-audit.sh --fix     # run cargo update for suggested upgrades
#   scripts/cargo-audit.sh --help    # show this help

set -euo pipefail

print_help() {
  sed -n '2,12p' "$0" | sed 's/^# \{0,1\}//'
}

if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
  print_help
  exit 0
fi

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
IGNORE_FILE="$REPO_ROOT/.cargo-audit-ignore.toml"

# Idempotent install — only invokes `cargo install` when missing.
if ! cargo install --list 2>/dev/null | grep -q '^cargo-audit '; then
  echo "Installing cargo-audit (one-time)..."
  cargo install cargo-audit --locked
fi

# Build --ignore args from the allowlist file (comment-only stub
# leaves args empty). Format: any line of the form `id = "RUSTSEC-..."`
# contributes a `--ignore RUSTSEC-...` argument.
IGNORE_ARGS=()
if [[ -f "$IGNORE_FILE" ]]; then
  while IFS= read -r advisory_id; do
    [[ -n "$advisory_id" ]] && IGNORE_ARGS+=(--ignore "$advisory_id")
  done < <(grep -E '^\s*id\s*=\s*"[^"]+"' "$IGNORE_FILE" \
    | sed -E 's/^\s*id\s*=\s*"([^"]+)".*/\1/')
fi

if [[ "${1:-}" == "--fix" ]]; then
  echo "Running cargo update to pick up suggested upgrades..."
  echo "Review the Cargo.lock diff before committing."
  cd "$REPO_ROOT"
  cargo update
  exit 0
fi

cd "$REPO_ROOT"
echo "Running cargo audit (${#IGNORE_ARGS[@]} advisory ignore(s) loaded from .cargo-audit-ignore.toml)..."
cargo audit --deny warnings ${IGNORE_ARGS[@]+"${IGNORE_ARGS[@]}"}
