"use client";

import Link from "next/link";
import { ArrowDown, ArrowRight } from "lucide-react";
import { Avatar } from "@/components/ui/Avatar";
import { StatusPill } from "@/components/ui/StatusPill";
import type { DelegationStep } from "@/lib/api/audit-detail";

export interface DelegationChainCardProps {
  chain: DelegationStep[];
}

export function DelegationChainCard({ chain }: DelegationChainCardProps) {
  if (chain.length === 0) return null;

  return (
    <section
      data-testid="audit-delegation-chain"
      className="overflow-hidden rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--card)]"
    >
      <header className="border-b border-[var(--border)] bg-[linear-gradient(135deg,var(--accent)_0%,var(--card)_60%,var(--muted)_100%)] px-5 py-4">
        <p className="text-[11px] font-semibold uppercase tracking-wider text-[var(--muted-foreground)]">
          Delegation chain
        </p>
        <h2 className="mt-1 text-sm font-semibold text-[var(--foreground)]">
          Who handed this work to whom
        </h2>
      </header>

      <ol className="space-y-0 px-5 py-4">
        {chain.map((step, idx) => (
          <DelegationStepRow
            key={`${step.actionId}-${idx}`}
            step={step}
            index={idx}
            showArrow={idx < chain.length - 1}
          />
        ))}
      </ol>
    </section>
  );
}

function DelegationStepRow({
  step,
  index,
  showArrow,
}: {
  step: DelegationStep;
  index: number;
  showArrow: boolean;
}) {
  return (
    <li data-testid={`delegation-chain-step-${index}`} className="relative">
      <div className="grid grid-cols-[auto_1fr_auto] items-center gap-3 rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--background)]/50 px-3 py-2.5">
        <Link
          href={`/workers/${encodeURIComponent(step.workerId)}/overview`}
          data-testid={`delegation-chain-worker-${index}`}
          className="inline-flex min-w-0 items-center gap-2 rounded-[var(--radius-sm)] text-left outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)]"
        >
          <Avatar
            size="xs"
            alt={step.workerName}
            fallback={initials(step.workerName)}
          />
          <span className="min-w-0">
            <span className="block truncate text-sm font-medium text-[var(--foreground)]">
              {step.workerName}
            </span>
            <span className="block truncate font-mono text-[11px] text-[var(--muted-foreground)]">
              {shortAction(step.actionId)}
            </span>
          </span>
        </Link>

        <div className="min-w-0 text-xs text-[var(--muted-foreground)]">
          {formatTimestamp(step.ts)}
        </div>

        <StatusPill tone={toneForRole(step.role)} size="sm">
          {ROLE_LABEL[step.role]}
        </StatusPill>
      </div>

      {showArrow ? (
        <div
          aria-hidden
          data-testid={`delegation-chain-arrow-${index}`}
          className="flex h-6 items-center pl-5 text-[var(--muted-foreground)]"
        >
          <ArrowDown className="size-3.5" />
          <ArrowRight className="ml-1 hidden size-3.5 sm:block" />
        </div>
      ) : null}
    </li>
  );
}

const ROLE_LABEL: Record<DelegationStep["role"], string> = {
  originator: "Originator",
  delegated_to: "Delegated",
  executed: "Executed",
};

function toneForRole(role: DelegationStep["role"]) {
  if (role === "originator") return "info";
  if (role === "executed") return "success";
  return "warning";
}

function initials(name: string): string {
  const parts = name.trim().split(/\s+/).filter(Boolean);
  if (parts.length === 0) return "??";
  if (parts.length === 1) return parts[0].slice(0, 2).toUpperCase();
  return `${parts[0][0]}${parts[parts.length - 1][0]}`.toUpperCase();
}

function shortAction(actionId: string): string {
  if (actionId.length <= 14) return actionId;
  return `${actionId.slice(0, 6)}...${actionId.slice(-4)}`;
}

function formatTimestamp(ts: string): string {
  const date = Date.parse(ts);
  if (Number.isNaN(date)) return ts;
  return new Intl.DateTimeFormat("en-US", {
    month: "short",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
  }).format(date);
}
