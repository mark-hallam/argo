"use client";

import Link from "next/link";
import { useParams, useSearchParams } from "next/navigation";
import { AlertCircle, Loader2 } from "lucide-react";
import { Breadcrumb, BreadcrumbItem, BreadcrumbSeparator } from "@/components/ui/Breadcrumb";
import { DiffView } from "@/components/skycot/canvas/DiffView";
import { SkillIrNotFoundError } from "@/lib/api/skill-ir";
import { useSkillIr } from "@/lib/hooks/use-skill-ir";

export default function SkillCanvasDiffPage() {
  const routeParams = useParams<{ workerId: string; skillId: string }>();
  const search = useSearchParams();
  const workerId = routeParams?.workerId ?? "";
  const skillId = routeParams?.skillId ?? "";
  const leftVersion = search?.get("from") ?? undefined;
  const rightVersion = search?.get("to") ?? search?.get("version") ?? undefined;

  const leftQuery = useSkillIr(skillId, leftVersion);
  const rightQuery = useSkillIr(skillId, rightVersion);
  const isPending = leftQuery.isPending || rightQuery.isPending;
  const error = leftQuery.error ?? rightQuery.error;

  return (
    <div data-testid="skill-canvas-diff-page" className="space-y-4">
      <Breadcrumb>
        <BreadcrumbItem href={`/workers/${workerId}/skills`}>Skills</BreadcrumbItem>
        <BreadcrumbSeparator />
        <BreadcrumbItem href={`/workers/${workerId}/skills/${skillId}/canvas`}>
          {skillId}
        </BreadcrumbItem>
        <BreadcrumbSeparator />
        <BreadcrumbItem current>Diff</BreadcrumbItem>
      </Breadcrumb>

      {isPending ? (
        <LoadingState />
      ) : error instanceof SkillIrNotFoundError ? (
        <ErrorState message={error.message} />
      ) : error ? (
        <ErrorState message={error.message || "Couldn't load skill diff"} />
      ) : leftQuery.data && rightQuery.data ? (
        <>
          <DiffView
            leftGraph={leftQuery.data.graph}
            rightGraph={rightQuery.data.graph}
            leftVersion={leftQuery.data.version}
            rightVersion={rightQuery.data.version}
          />
          <Link
            href={`/workers/${workerId}/skills/${skillId}/canvas?version=${encodeURIComponent(rightQuery.data.version)}`}
            className="inline-flex text-sm font-medium text-[var(--primary)] underline-offset-4 hover:underline"
          >
            Back to canvas
          </Link>
        </>
      ) : null}
    </div>
  );
}

function LoadingState() {
  return (
    <div
      data-testid="skill-canvas-diff-loading"
      className="flex h-[520px] min-h-[420px] w-full items-center justify-center rounded-[var(--radius-lg)] border border-dashed border-[var(--border)] bg-[var(--card)]/50"
    >
      <div className="flex flex-col items-center gap-2 text-[var(--muted-foreground)]">
        <Loader2 className="size-6 animate-spin" />
        <p className="text-sm">Loading version diff…</p>
      </div>
    </div>
  );
}

function ErrorState({ message }: { message: string }) {
  return (
    <div
      data-testid="skill-canvas-diff-error"
      className="rounded-[var(--radius-lg)] border border-[var(--destructive)]/30 bg-[var(--destructive)]/5 px-6 py-10 text-center space-y-3"
    >
      <div className="mx-auto grid size-10 place-items-center rounded-full bg-[var(--destructive)]/10 text-[var(--destructive)]">
        <AlertCircle />
      </div>
      <p className="text-sm text-[var(--foreground)]">{message}</p>
    </div>
  );
}
