"use client";

import { Download, Hash, ShieldAlert, ShieldCheck } from "lucide-react";
import { buttonVariants } from "@/components/ui/Button";
import {
  AUDIT_RANGE_LABELS,
  isCustomRange,
  type AuditRange,
  type AuditRow,
} from "@/lib/types/audit";
import { cn } from "@/lib/utils";

interface AuditChainExplorerHeroProps {
  rows: AuditRow[];
  range: AuditRange;
  isFiltered: boolean;
  onExportClick: () => void;
}

type EvidenceState = "empty" | "missing" | "partial" | "present";

export function AuditChainExplorerHero({
  rows,
  range,
  isFiltered,
  onExportClick,
}: AuditChainExplorerHeroProps) {
  const summary = summarizeRows(rows);
  const rangeLabel = isCustomRange(range)
    ? "Custom range"
    : AUDIT_RANGE_LABELS[range];
  const latestHash = summary.latest ? rowHash(summary.latest) : null;
  const latestLabel = summary.latest?.id ?? "No rows yet";
  const state = evidenceState(summary.hashRows, rows.length);
  const copy = EVIDENCE_COPY[state];

  return (
    <section
      data-testid="audit-chain-hero"
      aria-label="Audit chain evidence summary"
      className="mx-auto max-w-6xl px-4 pt-6 sm:px-6 lg:px-8"
    >
      <div
        className="relative overflow-hidden rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--card)] p-5 shadow-sm sm:p-6"
        style={{
          background:
            "linear-gradient(135deg, color-mix(in oklch, var(--primary) 7%, var(--card)), color-mix(in oklch, var(--gradient-end) 6%, var(--card)))",
        }}
      >
        <div
          aria-hidden
          className="pointer-events-none absolute -right-16 -top-20 size-56 rounded-full opacity-45 blur-3xl"
          style={{ background: "var(--brand-gradient)" }}
        />
        <div className="relative grid gap-5 lg:grid-cols-[minmax(0,1fr)_auto] lg:items-center">
          <div className="flex min-w-0 gap-4">
            <span
              className={cn(
                "grid size-12 shrink-0 place-items-center rounded-full text-[var(--primary-foreground)] shadow-sm",
                state === "present"
                  ? "bg-[var(--success)]"
                  : "bg-[var(--warning)] text-[var(--warning-foreground)]",
              )}
            >
              {state === "present" ? (
                <ShieldCheck aria-hidden className="size-5" />
              ) : (
                <ShieldAlert aria-hidden className="size-5" />
              )}
            </span>

            <div className="min-w-0">
              <p className="font-mono text-[11px] font-semibold uppercase tracking-[0.16em] text-[var(--muted-foreground)]">
                {rangeLabel}
                {isFiltered ? " · Filtered view" : " · Current view"}
              </p>
              <h2
                data-testid="audit-chain-status"
                className="mt-1 text-lg font-semibold tracking-tight text-[var(--foreground)] sm:text-xl"
              >
                {copy.title}
              </h2>
              <p className="mt-1 max-w-3xl text-sm leading-6 text-[var(--muted-foreground)]">
                {copy.body}
              </p>
            </div>
          </div>

          <div className="flex flex-wrap gap-2 lg:justify-end">
            <button
              type="button"
              data-testid="audit-chain-export"
              onClick={onExportClick}
              className={buttonVariants({ variant: "primary", size: "sm" })}
            >
              <Download aria-hidden className="size-4" />
              Proof bundle
            </button>
            <span
              data-testid="audit-chain-verifier-note"
              className="inline-flex items-center rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--card)]/80 px-3 py-2 text-xs font-medium text-[var(--muted-foreground)]"
            >
              CLI verify not run in browser
            </span>
          </div>
        </div>

        <div className="relative mt-5 grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
          <HeroStat label="Latest action" value={latestLabel} mono />
          <HeroStat
            label="Chain head in view"
            value={latestHash ? abbreviateHash(latestHash) : "Unavailable"}
            mono
          />
          <HeroStat
            label="Rows with hashes"
            value={`${summary.hashRows}/${rows.length}`}
            tone={state === "present" ? "success" : "warning"}
          />
          <HeroStat label="Spend in view" value={formatUsd(summary.spendUsd)} />
        </div>

        <div className="relative mt-4 flex flex-wrap gap-2 text-xs text-[var(--muted-foreground)]">
          <MetricPill label="Pending" value={summary.pending} />
          <MetricPill label="Refused" value={summary.refused} />
          <MetricPill label="Failed" value={summary.failed} />
          <MetricPill label="Parent links" value={summary.parentLinks} />
        </div>
      </div>
    </section>
  );
}

function summarizeRows(rows: AuditRow[]) {
  return {
    latest: rows[0],
    hashRows: rows.filter((row) => rowHash(row)).length,
    parentLinks: rows.filter((row) => row.parentHash).length,
    pending: rows.filter((row) => row.status === "pending").length,
    refused: rows.filter((row) => row.status === "refused").length,
    failed: rows.filter((row) => row.status === "failed").length,
    spendUsd: rows.reduce((sum, row) => sum + (row.cost?.usd ?? 0), 0),
  };
}

function rowHash(row: AuditRow): string | null {
  return row.selfHash ?? row.hash ?? null;
}

function evidenceState(hashRows: number, rowCount: number): EvidenceState {
  if (rowCount === 0) return "empty";
  if (hashRows === 0) return "missing";
  if (hashRows < rowCount) return "partial";
  return "present";
}

const EVIDENCE_COPY: Record<EvidenceState, { title: string; body: string }> = {
  empty: {
    title: "Awaiting live audit rows",
    body: "The chain explorer is connected to the audit feed. Once real gateway rows arrive, this panel will show the hashes returned by the backend.",
  },
  missing: {
    title: "Hash evidence unavailable in this response",
    body: "The browser is showing real audit rows, but this page did not receive self-hash values for the current view. No chain verification is implied.",
  },
  partial: {
    title: "Partial hash evidence in current view",
    body: "Some returned rows include chain hashes and some do not. Treat this as an evidence display, not a full verifier pass.",
  },
  present: {
    title: "Hash evidence present in current view",
    body: "Every row currently visible includes a returned self-hash. Full tamper verification still belongs to the backend or CLI verifier, not this browser panel.",
  },
};

function HeroStat({
  label,
  value,
  mono,
  tone,
}: {
  label: string;
  value: string;
  mono?: boolean;
  tone?: "success" | "warning";
}) {
  return (
    <div className="rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--card)]/82 p-4">
      <p className="font-mono text-[10px] font-semibold uppercase tracking-[0.14em] text-[var(--muted-foreground)]">
        {label}
      </p>
      <p
        className={cn(
          "mt-2 truncate text-xl font-semibold text-[var(--foreground)]",
          mono ? "font-mono text-sm tabular-nums" : "tabular-nums",
          tone === "success" ? "text-[var(--success)]" : null,
          tone === "warning" ? "text-[var(--warning)]" : null,
        )}
        title={value}
      >
        {value}
      </p>
    </div>
  );
}

function MetricPill({ label, value }: { label: string; value: number }) {
  return (
    <span className="inline-flex items-center gap-1 rounded-full border border-[var(--border)] bg-[var(--card)]/70 px-3 py-1 font-medium">
      <span>{label}</span>
      <span className="font-mono text-[var(--foreground)]">{value}</span>
    </span>
  );
}

function abbreviateHash(hash: string): string {
  if (hash.length <= 18) return hash;
  return `${hash.slice(0, 10)}...${hash.slice(-6)}`;
}

function formatUsd(amount: number): string {
  if (amount === 0) return "$0";
  return new Intl.NumberFormat("en-US", {
    style: "currency",
    currency: "USD",
    maximumFractionDigits: amount < 1 ? 3 : 2,
  }).format(amount);
}
