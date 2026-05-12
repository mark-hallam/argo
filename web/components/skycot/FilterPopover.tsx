"use client";

import * as Popover from "@radix-ui/react-popover";
import { Check } from "lucide-react";
import type { ReactNode } from "react";
import { Button } from "@/components/ui/Button";
import { cn } from "@/lib/utils";
import type { ChannelKind } from "@/lib/sample-approvals";
import {
  CHANNEL_KINDS,
  STATUS_KINDS,
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

export interface FilterPopoverProps {
  children: ReactNode;
  filters: ApprovalFilters;
  workerOptions: string[];
  onToggle: <K extends keyof ApprovalFilters>(
    dimension: K,
    value: ApprovalFilters[K][number],
  ) => void;
  onClearAll: () => void;
  open?: boolean;
  onOpenChange?: (open: boolean) => void;
}

export function FilterPopover({
  children,
  filters,
  workerOptions,
  onToggle,
  onClearAll,
  open,
  onOpenChange,
}: FilterPopoverProps) {
  const hasAny =
    filters.worker.length > 0 ||
    filters.channel.length > 0 ||
    filters.status.length > 0;

  return (
    <Popover.Root open={open} onOpenChange={onOpenChange}>
      <Popover.Trigger asChild>{children}</Popover.Trigger>
      <Popover.Portal>
        <Popover.Content
          align="end"
          sideOffset={8}
          collisionPadding={8}
          className={cn(
            "z-50 w-[320px] max-w-[calc(100vw-1rem)] outline-none",
            "rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--card)] shadow-lg",
            "data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=open]:zoom-in-95",
            "data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=closed]:zoom-out-95",
          )}
          data-testid="filter-popover-content"
        >
          <div className="flex items-center justify-between border-b border-[var(--border)] px-4 py-3">
            <h2 className="text-sm font-semibold text-[var(--foreground)]">Filter</h2>
            <Button
              variant="link"
              size="sm"
              className="h-auto p-0 text-xs"
              disabled={!hasAny}
              onClick={onClearAll}
              data-testid="filter-clear-all"
            >
              Clear all
            </Button>
          </div>

          <div className="max-h-[60vh] overflow-y-auto px-2 py-2">
            <Group label="Worker" dimension="worker">
              {workerOptions.length === 0 ? (
                <p className="px-2 py-1 text-xs text-[var(--muted-foreground)]">
                  No workers yet.
                </p>
              ) : (
                workerOptions.map((name) => (
                  <Option
                    key={name}
                    label={name}
                    checked={filters.worker.includes(name)}
                    onToggle={() => onToggle("worker", name)}
                    testId={`filter-worker-${name}`}
                  />
                ))
              )}
            </Group>

            <Group label="Channel" dimension="channel">
              {CHANNEL_KINDS.map((c) => (
                <Option
                  key={c}
                  label={CHANNEL_LABEL[c]}
                  checked={filters.channel.includes(c)}
                  onToggle={() => onToggle("channel", c)}
                  testId={`filter-channel-${c}`}
                />
              ))}
            </Group>

            <Group label="Status" dimension="status">
              {STATUS_KINDS.map((s) => (
                <Option
                  key={s}
                  label={STATUS_LABEL[s]}
                  checked={filters.status.includes(s)}
                  onToggle={() => onToggle("status", s)}
                  testId={`filter-status-${s}`}
                />
              ))}
            </Group>
          </div>
        </Popover.Content>
      </Popover.Portal>
    </Popover.Root>
  );
}

function Group({
  label,
  dimension,
  children,
}: {
  label: string;
  dimension: keyof ApprovalFilters;
  children: ReactNode;
}) {
  return (
    <section className="px-2 py-2" data-testid={`filter-group-${dimension}`}>
      <h3 className="px-2 pb-1 text-[10px] font-semibold uppercase tracking-wider text-[var(--muted-foreground)]">
        {label}
      </h3>
      <ul className="flex flex-col">{children}</ul>
    </section>
  );
}

function Option({
  label,
  checked,
  onToggle,
  testId,
}: {
  label: string;
  checked: boolean;
  onToggle: () => void;
  testId: string;
}) {
  return (
    <li>
      <button
        type="button"
        role="checkbox"
        aria-checked={checked}
        onClick={onToggle}
        data-testid={testId}
        className={cn(
          "w-full flex items-center justify-between gap-2 px-2 py-1.5 rounded-[var(--radius-sm)] text-sm",
          "hover:bg-[var(--accent)] hover:text-[var(--accent-foreground)] transition-colors",
          "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)]",
        )}
      >
        <span className="text-[var(--foreground)]">{label}</span>
        <span
          className={cn(
            "grid size-4 place-items-center rounded-[var(--radius-sm)] border",
            checked
              ? "bg-[var(--primary)] border-[var(--primary)] text-[var(--primary-foreground)]"
              : "border-[var(--border)] bg-transparent",
          )}
          aria-hidden
        >
          {checked ? <Check className="size-3" /> : null}
        </span>
      </button>
    </li>
  );
}
