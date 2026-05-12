"use client";

import { ActivityRow } from "@/components/skycot/ActivityRow";
import type { WorkerActivity, WorkerActivityStatus } from "@/lib/api/worker-activity";
import type { AuditDayBucket, AuditRow, AuditStatus } from "@/lib/types/audit";

export { groupByDay } from "@/lib/api/audit";

/**
 * Day-grouped section of the audit timeline. Renders a calendar-day
 * header ("TODAY" / "YESTERDAY" / "MAY 9") followed by one
 * `ActivityRow` per audit entry. The locked ActivityRow primitive
 * (E4-c) consumes the `WorkerActivity` shape, so we adapt each
 * `AuditRow` here — preserving id, verb, timestamp, worker, cost —
 * and map `status: "autonomous"` to `"succeeded"` for the pill
 * (a sensible-deviation; ActivityRow is locked at 4 statuses, the
 * raw audit status is exposed via `data-status` for E5-c styling).
 */

export interface AuditDayGroupProps {
  bucket: AuditDayBucket;
}

export function AuditDayGroup({ bucket }: AuditDayGroupProps) {
  return (
    <section
      data-testid={`audit-day-group-${bucket.key}`}
      data-day-key={bucket.key}
      aria-label={`Audit entries — ${bucket.label}`}
      className="space-y-2"
    >
      <h2
        data-testid={`audit-day-header-${bucket.key}`}
        className="sticky top-0 z-10 -mx-2 px-2 py-1.5 text-[11px] font-semibold uppercase tracking-wider text-[var(--muted-foreground)] bg-[var(--background)]/85 backdrop-blur-sm"
      >
        {bucket.label}
      </h2>
      <div role="list" className="space-y-2">
        {bucket.rows.map((row) => (
          <div
            key={row.id}
            data-status={row.status}
            data-action-type={row.actionType}
            data-worker-id={row.worker.id}
            data-testid={`audit-row-wrapper-${row.id}`}
          >
            <ActivityRow activity={toActivity(row)} />
          </div>
        ))}
      </div>
    </section>
  );
}

const STATUS_MAP: Record<AuditStatus, WorkerActivityStatus> = {
  succeeded: "succeeded",
  refused: "refused",
  pending: "pending",
  failed: "failed",
  autonomous: "succeeded",
};

function initials(name: string): string {
  const parts = name.trim().split(/\s+/);
  if (parts.length === 0) return "??";
  if (parts.length === 1) return parts[0].slice(0, 2).toUpperCase();
  return (parts[0][0] + parts[parts.length - 1][0]).toUpperCase();
}

/** Adapt an AuditRow to the locked WorkerActivity shape that
 *  ActivityRow consumes. actionType isn't rendered by ActivityRow,
 *  so we stub it as `message_sent` — the raw value lives on
 *  `data-action-type` on the wrapper for downstream consumers. */
function toActivity(row: AuditRow): WorkerActivity {
  return {
    id: row.id,
    workerId: row.worker.id,
    workerName: row.worker.name,
    workerInitials: initials(row.worker.name),
    timestamp: row.timestamp,
    verb: row.verb,
    actionType: "message_sent",
    status: STATUS_MAP[row.status],
    costUsd: row.cost?.usd,
  };
}

