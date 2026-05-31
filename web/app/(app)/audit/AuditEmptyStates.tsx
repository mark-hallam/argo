"use client";

import { EmptyState } from "@/components/ui/EmptyState";
import { AuditIllustration } from "@/components/empty-state/illustrations/AuditIllustration";
import type { AuditWorkspaceState } from "@/lib/hooks/use-audit-feed";
import type { AuditMultiFilters } from "@/lib/filters/audit";

/**
 * `/audit` empty-state views, extracted from `page.tsx` to keep that file
 * under the 500-LOC ceiling. Pristine (no filters) shows a fresh-workspace
 * vs quiet-day copy driven by the look-back freshness probe; the filtered
 * variant explains which active filters narrowed the feed to zero rows.
 */
export function AuditEmpty({
  isPristine,
  filters,
  workspaceState,
  onClearAll,
}: {
  isPristine: boolean;
  filters: AuditMultiFilters;
  workspaceState: AuditWorkspaceState;
  onClearAll: () => void;
}) {
  if (isPristine) {
    // Probe whether the workspace has any 30-day-window history. If
    // the probe knows the workspace is fresh, switch to the "no
    // activity yet" framing; otherwise fall back to the existing
    // "Quiet day" copy (also the safe default while the probe is
    // still loading or errored).
    if (workspaceState === "fresh") {
      return (
        <EmptyState
          data-testid="audit-empty-fresh"
          illustration={AuditIllustration}
          headline="No audit activity yet"
          body="Your team's actions will appear here as they happen — once a worker takes an action or an approval lands, you'll see it on this timeline."
        />
      );
    }
    return (
      <EmptyState
        data-testid="audit-empty-pristine"
        illustration={AuditIllustration}
        headline="Quiet day"
        body="Worker actions and approvals will appear here as they happen. Plenty of room for activity."
      />
    );
  }
  return (
    <EmptyState
      data-testid="audit-empty-filtered"
      variant="compact"
      headline="No actions match the current filters"
      body={filteredEmptyDetail(filters)}
      cta={{ label: "Clear filters", onClick: onClearAll }}
    />
  );
}

function filteredEmptyDetail(filters: AuditMultiFilters): string {
  const active: string[] = [];
  if (filters.worker.length > 0) active.push("worker");
  if (filters.channel.length > 0) active.push("channel");
  if (filters.actionType.length > 0) active.push("action type");
  if (filters.status.length > 0) active.push("status");
  if (filters.approver.length > 0) active.push("approver");
  if (filters.costBand.length > 0) active.push("cost band");
  if (filters.classification.length > 0) active.push("classification");
  if (active.length === 0) {
    return "Try widening the time range or clearing search.";
  }
  if (active.length === 1) {
    return `Try removing the ${active[0]} filter or widening the time range.`;
  }
  return `Try removing one of the ${active.length} active filters or widening the time range.`;
}
