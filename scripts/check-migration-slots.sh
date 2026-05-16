#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MIGRATIONS_DIR="$ROOT/crates/argo-store/migrations"

find "$MIGRATIONS_DIR" -maxdepth 1 -type f -name '*__*.sql' -print \
  | sed 's#.*/##' \
  | sort \
  | awk '
      {
        split($0, parts, "__")
        slot = parts[1]
        count[slot] += 1
        files[slot] = files[slot] (files[slot] == "" ? "" : "|") $0
      }
      END {
        bad = 0
        for (slot in count) {
          if (count[slot] <= 1) {
            continue
          }
          printf("migration slot %s has duplicate files: %s\n", slot, files[slot]) > "/dev/stderr"
          bad = 1
        }
        if (bad) {
          printf("Duplicate argo-store migration slots are not allowed. Pick the next free filename slot.\n") > "/dev/stderr"
        }
        exit bad
      }
    '
