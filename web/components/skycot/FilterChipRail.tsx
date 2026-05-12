"use client";

import { X } from "lucide-react";
import { cn } from "@/lib/utils";
import type { ChannelKind } from "@/lib/sample-approvals";
import {
  isFilterEmpty,
  type ApprovalFilters,
  type StatusKind,
} from "@/lib/filters/approvals";

const CHANNEL_LABEL: Record<ChannelKind, string> = {
  slack: "Slack",
  imessage: "iMessage",
  sms: "SMS",
  push: "Push",
  web: "Web inbox",
  voice: "Voice",
  whatsapp: "WhatsApp",
};

const STATUS_LABEL: Record<StatusKind, string> = {
  pending: "Pending",
  approved: "Approved",
  refused: "Refused",
};

export interface FilterChipRailProps {
  filters: ApprovalFilters;
  onClearDimension: (dimension: keyof ApprovalFilters) => void;
  onClearAll: () => void;
}

function summarize(values: string[]): string {
  if (values.length > 3) return `${values.length} selected`;
  return values.join(", ");
}

export function FilterChipRail({
  filters,
  onClearDimension,
  onClearAll,
}: FilterChipRailProps) {
  if (isFilterEmpty(filters)) return null;

  const channels = filters.channel.map((c) => CHANNEL_LABEL[c]);
  const statuses = filters.status.map((s) => STATUS_LABEL[s]);

  return (
    <div
      className="flex flex-wrap items-center gap-2 px-4 sm:px-6 lg:px-8 py-2 border-b border-[var(--border)] bg-[var(--background)]"
      data-testid="filter-chip-rail"
    >
      <span className="text-[10px] font-semibold uppercase tracking-wider text-[var(--muted-foreground)]">
        Filters
      </span>
      {filters.worker.length > 0 ? (
        <Chip
          label={`Worker: ${summarize(filters.worker)}`}
          onRemove={() => onClearDimension("worker")}
          testId="filter-chip-worker"
        />
      ) : null}
      {filters.channel.length > 0 ? (
        <Chip
          label={`Channel: ${summarize(channels)}`}
          onRemove={() => onClearDimension("channel")}
          testId="filter-chip-channel"
        />
      ) : null}
      {filters.status.length > 0 ? (
        <Chip
          label={`Status: ${summarize(statuses)}`}
          onRemove={() => onClearDimension("status")}
          testId="filter-chip-status"
        />
      ) : null}
      <button
        type="button"
        onClick={onClearAll}
        className="text-xs text-[var(--primary)] underline-offset-4 hover:underline"
        data-testid="filter-chip-rail-clear-all"
      >
        Clear all
      </button>
    </div>
  );
}

function Chip({
  label,
  onRemove,
  testId,
}: {
  label: string;
  onRemove: () => void;
  testId: string;
}) {
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1 rounded-full",
        "border border-[var(--border)] bg-[var(--card)]/60 px-2.5 py-1 text-xs text-[var(--foreground)]",
      )}
      data-testid={testId}
    >
      <span>{label}</span>
      <button
        type="button"
        onClick={onRemove}
        aria-label={`Remove ${label} filter`}
        className="text-[var(--muted-foreground)] hover:text-[var(--foreground)]"
        data-testid={`${testId}-remove`}
      >
        <X className="size-3" />
      </button>
    </span>
  );
}
