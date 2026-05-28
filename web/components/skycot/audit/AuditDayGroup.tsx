"use client";

import { AuditTableRow } from "@/components/skycot/audit/AuditTableRow";
import type { WorkerActivity, WorkerActivityStatus } from "@/lib/api/worker-activity";
import type { AuditDayBucket, AuditRow, AuditStatus } from "@/lib/types/audit";

export { groupByDay } from "@/lib/api/audit";

/**
 * Day-grouped section of the audit timeline. Renders a calendar-day
 * header ("TODAY" / "YESTERDAY" / "MAY 9") followed by a real
 * `<table>` (Etherscan-parity semantics): screen readers announce
 * "table, N rows, columns: time, worker, verb, id, status" and the
 * macOS VoiceOver table rotor can navigate column-by-column.
 *
 * `<AuditTableRow>` is the table-shaped audit-row primitive — the
 * shared `<ActivityRow>` stays as `<Link>`-outer for cohort drawer
 * and worker-activity feeds, which intentionally aren't tabular.
 *
 * Each row carries `data-status` matching the raw `AuditStatus`
 * (including `autonomous`) for E5-c styling hooks; the pill itself
 * maps `autonomous → succeeded` (StatusPill is locked at 4 tones).
 */

export interface AuditDayGroupProps {
  bucket: AuditDayBucket;
}

export function AuditDayGroup({ bucket }: AuditDayGroupProps) {
  const dayHeadingId = `audit-day-${bucket.key}`;
  const captionId = `${dayHeadingId}-caption`;
  return (
    <section
      data-testid={`audit-day-group-${bucket.key}`}
      data-day-key={bucket.key}
      aria-labelledby={dayHeadingId}
      className="space-y-2"
    >
      <h2
        id={dayHeadingId}
        data-testid={`audit-day-header-${bucket.key}`}
        className="sticky top-0 z-10 -mx-2 px-2 py-1 text-[11px] font-semibold uppercase tracking-wider text-[var(--muted-foreground)] bg-[var(--background)]/85 backdrop-blur-sm"
      >
        {bucket.label}
      </h2>
      <table
        data-testid={`audit-day-table-${bucket.key}`}
        aria-describedby={captionId}
        className="w-full border-separate border-spacing-y-2 text-left"
      >
        <caption id={captionId} className="sr-only">
          Audit rows for {bucket.label}
        </caption>
        <thead className="sr-only">
          <tr>
            <th scope="col">Time</th>
            <th scope="col">Worker</th>
            <th scope="col">Verb</th>
            <th scope="col">Id</th>
            <th scope="col">Status</th>
          </tr>
        </thead>
        <tbody>
          {bucket.rows.map((row) => (
            <AuditTableRow
              key={row.id}
              activity={toActivity(row)}
              status={row.status}
              actionType={row.actionType}
              workerId={row.worker.id}
            />
          ))}
        </tbody>
      </table>
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
 *  AuditTableRow consumes. actionType isn't rendered by the row, so
 *  we stub it as `message_sent` — the raw value lives on
 *  `data-action-type` on the row for downstream consumers. The
 *  per-row chain hash threads through verbatim so the row can render
 *  the Etherscan-style truncated cell. */
function toActivity(row: AuditRow): WorkerActivity {
  const selfHash = row.selfHash ?? row.hash;
  const sampleHash = row.sampleHash;
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
    hash: selfHash ?? sampleHash,
    hashKind: selfHash ? "self" : sampleHash ? "sample" : undefined,
    parentHash: row.parentHash,
  };
}
