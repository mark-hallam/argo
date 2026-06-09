#!/usr/bin/env bash
# free-ram-mb.sh — the ONE canonical "available RAM in MB", measured correctly.
#
# Why this exists (root-fix, 2026-06-09): RAM measurement was hand-rolled from
# `vm_stat` at every call site, which made it trivial to bake in the WRONG page
# size. On Apple Silicon the VM page size is **16384 bytes, not the 4096** that
# x86 muscle-memory assumes. A hardcoded 4096 undercounts available RAM by 4×.
# That bit the conductor this session: ad-hoc `vm_stat | awk '...*4096...'`
# reported "64 MB free" when the machine actually had ~256 MB+ (and ~11 GB
# reclaimable), so it wrongly deferred Rust-PR gates as "RAM-constrained" for
# hours. The same 4× error in a pressure gate would also trip it 4× too eagerly.
#
# Fix: ONE implementation that parses the ACTUAL page size from vm_stat's own
# header line. Everything that needs free RAM calls this — never hand-roll
# `vm_stat` math again. (worker-supervisor.sh's two RAM-pressure gates both call
# this; the conductor should too instead of typing an ad-hoc awk.)
#
# Definition of "available": instantly-recyclable RAM = free + speculative +
# inactive + purgeable pages (matches the supervisor's RAM-pressure gate; macOS
# keeps lots of RAM "inactive"/cached but reclaims it under pressure, so omitting
# inactive — as a naive "free only" reading does — badly understates headroom).
#
# Portability: macOS uses `vm_stat` (real page size from its header). Linux (no
# vm_stat) uses /proc/meminfo MemAvailable (already kB, kernel-computed).
#
# Output : available MB as a bare integer on stdout.
# Exit   : 0 on success; 1 if RAM could not be measured. Callers should treat a
#          non-zero exit as "unknown — do NOT gate" (fail-open), preserving the
#          supervisor's prior "can't measure → don't pause" behavior.
set -uo pipefail

if command -v vm_stat >/dev/null 2>&1; then
  # macOS. Parse the real page size from the header ("(page size of N bytes)").
  vm_stat | awk '
    /page size of/      { ps=$8 }
    /Pages free/        { gsub(/\./,"",$3); fp=$3 }
    /Pages speculative/ { gsub(/\./,"",$3); sp=$3 }
    /Pages inactive/    { gsub(/\./,"",$3); ip=$3 }
    /Pages purgeable/   { gsub(/\./,"",$3); pp=$3 }
    END {
      if (ps == "" || fp == "") exit 1            # vm_stat unparseable
      printf "%d\n", (fp + sp + ip + pp) * ps / 1024 / 1024
    }
  '
  exit $?
elif [ -r /proc/meminfo ]; then
  # Linux. MemAvailable is the kernel'\''s own reclaimable estimate, in kB.
  awk '/^MemAvailable:/ { printf "%d\n", $2 / 1024; found=1 }
       END { if (!found) exit 1 }' /proc/meminfo
  exit $?
else
  exit 1
fi
