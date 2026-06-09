#!/usr/bin/env bash
# test-free-ram-mb.sh — guards scripts/free-ram-mb.sh, and specifically the
# page-size regression it exists to prevent (hardcoded 4096 on a 16384-byte-page
# machine undercounts RAM 4×).
set -uo pipefail
DIR="$(cd "$(dirname "$0")" && pwd)"
HELPER="$DIR/free-ram-mb.sh"
fail() { echo "FAIL: $*" >&2; exit 1; }

out="$("$HELPER")" || fail "helper exited non-zero"
[[ "$out" =~ ^[0-9]+$ ]] || fail "output is not a bare integer: '$out'"
[ "$out" -gt 0 ] || fail "available MB not positive: $out"

if command -v sysctl >/dev/null 2>&1 && sysctl -n hw.memsize >/dev/null 2>&1; then
  total_mb=$(( $(sysctl -n hw.memsize) / 1024 / 1024 ))
  [ "$out" -le "$total_mb" ] || fail "available ($out MB) exceeds total RAM ($total_mb MB)"
fi

# Regression guard: recompute the SAME quantity with the page size parsed from
# vm_stat's header, and assert the helper is within tolerance of it (NOT ~4×
# lower, which is exactly what a hardcoded 4096 would produce). The 30% band
# absorbs RAM drift between the two vm_stat calls.
if command -v vm_stat >/dev/null 2>&1; then
  ref="$(vm_stat | awk '
    /page size of/      { ps=$8 }
    /Pages free/        { gsub(/\./,"",$3); fp=$3 }
    /Pages speculative/ { gsub(/\./,"",$3); sp=$3 }
    /Pages inactive/    { gsub(/\./,"",$3); ip=$3 }
    /Pages purgeable/   { gsub(/\./,"",$3); pp=$3 }
    END { printf "%d", (fp+sp+ip+pp)*ps/1024/1024 }')"
  [ "$ref" -gt 0 ] || fail "reference computed non-positive ($ref)"
  # |out-ref| must be < 30% of ref. A 4× error would be ~75% off → caught.
  delta=$(( out > ref ? out - ref : ref - out ))
  limit=$(( ref * 30 / 100 ))
  [ "$delta" -le "$limit" ] || fail "helper ($out MB) diverges >30% from page-size-correct reference ($ref MB) — likely a page-size bug (delta=$delta limit=$limit)"
fi

echo "PASS: free-ram-mb.sh = ${out}MB (positive integer, real page size, within tolerance of reference)"
