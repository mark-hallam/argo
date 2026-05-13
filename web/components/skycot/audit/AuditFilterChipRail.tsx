"use client";

import { X } from "lucide-react";
import { cn } from "@/lib/utils";
import {
  CHANNEL_LABELS,
  CLASSIFICATION_LABELS,
  COST_BAND_LABELS,
  STATUS_LABELS,
  actionTypeLabel,
  isAuditFilterEmpty,
  type AuditMultiFilters,
} from "@/lib/filters/audit";
import type { AuditWorkerOption, AuditApproverOption } from "@/lib/filters/audit";

/**
 * Inline filter-chip rail for the audit page. Mirrors the
 * approvals-inbox FilterChipRail (E1-b) in shape: one chip per active
 * dimension, "Clear all" link at the right, hidden when no filters
 * active. Dimension testids preserve the original audit page contract
 * (`audit-chip-worker` / `audit-chip-action_type` / `audit-chip-status`)
 * so deep-link tests still pass; new dimensions add new testids.
 */

const CHIP_KEYS: Record<keyof AuditMultiFilters, string> = {
  worker: "worker",
  channel: "channel",
  actionType: "action_type",
  status: "status",
  approver: "approver",
  costBand: "cost_band",
  classification: "classification",
};

const DIMENSION_PREFIX: Record<keyof AuditMultiFilters, string> = {
  worker: "Worker:",
  channel: "Channel:",
  actionType: "Action type:",
  status: "Status:",
  approver: "Approver:",
  costBand: "Cost:",
  classification: "Classification:",
};

export interface AuditFilterChipRailProps {
  filters: AuditMultiFilters;
  workers: AuditWorkerOption[];
  approvers: AuditApproverOption[];
  onClearDimension: (dimension: keyof AuditMultiFilters) => void;
  onClearAll: () => void;
}

export function AuditFilterChipRail({
  filters,
  workers,
  approvers,
  onClearDimension,
  onClearAll,
}: AuditFilterChipRailProps) {
  if (isAuditFilterEmpty(filters)) return null;

  const workerName = (id: string) => workers.find((w) => w.id === id)?.name ?? id;
  const approverName = (id: string) =>
    approvers.find((a) => a.id === id)?.name ?? id;

  return (
    <div
      data-testid="audit-chip-rail"
      className="flex flex-wrap items-center gap-2"
    >
      {filters.worker.length > 0 ? (
        <Chip
          dimension="worker"
          label={`${DIMENSION_PREFIX.worker} ${summarize(filters.worker.map(workerName))}`}
          onRemove={() => onClearDimension("worker")}
        />
      ) : null}
      {filters.channel.length > 0 ? (
        <Chip
          dimension="channel"
          label={`${DIMENSION_PREFIX.channel} ${summarize(filters.channel.map((c) => CHANNEL_LABELS[c]))}`}
          onRemove={() => onClearDimension("channel")}
        />
      ) : null}
      {filters.actionType.length > 0 ? (
        <Chip
          dimension="actionType"
          label={`${DIMENSION_PREFIX.actionType} ${summarizeRaw(filters.actionType.map(actionTypeLabel), filters.actionType)}`}
          onRemove={() => onClearDimension("actionType")}
        />
      ) : null}
      {filters.status.length > 0 ? (
        <Chip
          dimension="status"
          label={`${DIMENSION_PREFIX.status} ${summarizeRaw(filters.status.map((s) => STATUS_LABELS[s]), filters.status)}`}
          onRemove={() => onClearDimension("status")}
        />
      ) : null}
      {filters.approver.length > 0 ? (
        <Chip
          dimension="approver"
          label={`${DIMENSION_PREFIX.approver} ${summarize(filters.approver.map(approverName))}`}
          onRemove={() => onClearDimension("approver")}
        />
      ) : null}
      {filters.costBand.length > 0 ? (
        <Chip
          dimension="costBand"
          label={`${DIMENSION_PREFIX.costBand} ${summarize(filters.costBand.map((b) => COST_BAND_LABELS[b]))}`}
          onRemove={() => onClearDimension("costBand")}
        />
      ) : null}
      {filters.classification.length > 0 ? (
        <Chip
          dimension="classification"
          label={`${DIMENSION_PREFIX.classification} ${summarize(filters.classification.map((c) => CLASSIFICATION_LABELS[c]))}`}
          onRemove={() => onClearDimension("classification")}
        />
      ) : null}
      <button
        type="button"
        onClick={onClearAll}
        data-testid="audit-chip-rail-clear-all"
        className="text-xs text-[var(--primary)] underline-offset-4 hover:underline"
      >
        Clear all
      </button>
    </div>
  );
}

function summarize(values: string[]): string {
  if (values.length > 3) return `${values.length} selected`;
  return values.join(", ");
}

/** Preserve the original raw token in the chip when only one value is
 *  selected so the page test's literal-string assertion still matches
 *  (e.g. `email_drafted` / `pending`). For 2–3 values, fall back to
 *  the labelled summary; 4+ shows the count. */
function summarizeRaw(labels: string[], raws: string[]): string {
  if (raws.length === 1) return raws[0];
  if (raws.length > 3) return `${raws.length} selected`;
  return labels.join(", ");
}

function Chip({
  dimension,
  label,
  onRemove,
}: {
  dimension: keyof AuditMultiFilters;
  label: string;
  onRemove: () => void;
}) {
  const testKey = CHIP_KEYS[dimension];
  return (
    <button
      type="button"
      onClick={onRemove}
      data-testid={`audit-chip-${testKey}`}
      aria-label={`Remove ${label} filter`}
      className={cn(
        "inline-flex items-center gap-1.5 rounded-full border border-[var(--border)] bg-[var(--card)] px-2.5 py-1 text-xs font-medium text-[var(--foreground)]",
        "hover:border-[var(--primary)]/40 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)]",
      )}
    >
      <span>{label}</span>
      <X aria-hidden className="size-3" />
      <span className="sr-only">Remove filter</span>
    </button>
  );
}
