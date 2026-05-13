"use client";

import * as Popover from "@radix-ui/react-popover";
import { Check, ChevronDown, ChevronRight } from "lucide-react";
import { useState, type ReactNode } from "react";
import { Button } from "@/components/ui/Button";
import { cn } from "@/lib/utils";
import {
  AUDIT_STATUSES,
  type AuditRow,
  type AuditStatus,
  type DataClassification,
} from "@/lib/types/audit";
import {
  CHANNEL_KINDS,
} from "@/lib/filters/approvals";
import type { ChannelKind } from "@/lib/sample-approvals";
import {
  CHANNEL_LABELS,
  CLASSIFICATION_LABELS,
  COST_BANDS,
  COST_BAND_LABELS,
  DATA_CLASSIFICATIONS,
  STATUS_LABELS,
  actionTypeLabel,
  auditActionTypeOptions,
  auditApproverOptions,
  auditWorkerOptions,
  isAuditFilterEmpty,
  type AuditMultiFilters,
  type CostBand,
} from "@/lib/filters/audit";

export interface AuditFilterPopoverProps {
  children: ReactNode;
  rows: AuditRow[];
  filters: AuditMultiFilters;
  onToggle: <K extends keyof AuditMultiFilters>(
    dimension: K,
    value: AuditMultiFilters[K][number],
  ) => void;
  onClearAll: () => void;
  /** RBAC gate — hides the data-classification dimension when false.
   *  v1 default true (no real RBAC yet); v1.5 swaps in real per-viewer
   *  permission via `mission-enterprise-audit-classification-rbac`. */
  canViewClassification?: boolean;
  open?: boolean;
  onOpenChange?: (open: boolean) => void;
}

type DimensionKey = keyof AuditMultiFilters;

const DEFAULT_OPEN: ReadonlySet<DimensionKey> = new Set(["worker", "status"]);

export function AuditFilterPopover({
  children,
  rows,
  filters,
  onToggle,
  onClearAll,
  canViewClassification = true,
  open,
  onOpenChange,
}: AuditFilterPopoverProps) {
  const hasAny = !isAuditFilterEmpty(filters);
  const workers = auditWorkerOptions(rows);
  const approvers = auditApproverOptions(rows);
  const actionTypes = auditActionTypeOptions(rows);

  return (
    <Popover.Root open={open} onOpenChange={onOpenChange}>
      <Popover.Trigger asChild>{children}</Popover.Trigger>
      <Popover.Portal>
        <Popover.Content
          align="end"
          sideOffset={8}
          collisionPadding={8}
          className={cn(
            "z-50 w-[340px] max-w-[calc(100vw-1rem)] outline-none",
            "rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--card)] shadow-lg",
            "data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=open]:zoom-in-95",
            "data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=closed]:zoom-out-95",
          )}
          data-testid="audit-filter-popover-content"
        >
          <div className="flex items-center justify-between border-b border-[var(--border)] px-4 py-3">
            <h2 className="text-sm font-semibold text-[var(--foreground)]">Filter</h2>
            <Button
              variant="link"
              size="sm"
              className="h-auto p-0 text-xs"
              disabled={!hasAny}
              onClick={onClearAll}
              data-testid="audit-filter-clear-all"
            >
              Clear all
            </Button>
          </div>

          <div className="max-h-[60vh] overflow-y-auto px-2 py-2">
            <Section
              dimension="worker"
              label="Worker"
              count={filters.worker.length}
              defaultOpen={DEFAULT_OPEN.has("worker")}
            >
              {workers.length === 0 ? (
                <Empty text="No workers yet." />
              ) : (
                workers.map((w) => (
                  <Option
                    key={w.id}
                    label={w.name}
                    checked={filters.worker.includes(w.id)}
                    onToggle={() => onToggle("worker", w.id)}
                    testId={`audit-filter-worker-${w.id}`}
                  />
                ))
              )}
            </Section>

            <Section
              dimension="channel"
              label="Channel"
              count={filters.channel.length}
              defaultOpen={DEFAULT_OPEN.has("channel")}
            >
              {CHANNEL_KINDS.map((c: ChannelKind) => (
                <Option
                  key={c}
                  label={CHANNEL_LABELS[c]}
                  checked={filters.channel.includes(c)}
                  onToggle={() => onToggle("channel", c)}
                  testId={`audit-filter-channel-${c}`}
                />
              ))}
            </Section>

            <Section
              dimension="actionType"
              label="Action type"
              count={filters.actionType.length}
              defaultOpen={DEFAULT_OPEN.has("actionType")}
            >
              {actionTypes.map((t) => (
                <Option
                  key={t}
                  label={actionTypeLabel(t)}
                  checked={filters.actionType.includes(t)}
                  onToggle={() => onToggle("actionType", t)}
                  testId={`audit-filter-action_type-${t}`}
                />
              ))}
            </Section>

            <Section
              dimension="status"
              label="Status"
              count={filters.status.length}
              defaultOpen={DEFAULT_OPEN.has("status")}
            >
              {AUDIT_STATUSES.map((s: AuditStatus) => (
                <Option
                  key={s}
                  label={STATUS_LABELS[s]}
                  checked={filters.status.includes(s)}
                  onToggle={() => onToggle("status", s)}
                  testId={`audit-filter-status-${s}`}
                />
              ))}
            </Section>

            <Section
              dimension="approver"
              label="Approver"
              count={filters.approver.length}
              defaultOpen={DEFAULT_OPEN.has("approver")}
            >
              {approvers.length === 0 ? (
                <Empty text="No approvers in the loaded set." />
              ) : (
                approvers.map((a) => (
                  <Option
                    key={a.id}
                    label={a.name}
                    checked={filters.approver.includes(a.id)}
                    onToggle={() => onToggle("approver", a.id)}
                    testId={`audit-filter-approver-${a.id}`}
                  />
                ))
              )}
            </Section>

            <Section
              dimension="costBand"
              label="Cost band"
              count={filters.costBand.length}
              defaultOpen={DEFAULT_OPEN.has("costBand")}
            >
              {COST_BANDS.map((b: CostBand) => (
                <Option
                  key={b}
                  label={COST_BAND_LABELS[b]}
                  checked={filters.costBand.includes(b)}
                  onToggle={() => onToggle("costBand", b)}
                  testId={`audit-filter-cost_band-${b}`}
                />
              ))}
            </Section>

            {canViewClassification ? (
              <Section
                dimension="classification"
                label="Data classification"
                count={filters.classification.length}
                defaultOpen={DEFAULT_OPEN.has("classification")}
              >
                {DATA_CLASSIFICATIONS.map((c: DataClassification) => (
                  <Option
                    key={c}
                    label={CLASSIFICATION_LABELS[c]}
                    checked={filters.classification.includes(c)}
                    onToggle={() => onToggle("classification", c)}
                    testId={`audit-filter-classification-${c}`}
                  />
                ))}
              </Section>
            ) : null}
          </div>
        </Popover.Content>
      </Popover.Portal>
    </Popover.Root>
  );
}

interface SectionProps {
  dimension: DimensionKey;
  label: string;
  count: number;
  defaultOpen: boolean;
  children: ReactNode;
}

function Section({ dimension, label, count, defaultOpen, children }: SectionProps) {
  const [open, setOpen] = useState(defaultOpen);
  return (
    <section
      className="px-2 py-1"
      data-testid={`audit-filter-section-${dimension}`}
      data-open={open}
    >
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        aria-expanded={open}
        data-testid={`audit-filter-section-toggle-${dimension}`}
        className={cn(
          "w-full flex items-center justify-between gap-2 px-2 py-1.5 rounded-[var(--radius-sm)]",
          "hover:bg-[var(--accent)]/40 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)]",
        )}
      >
        <span className="inline-flex items-center gap-1 text-[11px] font-semibold uppercase tracking-wider text-[var(--muted-foreground)]">
          {open ? <ChevronDown className="size-3" /> : <ChevronRight className="size-3" />}
          {label}
        </span>
        {count > 0 ? (
          <span
            className="rounded-full bg-[var(--primary)]/15 px-1.5 py-0.5 text-[10px] font-semibold text-[var(--primary)]"
            data-testid={`audit-filter-section-count-${dimension}`}
          >
            {count}
          </span>
        ) : null}
      </button>
      {open ? <ul className="flex flex-col">{children}</ul> : null}
    </section>
  );
}

function Empty({ text }: { text: string }) {
  return (
    <li>
      <p className="px-4 py-1 text-xs text-[var(--muted-foreground)]">{text}</p>
    </li>
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
        <span className="text-[var(--foreground)] truncate text-left">{label}</span>
        <span
          className={cn(
            "grid size-4 shrink-0 place-items-center rounded-[var(--radius-sm)] border",
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

