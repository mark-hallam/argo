#!/usr/bin/env bash
# Workspace-wide steady-state lint: every Rust crate in the
# workspace must have at least one `use <crate_name>::` reference
# in another crate's `src/` — unless the crate's `Cargo.toml`
# carries a `# zero-caller-allowed: <reason>` marker on a line
# above `[package]`.
#
# Catches the drift pattern that motivated the lint
# (claude-zero-caller-substrate-lint, evaluator round-3 finding):
# PR #2034 landed `argo-workplace-knowledge` as substrate-without-
# runtime; the would-be consumer brief sat BLOCKED for 4+ days
# before the gateway brief unblocked it via PR #2062. Discoverable
# via `cargo tree` but invisible to a glance — this lint surfaces
# the regression at merge time.
#
# Scope: the lint counts uses across every workspace member's
# `src/` directory (crates/ AND mcp-servers/ AND any other paths
# `cargo metadata` reports). Benches and integration tests under
# `tests/` are NOT counted — they exercise the crate but aren't a
# runtime caller.
#
# Usage:
#   ./scripts/check-zero-caller-substrate.sh [--root PATH] [--json]
#
# Exit codes:
#   0   every crate has callers OR is allowlisted
#   1   ≥1 unallowed zero-caller crate found
#   2   prerequisite missing (cargo / jq / grep not on PATH, or
#       cargo metadata fails)

set -uo pipefail

ROOT=""
JSON=false
for arg in "$@"; do
  case "$arg" in
    --root=*) ROOT="${arg#*=}" ;;
    --json)   JSON=true ;;
    --help|-h)
      sed -n '2,/^$/p' "$0" | sed 's/^#\s\?//'
      exit 0
      ;;
    *) echo "FAIL: unknown arg '$arg' (try --help)" >&2; exit 2 ;;
  esac
done

if [ -z "$ROOT" ]; then
  ROOT="$(cd "$(dirname "$0")/.." && pwd)"
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "FAIL: cargo required for cargo metadata" >&2
  exit 2
fi
if ! command -v jq >/dev/null 2>&1; then
  echo "FAIL: jq required to parse cargo metadata" >&2
  exit 2
fi

# Pull `(name, manifest_path)` pairs for every workspace member.
metadata=$(cd "$ROOT" && cargo metadata --no-deps --format-version 1 2>/dev/null) || {
  echo "FAIL: cargo metadata failed in $ROOT" >&2
  exit 2
}

# Build an array of "name<TAB>manifest_path<TAB>src_dir" lines.
# `src_dir` is the manifest's parent + /src; absent src/ dirs
# (rare but possible for tooling-only crates) collapse to empty
# and the crate is treated as "no own src/ to exclude".
entries=$(echo "$metadata" | jq -r '
  .packages[]
  | "\(.name)\t\(.manifest_path)"
')

# Pre-compute the union of every workspace `src/` directory for
# the use-count grep. This is one big list we pass to a single
# grep -r invocation per crate, excluding the crate's own src/.
all_src_dirs=()
while IFS=$'\t' read -r name manifest; do
  [ -z "$manifest" ] && continue
  src_dir="$(dirname "$manifest")/src"
  [ -d "$src_dir" ] && all_src_dirs+=("$src_dir")
done <<< "$entries"

missing=()
allowed=()
covered=()

# Allow markers: scan each Cargo.toml for a comment line of the
# form `# zero-caller-allowed: <reason>` anywhere above [package]
# or [workspace] (we accept it anywhere in the file for forgiveness).
is_allowlisted() {
  local manifest="$1"
  grep -qE '^[[:space:]]*#[[:space:]]*zero-caller-allowed[[:space:]]*:' "$manifest"
}

count_callers() {
  local underscored="$1" own_src="$2"
  local count=0
  local pattern="use ${underscored}::"
  for dir in "${all_src_dirs[@]}"; do
    [ "$dir" = "$own_src" ] && continue
    if grep -lr --include='*.rs' -F "$pattern" "$dir" >/dev/null 2>&1; then
      count=$((count + 1))
      # Don't break — count of >0 is all we need, but iterating
      # all dirs gives a more informative future error message
      # without adding latency on the no-match path.
    fi
  done
  echo "$count"
}

while IFS=$'\t' read -r name manifest; do
  [ -z "$name" ] && continue
  own_src="$(dirname "$manifest")/src"
  if is_allowlisted "$manifest"; then
    allowed+=("$name")
    continue
  fi
  underscored="$(echo "$name" | tr '-' '_')"
  callers=$(count_callers "$underscored" "$own_src")
  if [ "$callers" -eq 0 ]; then
    missing+=("$name")
  else
    covered+=("$name")
  fi
done <<< "$entries"

if $JSON; then
  printf '{"ok":%s,"covered":%d,"allowlisted":%d,"missing":[' \
    "$([ "${#missing[@]}" -eq 0 ] && echo true || echo false)" \
    "${#covered[@]}" "${#allowed[@]}"
  if [ "${#missing[@]}" -gt 0 ]; then
    sep=""
    for m in "${missing[@]}"; do
      printf '%s"%s"' "$sep" "$m"
      sep=","
    done
  fi
  printf ']}\n'
else
  if [ "${#missing[@]}" -gt 0 ]; then
    echo "❌ ${#missing[@]} crate(s) ship with zero callers AND no zero-caller-allowed marker:"
    for m in "${missing[@]}"; do
      echo "     $m"
    done
    echo
    echo "Fix one of:"
    echo "  1. Wire a real caller: add 'use $(echo "${missing[0]}" | tr '-' '_')::...' in another crate's src/."
    echo "  2. Mark the crate as intentionally standalone:"
    echo "       open the crate's Cargo.toml, add a comment line above [package]:"
    echo "         # zero-caller-allowed: <one-line reason — binary, MCP subprocess, scaffolding, ...>"
    echo
    echo "(${#covered[@]} crate(s) wired; ${#allowed[@]} allowlisted.)"
  else
    echo "✅ zero-caller-substrate check: ${#covered[@]} wired, ${#allowed[@]} allowlisted, 0 missing."
  fi
fi

[ "${#missing[@]}" -eq 0 ] && exit 0 || exit 1
