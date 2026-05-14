"use client";

import Link from "next/link";
import { useParams, useSearchParams } from "next/navigation";
import { AlertCircle, Loader2 } from "lucide-react";
import { Breadcrumb, BreadcrumbItem, BreadcrumbSeparator } from "@/components/ui/Breadcrumb";
import { CodeView } from "@/components/skycot/canvas/CodeView";
import { SkillIrNotFoundError } from "@/lib/api/skill-ir";
import { useSkillIr } from "@/lib/hooks/use-skill-ir";

export default function SkillCodePage() {
  const routeParams = useParams<{ workerId: string; skillId: string }>();
  const search = useSearchParams();
  const workerId = routeParams?.workerId ?? "";
  const skillId = routeParams?.skillId ?? "";
  const version = search?.get("version") ?? undefined;
  const irQuery = useSkillIr(skillId, version);

  return (
    <div data-testid="skill-code-page" className="space-y-4">
      <Breadcrumb>
        <BreadcrumbItem href={`/workers/${workerId}/skills`}>Skills</BreadcrumbItem>
        <BreadcrumbSeparator />
        <BreadcrumbItem href={`/workers/${workerId}/skills/${skillId}/canvas`}>
          {skillId}
        </BreadcrumbItem>
        <BreadcrumbSeparator />
        <BreadcrumbItem current>Code</BreadcrumbItem>
      </Breadcrumb>

      {irQuery.isPending ? (
        <LoadingState />
      ) : irQuery.error instanceof SkillIrNotFoundError ? (
        <ErrorState
          message={irQuery.error.message}
          actionHref={`/workers/${workerId}/skills/${skillId}/canvas`}
        />
      ) : irQuery.error ? (
        <ErrorState message={irQuery.error.message || "Couldn't load skill code"} />
      ) : irQuery.data ? (
        <CodeView
          graph={irQuery.data.graph}
          skillId={skillId}
          version={irQuery.data.version}
        />
      ) : null}
    </div>
  );
}

function LoadingState() {
  return (
    <div
      data-testid="skill-code-loading"
      className="flex h-[520px] min-h-[420px] w-full items-center justify-center rounded-[var(--radius-lg)] border border-dashed border-[var(--border)] bg-[var(--card)]/50"
    >
      <div className="flex flex-col items-center gap-2 text-[var(--muted-foreground)]">
        <Loader2 className="size-6 animate-spin" />
        <p className="text-sm">Loading skill code...</p>
      </div>
    </div>
  );
}

function ErrorState({
  message,
  actionHref,
}: {
  message: string;
  actionHref?: string;
}) {
  return (
    <div
      data-testid="skill-code-error"
      className="rounded-[var(--radius-lg)] border border-[var(--destructive)]/30 bg-[var(--destructive)]/5 px-6 py-10 text-center space-y-3"
    >
      <div className="mx-auto grid size-10 place-items-center rounded-full bg-[var(--destructive)]/10 text-[var(--destructive)]">
        <AlertCircle />
      </div>
      <p className="text-sm text-[var(--foreground)]">{message}</p>
      {actionHref ? (
        <Link
          href={actionHref}
          className="inline-flex text-sm font-medium text-[var(--primary)] underline-offset-4 hover:underline"
        >
          Back to canvas
        </Link>
      ) : null}
    </div>
  );
}
