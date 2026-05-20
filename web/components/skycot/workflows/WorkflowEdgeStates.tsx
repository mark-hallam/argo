"use client";

import Link from "next/link";
import { AlertCircle, RotateCw, Sparkles, Plus, Users } from "lucide-react";
import { buttonVariants } from "@/components/ui/Button";

/**
 * Empty + error edge states for the visual workflow builder, the
 * timeline viewer, and the orchestration queue.
 *
 * The 4 wave-1-audited pages each get a dedicated state that mirrors
 * the host page's brand register: gradient icon, brand-ink headline,
 * muted-foreground description, primary CTA. None of these use
 * `<ErrorShell>` directly — they live inside the page chrome (sticky
 * top bar, sidebar) and `ErrorShell` owns the full-screen 404/500
 * routes. The styling matches the team-page ErrorState shape
 * (rounded card with dashed/destructive border + icon + heading +
 * description + action).
 */

interface ActionProps {
  onRetry?: () => void;
}

export function WorkflowBuilderErrorState({ onRetry }: ActionProps) {
  return (
    <div
      data-testid="workflow-builder-error-state"
      role="alert"
      className="mx-auto my-12 max-w-md rounded-[var(--radius-lg)] border border-[var(--destructive)]/30 bg-[var(--destructive)]/5 px-6 py-12 text-center space-y-4"
    >
      <div className="mx-auto grid size-12 place-items-center rounded-full bg-[var(--destructive)]/10 text-[var(--destructive)]">
        <AlertCircle aria-hidden />
      </div>
      <div className="space-y-1">
        <h2 className="text-base font-semibold text-[var(--foreground)]">
          Couldn&apos;t load the workflow
        </h2>
        <p className="text-sm text-[var(--muted-foreground)]">
          The gateway returned an error while loading this workflow. Your unsaved
          edits aren&apos;t lost — retry, or head back to the catalog.
        </p>
      </div>
      <div className="flex flex-wrap items-center justify-center gap-3">
        {onRetry ? (
          <button
            type="button"
            onClick={onRetry}
            data-testid="workflow-builder-error-retry"
            className="inline-flex items-center gap-2 rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--background)] px-4 py-2 text-sm font-medium hover:bg-[var(--accent)]"
          >
            <RotateCw className="size-3.5" />
            Try again
          </button>
        ) : null}
        <Link
          href="/workflows"
          className={buttonVariants({ variant: "secondary", size: "md" })}
        >
          Back to workflows
        </Link>
      </div>
    </div>
  );
}

export function WorkflowBuilderEmptyState({
  onAddStep,
}: {
  onAddStep?: () => void;
}) {
  return (
    <div
      data-testid="workflow-builder-empty-state"
      className="mx-auto my-16 max-w-md rounded-[var(--radius-lg)] border border-dashed border-[var(--border)] bg-[var(--card)]/40 px-6 py-12 text-center space-y-4"
    >
      <div
        className="mx-auto grid size-12 place-items-center rounded-[12px] text-white"
        style={{ background: "linear-gradient(135deg, #8b5cf6, #ec4899)" }}
      >
        <Sparkles aria-hidden />
      </div>
      <div className="space-y-1">
        <h2 className="text-base font-semibold text-[var(--foreground)]">
          This workflow has no steps yet
        </h2>
        <p className="text-sm text-[var(--muted-foreground)]">
          Add a trigger, a skill, or an approval gate — or describe the change
          in the prompt bar above and let the AI propose it.
        </p>
      </div>
      <button
        type="button"
        onClick={onAddStep}
        data-testid="workflow-builder-empty-add-step"
        className="inline-flex items-center gap-2 rounded-[var(--radius-md)] px-4 py-2 text-sm font-semibold text-white"
        style={{
          background: "linear-gradient(135deg, #8b5cf6, #ec4899)",
          boxShadow: "0 4px 12px -2px rgba(139,92,246,0.4)",
        }}
      >
        <Plus className="size-3.5" />
        Add the first step
      </button>
    </div>
  );
}

export function WorkflowViewerErrorState({ onRetry }: ActionProps) {
  return (
    <div
      data-testid="workflow-viewer-error-state"
      role="alert"
      className="mx-auto my-12 max-w-md rounded-[var(--radius-lg)] border border-[var(--destructive)]/30 bg-[var(--destructive)]/5 px-6 py-12 text-center space-y-4"
    >
      <div className="mx-auto grid size-12 place-items-center rounded-full bg-[var(--destructive)]/10 text-[var(--destructive)]">
        <AlertCircle aria-hidden />
      </div>
      <div className="space-y-1">
        <h2 className="text-base font-semibold text-[var(--foreground)]">
          Couldn&apos;t load this workflow
        </h2>
        <p className="text-sm text-[var(--muted-foreground)]">
          The gateway didn&apos;t respond. Try again, or pick a different workflow
          from the catalog.
        </p>
      </div>
      <div className="flex flex-wrap items-center justify-center gap-3">
        {onRetry ? (
          <button
            type="button"
            onClick={onRetry}
            data-testid="workflow-viewer-error-retry"
            className="inline-flex items-center gap-2 rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--background)] px-4 py-2 text-sm font-medium hover:bg-[var(--accent)]"
          >
            <RotateCw className="size-3.5" />
            Try again
          </button>
        ) : null}
        <Link
          href="/workflows"
          className={buttonVariants({ variant: "secondary", size: "md" })}
        >
          Back to workflows
        </Link>
      </div>
    </div>
  );
}

export function OrchestrationEmptyState() {
  return (
    <div
      data-testid="orchestration-empty-state"
      className="mx-auto my-16 max-w-md rounded-[var(--radius-lg)] border border-dashed border-[var(--border)] bg-[var(--card)]/40 px-6 py-12 text-center space-y-4"
    >
      <div
        className="mx-auto grid size-12 place-items-center rounded-[12px] text-white"
        style={{ background: "linear-gradient(135deg, #8b5cf6, #ec4899)" }}
      >
        <Users aria-hidden />
      </div>
      <div className="space-y-1">
        <h2 className="text-base font-semibold text-[var(--foreground)]">
          No orchestration yet
        </h2>
        <p className="text-sm text-[var(--muted-foreground)]">
          Hire your first teammate to start. Their runs will show up here so you
          can watch SLAs, force escalations, and reset retries in real time.
        </p>
      </div>
      <Link
        href="/hire"
        data-testid="orchestration-empty-hire-cta"
        className="inline-flex items-center gap-2 rounded-[var(--radius-md)] px-4 py-2 text-sm font-semibold text-white"
        style={{
          background: "linear-gradient(135deg, #8b5cf6, #ec4899)",
          boxShadow: "0 4px 12px -2px rgba(139,92,246,0.4)",
        }}
      >
        <Plus className="size-3.5" />
        Hire your first teammate
      </Link>
    </div>
  );
}
