"use client";

import { UserX } from "lucide-react";
import { buttonVariants } from "@/components/ui/Button";

export interface WorkerNotFoundProps {
  workerId: string;
}

/**
 * Inside-AppShell empty state shown when a worker id doesn't
 * resolve (gateway 404, mock fallback missed, or cross-tenant
 * access refused). Includes a CTA back to the Team page so the
 * operator can pick another worker.
 */
export function WorkerNotFound({ workerId }: WorkerNotFoundProps) {
  return (
    <div
      data-testid="worker-not-found"
      className="rounded-[var(--radius-lg)] border border-dashed border-[var(--border)] py-16 px-6 text-center space-y-4"
    >
      <div className="mx-auto grid size-12 place-items-center rounded-full bg-[var(--muted)] text-[var(--muted-foreground)]">
        <UserX />
      </div>
      <div className="space-y-1">
        <h1 className="text-base font-semibold text-[var(--foreground)]">
          Worker not found
        </h1>
        <p className="text-sm text-[var(--muted-foreground)]">
          The worker &lsquo;{workerId}&rsquo; doesn&apos;t exist or you don&apos;t have access.
        </p>
      </div>
      <div className="pt-2">
        <a href="/team" className={buttonVariants({ variant: "secondary", size: "md" })}>
          Back to team
        </a>
      </div>
    </div>
  );
}
