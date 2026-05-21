"use client";

import { use, useCallback, useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { Breadcrumb, BreadcrumbItem, BreadcrumbSeparator } from "@/components/ui/Breadcrumb";
import { ProposeWorkflowChat } from "@/components/skycot/ProposeWorkflowChat";
import { fetchWorker, WorkerNotFoundError } from "@/lib/api/workers";

/**
 * `/workers/<workerId>/responsibilities/new`
 *
 * Author surface for the conversational "add a responsibility in plain
 * English" flow. Streams a workflow-TOML proposal, lets the operator
 * approve / edit / cancel. On approve, posts to the P1 #6 gateway
 * workflow-edit API.
 */
export default function NewResponsibilityPage({
  params,
}: {
  params: Promise<{ workerId: string }>;
}) {
  const { workerId } = use(params);
  const router = useRouter();
  const [verticalId, setVerticalId] = useState<string | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const worker = await fetchWorker(workerId);
        if (cancelled) return;
        setVerticalId(worker.role.vertical);
      } catch (err) {
        if (cancelled) return;
        if (err instanceof WorkerNotFoundError) {
          setLoadError(`Worker "${workerId}" not found.`);
        } else {
          setLoadError("Couldn't load this worker. Please try again.");
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [workerId]);

  const onCancel = useCallback(() => {
    router.push(`/workers/${workerId}/overview`);
  }, [router, workerId]);

  const onApproved = useCallback(
    (workflowName: string) => {
      const path = `/workers/${workerId}/overview?responsibility=${encodeURIComponent(workflowName)}`;
      router.push(path);
    },
    [router, workerId],
  );

  return (
    <div data-testid="propose-responsibility-page" className="space-y-4 max-w-3xl">
      <Breadcrumb>
        <BreadcrumbItem href={`/workers/${workerId}/overview`}>Worker</BreadcrumbItem>
        <BreadcrumbSeparator />
        <BreadcrumbItem href={`/workers/${workerId}/overview`}>Responsibilities</BreadcrumbItem>
        <BreadcrumbSeparator />
        <BreadcrumbItem current>New</BreadcrumbItem>
      </Breadcrumb>

      <header className="space-y-1">
        <h2 className="text-xl font-semibold text-[var(--foreground)]">
          Describe a new responsibility
        </h2>
        <p className="text-sm text-[color:var(--muted-foreground)]">
          Tell your teammate the recurring duty in plain English. We&apos;ll
          draft a workflow TOML for you to approve, edit, or cancel.
        </p>
      </header>

      {loadError ? (
        <p
          role="alert"
          className="rounded-[var(--radius-md)] border border-[color:var(--destructive)]/40 bg-[color:var(--destructive)]/10 px-3 py-2 text-sm text-[color:var(--destructive)]"
        >
          {loadError}
        </p>
      ) : verticalId === null ? (
        <p className="text-sm text-[color:var(--muted-foreground)]" aria-live="polite">
          Loading worker…
        </p>
      ) : (
        <ProposeWorkflowChat
          workerId={workerId}
          verticalId={verticalId}
          approveEndpoint={`/api/v1/verticals/${encodeURIComponent(verticalId)}/workflows`}
          onCancel={onCancel}
          onApproved={onApproved}
        />
      )}
    </div>
  );
}
