#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MIGRATIONS_DIR="$ROOT/crates/argo-store/migrations"

known_duplicates='
19:19__anomaly.sql|19__worker_fs_root.sql
22:22__roster.sql|22__tenant_archived.sql
23:23__handoff_routing.sql|23__workflow_checkpoints.sql
24:24__operator_feedback.sql|24__skill_quality_score.sql|24__vertical_archived.sql
26:26__data_classification.sql|26__spend_baseline.sql|26__worker_budget_alert.sql
27:27__handoff_auctions.sql|27__spend_anomaly.sql
28:28__recovery_cache.sql|28__role_duties.sql|28__tenant_quota.sql
30:30__learned_plan_templates.sql|30__worker_charter.sql
45:45__cu_screenshot_index.sql|45__worker_dependency.sql|45__worker_level.sql
51:51__audit_worker_deleted_at.sql|51__worker_skill.sql
75:75__scim_provisioned_tombstones.sql|75__siem_dispatch_cursor.sql
'

{
  printf '%s\n' "$known_duplicates" | sed '/^$/d' | sed 's/^/KNOWN /'
  find "$MIGRATIONS_DIR" -maxdepth 1 -type f -name '*__*.sql' -print \
    | sed 's#.*/##' \
    | sort \
    | sed 's/^/FILE /'
} | awk '
    $1 == "KNOWN" {
      row = substr($0, 7)
      split(row, parts, ":")
      allowed[parts[1] ":" parts[2]] = 1
      next
    }
    $1 == "FILE" {
      file = substr($0, 6)
      split(file, parts, "__")
      slot = parts[1]
      count[slot] += 1
      files[slot] = files[slot] (files[slot] == "" ? "" : "|") file
      next
    }
    END {
      bad = 0
      for (slot in count) {
        if (count[slot] <= 1) {
          continue
        }
        key = slot ":" files[slot]
        if (!(key in allowed)) {
          printf("migration slot %s has duplicate files: %s\n", slot, files[slot]) > "/dev/stderr"
          bad = 1
        }
      }
      if (bad) {
        printf("New duplicate argo-store migration slots are not allowed. Pick the next free filename slot.\n") > "/dev/stderr"
      }
      exit bad
    }
  '
