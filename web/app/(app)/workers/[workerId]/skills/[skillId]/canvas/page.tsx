"use client";

import Link from "next/link";
import { useParams, useSearchParams } from "next/navigation";
import { AlertCircle, Loader2, Sparkles, Tv } from "lucide-react";
import { Breadcrumb, BreadcrumbItem, BreadcrumbSeparator } from "@/components/ui/Breadcrumb";
import { SkillCanvas } from "@/components/skycot/canvas/SkillCanvas";
import { SkillIrNotFoundError } from "@/lib/api/skill-ir";
import { useSkillIr } from "@/lib/hooks/use-skill-ir";
import { cn } from "@/lib/utils";

/**
 * Read-only Skill Canvas route — `/workers/<workerId>/skills/<skillId>/canvas`.
 *
 * Composes the L4 `SkillCanvas` over `useSkillIr(skillId, version?)`.
 * Renders a skeleton while loading, surfaces a typed error state on
 * server errors, and falls through to the "teach this skill first"
 * empty-state CTA when the gateway responds 404 (the
 * `SkillIrNotFoundError` branch).
 *
 * Sensible deviation from L4 brief: the route segment is `[workerId]`
 * (matching the existing `(app)/workers/[workerId]/...` tree from E0
 * / E4) rather than `[id]`. The `[id]` form would have introduced a
 * parallel parameter name in the same tree — drift the swarm protocol
 * (DRIFT_PREVENTION.md "No parallel abstractions") explicitly warns
 * against.
 */
export default function SkillCanvasPage() {
  const routeParams = useParams<{ workerId: string; skillId: string }>();
  const search = useSearchParams();
  const workerId = routeParams?.workerId ?? "";
  const skillId = routeParams?.skillId ?? "";
  const version = search?.get("version") ?? undefined;

  const irQuery = useSkillIr(skillId, version);

  return (
    <div data-testid="skill-canvas-page" className="space-y-4">
      <CanvasBreadcrumb workerId={workerId} skillId={skillId} />

      {irQuery.isPending ? (
        <LoadingState />
      ) : irQuery.error instanceof SkillIrNotFoundError ? (
        <EmptyState workerId={workerId} skillId={skillId} />
      ) : irQuery.error ? (
        <ErrorState
          message={irQuery.error.message || "Couldn't load skill"}
          onRetry={() => irQuery.refetch()}
        />
      ) : irQuery.data ? (
        <>
          <SkillCanvas graph={irQuery.data.graph} />
          <p
            data-testid="canvas-source-line"
            className="text-xs text-[var(--muted-foreground)]"
          >
            {sourceLine(irQuery.data.source, irQuery.data.version)}
          </p>
        </>
      ) : null}
    </div>
  );
}

function CanvasBreadcrumb({
  workerId,
  skillId,
}: {
  workerId: string;
  skillId: string;
}) {
  return (
    <Breadcrumb>
      <BreadcrumbItem href={`/workers/${workerId}/skills`}>Skills</BreadcrumbItem>
      <BreadcrumbSeparator />
      <BreadcrumbItem current>{skillId}</BreadcrumbItem>
    </Breadcrumb>
  );
}

function LoadingState() {
  return (
    <div
      data-testid="skill-canvas-loading"
      className="flex h-[640px] min-h-[480px] w-full items-center justify-center rounded-[var(--radius-lg)] border border-dashed border-[var(--border)] bg-[var(--card)]/50"
    >
      <div className="flex flex-col items-center gap-2 text-[var(--muted-foreground)]">
        <Loader2 className="size-6 animate-spin" />
        <p className="text-sm">Loading skill canvas…</p>
      </div>
    </div>
  );
}

interface EmptyStateProps {
  workerId: string;
  skillId: string;
}

export function EmptyState({ workerId, skillId }: EmptyStateProps) {
  return (
    <div
      data-testid="skill-canvas-empty-state"
      role="region"
      aria-label="Skill not yet learned"
      className="rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--card)] px-6 py-10 text-center space-y-4"
    >
      <p className="text-base text-[var(--foreground)]">
        Worker hasn&apos;t learned this skill yet.
      </p>
      <p className="text-sm text-[var(--muted-foreground)]">
        Skill <code>{skillId}</code> has no IR versions on file.
      </p>
      <div className="flex flex-wrap items-center justify-center gap-3 pt-2">
        <Link
          data-testid="empty-state-teach"
          href={`/workers/${workerId}/skills`}
          className={primaryActionClass}
        >
          <Tv className="size-4" aria-hidden="true" />
          Teach via observation
        </Link>
        <Link
          data-testid="empty-state-ghostwriter"
          href={`/workers/${workerId}/skills/new`}
          className={secondaryActionClass}
        >
          <Sparkles className="size-4" aria-hidden="true" />
          Generate from prompt
        </Link>
      </div>
    </div>
  );
}

function ErrorState({
  message,
  onRetry,
}: {
  message: string;
  onRetry: () => void;
}) {
  return (
    <div
      data-testid="skill-canvas-error"
      className="rounded-[var(--radius-lg)] border border-[var(--destructive)]/30 bg-[var(--destructive)]/5 px-6 py-10 text-center space-y-3"
    >
      <div className="mx-auto grid size-10 place-items-center rounded-full bg-[var(--destructive)]/10 text-[var(--destructive)]">
        <AlertCircle />
      </div>
      <p className="text-sm text-[var(--foreground)]">{message}</p>
      <button
        type="button"
        onClick={onRetry}
        className="text-sm font-medium text-[var(--primary)] underline-offset-4 hover:underline"
      >
        Try again
      </button>
    </div>
  );
}

function sourceLine(
  source: "gateway" | "mock-offline" | "mock-bypass",
  version: string,
): string {
  if (source === "gateway") return `Version ${version} · live gateway`;
  if (source === "mock-offline") {
    return `Version ${version} · sample IR (gateway endpoint pending L4-gw)`;
  }
  return `Version ${version} · sample IR (auth-bypass dev mode)`;
}

const primaryActionClass = cn(
  "inline-flex items-center gap-2 rounded-[var(--radius-md)] bg-[var(--primary)] px-4 py-2 text-sm font-medium text-[var(--primary-foreground)] shadow-sm hover:bg-[var(--primary)]/90",
  "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)] focus-visible:ring-offset-2 focus-visible:ring-offset-[var(--background)]",
);

const secondaryActionClass = cn(
  "inline-flex items-center gap-2 rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--background)] px-4 py-2 text-sm font-medium text-[var(--foreground)] hover:bg-[var(--accent)]/40",
  "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)] focus-visible:ring-offset-2 focus-visible:ring-offset-[var(--background)]",
);
