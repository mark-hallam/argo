#!/usr/bin/env bash
# Enforce SAFETY comments on audited libc::kill FFI shims.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

status=0
while IFS=: read -r file line _; do
    context="$(sed -n "${line},$((line + 6))p" "$file")"
    [[ "$context" == *"libc::kill"* ]] || continue

    pre_start=$(( line > 3 ? line - 3 : 1 ))
    if (( line > 1 )); then
        pre_context="$(sed -n "${pre_start},$((line - 1))p" "$file")"
    else
        pre_context=""
    fi

    if ! grep -q "SAFETY:" <<<"$pre_context"; then
        echo "FAIL: $file:$line: libc::kill unsafe block missing preceding SAFETY comment" >&2
        status=1
    fi
done < <(git grep -n -E 'unsafe[[:space:]]*\{' -- '*.rs')

if (( status == 0 )); then
    echo "unsafe-safety: all libc::kill unsafe blocks have SAFETY comments"
fi
exit "$status"
