"use client";

import { ShieldCheck } from "lucide-react";
import { Card, CardBody, CardDescription, CardHeader, CardTitle } from "@/components/ui/Card";
import { StatusPill } from "@/components/ui/StatusPill";
import { cn } from "@/lib/utils";
import type { TrustGradeSnapshot, TrustGradeTone } from "@/lib/api/worker-trust";
import { TrustGradeProgress } from "./TrustGradeProgress";

/**
 * Trust-grade card — top-level surface on the per-worker Trust tab.
 *
 * Composition:
 * - Header: tone-aware grade badge + current label + persona name.
 * - Signals row: the underlying values that drive the grade
 *   (trust_grade_value, has_been_used, last_active_days_ago).
 * - Progress: 5-step ladder + criteria list (delegated to
 *   `TrustGradeProgress`).
 *
 * Strictly read-only — transparency is the load-bearing UX detail
 * per the brief. The badge tone matches the approval-card severity
 * vocabulary (success / warning / destructive) so operators read
 * the colour the same way across the SPA.
 */

export interface TrustGradeCardProps {
  snapshot: TrustGradeSnapshot;
  workerName?: string;
  className?: string;
}

/** Translate the backend's snake_case tone to the StatusPill prop. */
function toPillTone(tone: TrustGradeTone): "success" | "warning" | "error" {
  switch (tone) {
    case "success":
      return "success";
    case "warning":
      return "warning";
    case "destructive":
      return "error";
  }
}

function formatLastActive(daysAgo: number | null): string {
  if (daysAgo === null) return "Never active";
  if (daysAgo === 0) return "Active today";
  if (daysAgo === 1) return "Active 1 day ago";
  if (daysAgo < 30) return `Active ${daysAgo} days ago`;
  if (daysAgo < 60) return "Active over a month ago";
  return `Active ${Math.floor(daysAgo / 30)} months ago`;
}

export function TrustGradeCard({ snapshot, workerName, className }: TrustGradeCardProps) {
  const subjectName = workerName ?? "This worker";
  const pillTone = toPillTone(snapshot.currentGradeTone);

  return (
    <Card
      data-testid="trust-grade-card"
      data-tone={snapshot.currentGradeTone}
      className={cn("space-y-0", className)}
    >
      <CardHeader>
        <div className="flex items-start justify-between gap-3">
          <div className="space-y-1">
            <CardTitle className="flex items-center gap-2">
              <ShieldCheck className="size-4 text-[var(--muted-foreground)]" aria-hidden />
              Trust grade
            </CardTitle>
            <CardDescription>
              How much autonomy {subjectName} has earned, and what would unlock the next rung.
            </CardDescription>
          </div>
          <StatusPill
            data-testid="trust-grade-badge"
            data-grade={snapshot.currentGrade}
            tone={pillTone}
            size="md"
            dot
          >
            {snapshot.currentGradeLabel}
          </StatusPill>
        </div>
      </CardHeader>
      <CardBody className="space-y-6">
        <dl
          data-testid="trust-grade-signals"
          className="grid gap-3 sm:grid-cols-3"
        >
          <SignalCell label="Trust value" value={snapshot.signals.trustGradeValue.toString()} />
          <SignalCell
            label="Has been used"
            value={snapshot.signals.hasBeenUsed ? "Yes" : "Not yet"}
          />
          <SignalCell
            label="Last activity"
            value={formatLastActive(snapshot.signals.lastActiveDaysAgo)}
          />
        </dl>

        <TrustGradeProgress
          currentGrade={snapshot.currentGrade}
          currentGradeIndex={snapshot.currentGradeIndex}
          nextGrade={snapshot.nextGrade}
          nextGradeLabel={snapshot.nextGradeLabel}
          atMaxGrade={snapshot.atMaxGrade}
          criteria={snapshot.nextGradeCriteria}
        />
      </CardBody>
    </Card>
  );
}

function SignalCell({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--muted)]/30 px-3 py-2">
      <dt className="text-[11px] uppercase tracking-wide text-[var(--muted-foreground)]">
        {label}
      </dt>
      <dd className="mt-0.5 text-sm font-medium text-[var(--foreground)]">{value}</dd>
    </div>
  );
}
