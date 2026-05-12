"use client";

import { MoreVertical } from "lucide-react";
import dynamic from "next/dynamic";
import { useState } from "react";
import { Avatar, type WorkerStatus as AvatarStatus } from "@/components/ui/Avatar";
import { Button } from "@/components/ui/Button";
import { StatusPill } from "@/components/ui/StatusPill";
import { cn } from "@/lib/utils";
import {
  WORKER_STATUS_PILL_LABEL,
  WORKER_STATUS_TONE,
  type Worker,
  type WorkerStatus,
} from "@/lib/types/worker";
import type { WorkerHeroAction } from "@/components/skycot/WorkerOverflowMenu";

export type { WorkerHeroAction } from "@/components/skycot/WorkerOverflowMenu";

/**
 * Hero card sitting above the WorkerTabs strip. Displays the
 * worker's identity (avatar + name + role tagline + org placement),
 * operational status pill, today's KPI summary, and an overflow
 * menu for delegator-level actions (pause / disable / archive /
 * export / view-as-operator).
 *
 * The overflow menu (which pulls in Radix DropdownMenu) is
 * lazy-loaded via `next/dynamic` — same pattern as
 * `NotificationsPopover` in E1-c. Keeps the First-Load JS delta
 * for `/workers/<id>/<tab>` lean; the menu chunk only ships once
 * the user opens it.
 *
 * v1 posture: department + team + kpiToday + permissions are all
 * optional fields on `Worker` (the Track I `mission-enterprise-org-
 * hierarchy` brief promotes them into first-class entities). When a
 * field is absent the hero falls back gracefully.
 */

const LazyOverflowMenu = dynamic(
  () => import("./WorkerOverflowMenu").then((m) => m.WorkerOverflowMenu),
  { ssr: false },
);

function workerStatusToAvatar(status: WorkerStatus): AvatarStatus {
  switch (status) {
    case "working":
      return "working";
    case "waiting":
      return "waiting";
    case "blocked":
      return "blocked";
    case "idle":
    case "paused":
      return "idle";
  }
}

export interface WorkerHeroProps {
  worker: Worker;
  onAction?: (action: WorkerHeroAction) => void;
  className?: string;
}

export function WorkerHero({ worker, onAction, className }: WorkerHeroProps) {
  const orgLine = formatOrgLine(worker);
  const kpiLine = worker.kpiToday
    ? `${worker.kpiToday.value} ${worker.kpiToday.label}`
    : null;
  const [menuOpen, setMenuOpen] = useState(false);

  const triggerButton = (
    <Button
      variant="ghost"
      size="icon"
      aria-label="Worker actions"
      data-testid="worker-overflow"
      onClick={() => setMenuOpen(true)}
    >
      <MoreVertical />
    </Button>
  );

  return (
    <section
      data-testid="worker-hero"
      className={cn(
        "rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--card)] text-[var(--card-foreground)] shadow-sm",
        "px-5 py-5 sm:px-6 sm:py-6",
        className,
      )}
    >
      <div className="flex items-start gap-4 sm:gap-5">
        <Avatar
          size="lg"
          alt={worker.name}
          src={worker.avatarUrl}
          status={workerStatusToAvatar(worker.status)}
        />

        <div className="min-w-0 flex-1 space-y-2">
          <div className="flex flex-wrap items-center gap-x-3 gap-y-1.5">
            <h1
              data-testid="worker-name"
              className="font-semibold text-base text-[var(--foreground)] leading-tight"
            >
              {worker.name}
            </h1>
            <StatusPill
              data-testid="worker-status-pill"
              tone={WORKER_STATUS_TONE[worker.status]}
              size="sm"
            >
              {WORKER_STATUS_PILL_LABEL[worker.status]}
            </StatusPill>
          </div>

          <p data-testid="worker-role" className="text-sm text-[var(--muted-foreground)]">
            {worker.role.tagline}
          </p>

          {orgLine ? (
            <p
              data-testid="worker-org-line"
              className="text-xs text-[var(--muted-foreground)]"
            >
              {orgLine}
            </p>
          ) : null}

          {kpiLine ? (
            <p data-testid="worker-kpi" className="text-sm text-[var(--foreground)]/80">
              {kpiLine}
            </p>
          ) : null}
        </div>

        {menuOpen ? (
          <LazyOverflowMenu
            worker={worker}
            trigger={triggerButton}
            defaultOpen
            onOpenChange={(open) => {
              if (!open) setMenuOpen(false);
            }}
            onAction={onAction}
          />
        ) : (
          triggerButton
        )}
      </div>
    </section>
  );
}

function formatOrgLine(worker: Worker): string | null {
  const dept = worker.department?.name;
  const team = worker.team?.name;
  if (dept && team) return `${dept} · ${team}`;
  return dept ?? team ?? null;
}
