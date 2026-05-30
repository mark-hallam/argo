"use client";

import {
  CheckCircle2,
  CircleDashed,
  CircleSlash,
  Clock3,
  CornerDownRight,
  Loader2,
  Users,
} from "lucide-react";
import type { ComponentType } from "react";
import {
  teamRunLabel,
  type OrchestrationRun,
  type OrchestrationStatus,
  type TeamRunGroup,
} from "@/lib/api/orchestration";
import { cn } from "@/lib/utils";

/**
 * Renders the Supervisor team runs as lead-with-children trees: each
 * lead teammate sits in a brand-gradient banner with the child
 * delegations it coordinated listed beneath, every child carrying its
 * own live status. This is the visible "watch the team work" beat from
 * design plan 02 (Phase 3) — driven by the real
 * `GET /api/v1/orchestration/runs` rows (or the auth-bypass mock),
 * grouped via {@link groupTeamRuns}.
 */
export function TeamRunTree({ teams }: { teams: TeamRunGroup[] }) {
  if (teams.length === 0) return null;
  return (
    <section className="space-y-4" data-testid="team-run-tree">
      <div className="flex items-center gap-2">
        <span
          className="inline-flex size-8 items-center justify-center rounded-[var(--radius-sm)] text-white"
          style={{ background: "var(--brand-gradient)" }}
          aria-hidden
        >
          <Users className="size-4" />
        </span>
        <div>
          <h2
            className="text-base tracking-tight text-[var(--foreground)]"
            style={{ fontFamily: "var(--font-audiowide)" }}
          >
            Team runs
          </h2>
          <p className="text-xs text-[var(--muted-foreground)]">
            A lead teammate fanning work out to the team — watch each delegation resolve
          </p>
        </div>
      </div>
      <div className="space-y-4">
        {teams.map((team) => (
          <TeamCard key={team.key} team={team} />
        ))}
      </div>
    </section>
  );
}

function TeamCard({ team }: { team: TeamRunGroup }) {
  const done = team.children.filter((c) => c.status === "done").length;
  return (
    <article
      className="overflow-hidden rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--background)]"
      data-testid="team-run-card"
    >
      <header
        className="flex flex-col gap-1 px-4 py-3 text-white sm:flex-row sm:items-center sm:justify-between"
        style={{ background: "var(--brand-gradient)" }}
      >
        <div className="min-w-0">
          <div className="flex items-center gap-2">
            <Users className="size-4 shrink-0" aria-hidden />
            <span className="truncate text-sm font-semibold">
              {team.lead ? team.lead.worker : "Delegated steps"}
            </span>
            {team.lead ? (
              <span className="rounded-full bg-white/20 px-2 py-0.5 text-[10px] font-semibold uppercase tracking-wide">
                Lead
              </span>
            ) : null}
          </div>
          <p className="mt-0.5 truncate text-xs text-white/85">{team.title}</p>
        </div>
        <div className="flex items-center gap-2 text-xs text-white/90">
          {team.lead ? <StatusChip status={team.lead.status} onDark /> : null}
          <span className="rounded-full bg-white/15 px-2 py-0.5 font-medium">
            {done}/{team.children.length} done
          </span>
        </div>
      </header>

      <ul className="divide-y divide-[var(--border)]" data-testid="team-run-children">
        {team.children.length === 0 ? (
          <li className="px-4 py-3 text-xs text-[var(--muted-foreground)]">
            No child delegations attributed to this lead yet.
          </li>
        ) : (
          team.children.map((child) => <ChildRow key={child.id} run={child} />)
        )}
      </ul>
    </article>
  );
}

function ChildRow({ run }: { run: OrchestrationRun }) {
  return (
    <li className="flex items-center gap-3 px-4 py-2.5" data-testid="team-run-child">
      <CornerDownRight className="size-4 shrink-0 text-[var(--muted-foreground)]" aria-hidden />
      <div className="min-w-0 flex-1">
        <div className="truncate text-sm font-medium text-[var(--foreground)]">
          {teamRunLabel(run)}
        </div>
        <div className="truncate text-xs text-[var(--muted-foreground)]">{run.worker}</div>
      </div>
      <StatusChip status={run.status} />
    </li>
  );
}

const STATUS_META: Record<
  OrchestrationStatus,
  { label: string; icon: ComponentType<{ className?: string; "aria-hidden"?: boolean }>; tone: string }
> = {
  running: { label: "Running", icon: Loader2, tone: "text-[var(--brand-primary)]" },
  queued: { label: "Queued", icon: CircleDashed, tone: "text-[var(--muted-foreground)]" },
  waiting: { label: "Waiting", icon: Clock3, tone: "text-amber-600" },
  blocked: { label: "Blocked", icon: CircleSlash, tone: "text-red-600" },
  done: { label: "Done", icon: CheckCircle2, tone: "text-emerald-600" },
};

function StatusChip({ status, onDark = false }: { status: OrchestrationStatus; onDark?: boolean }) {
  const meta = STATUS_META[status];
  const Icon = meta.icon;
  return (
    <span
      className={cn(
        "inline-flex h-7 items-center gap-1.5 rounded-[var(--radius-sm)] border px-2 text-xs font-medium",
        onDark
          ? "border-white/30 bg-white/15 text-white"
          : cn("border-[var(--border)] bg-[var(--card)]", meta.tone),
      )}
      title={meta.label}
    >
      <Icon className={cn("size-3.5", status === "running" && "animate-spin")} aria-hidden />
      <span>{meta.label}</span>
    </span>
  );
}
