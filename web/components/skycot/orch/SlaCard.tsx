"use client";

import { AlertTriangle, Clock3, ListChecks, Play } from "lucide-react";
import type { OrchestrationSummary } from "@/lib/api/orchestration";

export function SlaCard({ summary }: { summary: OrchestrationSummary }) {
  const items = [
    { label: "Running", value: summary.running, icon: Play },
    { label: "Queued", value: summary.queued, icon: ListChecks },
    { label: "Breached", value: summary.breached, icon: AlertTriangle },
    { label: "Total", value: summary.total, icon: Clock3 },
  ];
  return (
    <div className="grid grid-cols-2 gap-3 lg:grid-cols-4" data-testid="orchestration-sla-cards">
      {items.map(({ label, value, icon: Icon }) => (
        <div
          key={label}
          className="rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--card)] p-4"
        >
          <div className="flex items-center justify-between gap-3">
            <span className="text-sm font-medium text-[var(--muted-foreground)]">{label}</span>
            <Icon className="size-4 text-[var(--muted-foreground)]" aria-hidden />
          </div>
          <div className="mt-3 text-2xl font-semibold text-[var(--foreground)]">{value}</div>
        </div>
      ))}
    </div>
  );
}
