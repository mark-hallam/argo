"use client";

import * as DialogPrimitive from "@radix-ui/react-dialog";
import { X } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { ProposeWorkflowChat } from "@/components/skycot/ProposeWorkflowChat";
import { useTeam } from "@/lib/hooks/use-team";
import { useToast } from "@/lib/ui/use-toast";
import type { Worker } from "@/lib/types/worker";
import { cn } from "@/lib/utils";

/**
 * WorkflowProposeModal — dialog wrapper around `ProposeWorkflowChat`.
 *
 * Opened from the `/workflows` catalog "Propose new" CTA and from the
 * `Edit in chat` button on the `/workflows/[v]/[n]` viewer. When
 * `editingWorkflow` is provided, the chat is anchored to that
 * workflow's vertical and the modal title shifts from "Propose new
 * workflow" to "Edit <name> in chat".
 *
 * Worker + vertical pickers live at the top of the modal. The vertical
 * field auto-fills from the chosen worker; the operator can override
 * it manually for cross-vertical cases. On approval, the modal closes
 * and a sonner toast confirms the proposal landed.
 */

export interface ProposeModalEditingContext {
  /** Workflow name being edited (passed to chat for context). */
  workflowName: string;
  /** Vertical the workflow lives in — pins the vertical picker. */
  verticalId: string;
}

export interface WorkflowProposeModalProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** When set, the modal opens in "Edit in chat" mode. */
  editingWorkflow?: ProposeModalEditingContext | null;
  /** Test seam — overrides the team fetch with a fixed worker list. */
  workersOverride?: Worker[];
  /** Test seam — passed straight to the chat component. */
  fetchImpl?: typeof fetch;
}

function workerVertical(worker: Worker | undefined): string {
  return worker?.role.vertical ?? "";
}

export function WorkflowProposeModal({
  open,
  onOpenChange,
  editingWorkflow,
  workersOverride,
  fetchImpl,
}: WorkflowProposeModalProps) {
  const toast = useToast();
  const teamQuery = useTeam();
  const workers = workersOverride ?? teamQuery.data?.workers ?? [];

  const editingVertical = editingWorkflow?.verticalId ?? null;

  const initialWorkerId = useMemo(() => {
    if (editingVertical) {
      const match = workers.find((w) => w.role.vertical === editingVertical);
      if (match) return match.id;
    }
    return workers[0]?.id ?? "";
  }, [workers, editingVertical]);

  const [workerId, setWorkerId] = useState<string>(initialWorkerId);
  const [verticalId, setVerticalId] = useState<string>(
    editingVertical ?? workerVertical(workers[0]),
  );

  useEffect(() => {
    if (!open) return;
    setWorkerId(initialWorkerId);
    setVerticalId(
      editingVertical ?? workerVertical(workers.find((w) => w.id === initialWorkerId)),
    );
  }, [open, initialWorkerId, editingVertical, workers]);

  function handleWorkerChange(nextId: string) {
    setWorkerId(nextId);
    if (editingVertical) return;
    const next = workers.find((w) => w.id === nextId);
    if (next) setVerticalId(next.role.vertical);
  }

  function handleApproved(workflowName: string) {
    toast.success("Proposed workflow lands when reviewed", {
      description: editingWorkflow
        ? `Edits to ${workflowName} queued for review.`
        : `${workflowName} is now in the review queue.`,
    });
    onOpenChange(false);
  }

  const title = editingWorkflow
    ? `Edit ${editingWorkflow.workflowName} in chat`
    : "Propose a new workflow";
  const description = editingWorkflow
    ? "Describe the change you want in plain English — the assistant will draft a diff against the current TOML."
    : "Describe the responsibility in plain English. The assistant drafts a workflow TOML you can approve, edit, or cancel.";

  const verticalOptions = useMemo(() => {
    const seen = new Set<string>();
    const out: string[] = [];
    for (const w of workers) {
      const v = w.role.vertical;
      if (v && !seen.has(v)) {
        seen.add(v);
        out.push(v);
      }
    }
    if (verticalId && !seen.has(verticalId)) out.unshift(verticalId);
    return out;
  }, [workers, verticalId]);

  const approveEndpoint = `/api/v1/verticals/${encodeURIComponent(verticalId)}/workflows`;
  const canRender = workerId.length > 0 && verticalId.length > 0;

  return (
    <DialogPrimitive.Root open={open} onOpenChange={onOpenChange}>
      <DialogPrimitive.Portal>
        <DialogPrimitive.Overlay
          className={cn(
            "fixed inset-0 z-40 bg-black/40 backdrop-blur-sm",
            "data-[state=open]:animate-in data-[state=closed]:animate-out",
          )}
        />
        <DialogPrimitive.Content
          aria-modal="true"
          data-testid="workflow-propose-modal"
          className={cn(
            "fixed inset-0 z-50 bg-[var(--background)] sm:inset-auto sm:left-1/2 sm:top-1/2",
            "sm:w-[92vw] sm:max-w-2xl sm:-translate-x-1/2 sm:-translate-y-1/2",
            "sm:rounded-[var(--radius-lg)] sm:border sm:border-[var(--border)] sm:shadow-lg",
            "flex flex-col sm:max-h-[85vh] focus:outline-none",
          )}
        >
          <header className="flex items-start justify-between gap-4 border-b border-[var(--border)] px-6 py-5">
            <div className="flex-1 min-w-0">
              <DialogPrimitive.Title
                className="text-base font-semibold text-[var(--foreground)]"
                data-testid="workflow-propose-title"
              >
                {title}
              </DialogPrimitive.Title>
              <DialogPrimitive.Description className="mt-1 text-sm text-[var(--muted-foreground)]">
                {description}
              </DialogPrimitive.Description>
            </div>
            <DialogPrimitive.Close
              aria-label="Close"
              data-testid="workflow-propose-close"
              className={cn(
                "grid size-8 place-items-center rounded-[var(--radius-md)]",
                "text-[var(--muted-foreground)] hover:bg-[var(--muted)] hover:text-[var(--foreground)]",
                "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)]",
              )}
            >
              <X aria-hidden size={18} />
            </DialogPrimitive.Close>
          </header>

          <div className="grid grid-cols-1 gap-3 px-6 py-4 sm:grid-cols-2 border-b border-[var(--border)]">
            <label className="flex flex-col gap-1 text-sm">
              <span className="text-xs font-medium uppercase tracking-wide text-[var(--muted-foreground)]">
                Worker
              </span>
              <select
                data-testid="workflow-propose-worker"
                value={workerId}
                onChange={(e) => handleWorkerChange(e.target.value)}
                disabled={workers.length === 0}
                className={cn(
                  "rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--background)]",
                  "px-3 py-2 text-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)]",
                  "disabled:opacity-60",
                )}
              >
                {workers.length === 0 ? (
                  <option value="">No workers available</option>
                ) : null}
                {workers.map((w) => (
                  <option key={w.id} value={w.id}>
                    {w.name} · {w.role.vertical}
                  </option>
                ))}
              </select>
            </label>
            <label className="flex flex-col gap-1 text-sm">
              <span className="text-xs font-medium uppercase tracking-wide text-[var(--muted-foreground)]">
                Vertical
              </span>
              <select
                data-testid="workflow-propose-vertical"
                value={verticalId}
                onChange={(e) => setVerticalId(e.target.value)}
                disabled={Boolean(editingVertical)}
                className={cn(
                  "rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--background)]",
                  "px-3 py-2 text-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)]",
                  "disabled:opacity-60",
                )}
              >
                {verticalOptions.length === 0 ? (
                  <option value="">No verticals available</option>
                ) : null}
                {verticalOptions.map((v) => (
                  <option key={v} value={v}>
                    {v}
                  </option>
                ))}
              </select>
            </label>
          </div>

          <div className="flex-1 overflow-y-auto px-6 py-5">
            {canRender ? (
              <ProposeWorkflowChat
                key={`${workerId}:${verticalId}:${editingWorkflow?.workflowName ?? ""}`}
                workerId={workerId}
                verticalId={verticalId}
                approveEndpoint={approveEndpoint}
                onCancel={() => onOpenChange(false)}
                onApproved={handleApproved}
                fetchImpl={fetchImpl}
              />
            ) : (
              <p
                data-testid="workflow-propose-no-workers"
                className="rounded-[var(--radius-md)] border border-dashed border-[var(--border)] px-4 py-6 text-center text-sm text-[var(--muted-foreground)]"
              >
                Hire your first worker to draft a workflow.
              </p>
            )}
          </div>
        </DialogPrimitive.Content>
      </DialogPrimitive.Portal>
    </DialogPrimitive.Root>
  );
}
