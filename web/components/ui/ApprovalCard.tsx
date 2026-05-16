"use client";

import { motion as fm } from "motion/react";
import { forwardRef, type HTMLAttributes, type ReactNode } from "react";
import { Button } from "@/components/ui/Button";
import { StatusPill } from "@/components/ui/StatusPill";
import { useMotion } from "@/lib/motion/provider";
import { cn } from "@/lib/utils";

export type ApprovalState = "pending" | "approved" | "denied";

export interface ApprovalCardProps extends Omit<HTMLAttributes<HTMLDivElement>, "title" | "onAnimationStart" | "onDragStart" | "onDragEnd" | "onDrag" | "onAnimationEnd" | "onAnimationIteration"> {
  title: ReactNode;
  summary?: ReactNode;
  requestedBy?: ReactNode;
  requestedAt?: string;
  payload?: ReactNode;
  state?: ApprovalState;
  onApprove?: () => void;
  onDeny?: () => void;
  busy?: boolean;
}

export const ApprovalCard = forwardRef<HTMLDivElement, ApprovalCardProps>(
  (
    {
      className,
      title,
      summary,
      requestedBy,
      requestedAt,
      payload,
      state = "pending",
      onApprove,
      onDeny,
      busy = false,
      ...props
    },
    ref,
  ) => {
    const finalized = state !== "pending";
    const m = useMotion();
    const pulseAnimate = m.successPulse.animate ?? {};
    const animate = finalized
      ? {
          ...(m.scaleIn.animate ?? {}),
          scale: (pulseAnimate as { scale?: number | number[] }).scale ?? 1,
          transition: (pulseAnimate as { transition?: object }).transition,
        }
      : (m.scaleIn.animate ?? {});
    return (
      <fm.div
        ref={ref}
        data-testid="approval-card"
        data-state={state}
        initial={m.scaleIn.initial}
        animate={animate}
        transition={m.scaleIn.transition}
        className={cn(
          "flex flex-col gap-4 p-5 rounded-[var(--radius-lg)] border bg-[var(--card)] text-[var(--card-foreground)] shadow-sm",
          state === "approved" && "border-[var(--success)]/40",
          state === "denied" && "border-[var(--destructive)]/40",
          state === "pending" && "border-[var(--border)]",
          className,
        )}
        {...props}
      >
        <div className="flex items-start justify-between gap-3">
          <div className="flex flex-col gap-1 min-w-0">
            <h3 className="text-base font-semibold text-[var(--foreground)] truncate">{title}</h3>
            {summary ? (
              <p className="text-sm text-[var(--muted-foreground)] leading-relaxed">{summary}</p>
            ) : null}
            {(requestedBy || requestedAt) ? (
              <p className="text-xs text-[var(--muted-foreground)]">
                {requestedBy ? <>Requested by {requestedBy}</> : null}
                {requestedBy && requestedAt ? <span aria-hidden> &middot; </span> : null}
                {requestedAt ? (
                  <time dateTime={requestedAt}>{new Date(requestedAt).toLocaleString()}</time>
                ) : null}
              </p>
            ) : null}
          </div>
          {state === "approved" ? <StatusPill tone="success">Approved</StatusPill> : null}
          {state === "denied" ? <StatusPill tone="error">Denied</StatusPill> : null}
        </div>
        {payload ? (
          <div className="rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--muted)] p-3 text-xs font-mono text-[var(--foreground)] overflow-x-auto">
            {payload}
          </div>
        ) : null}
        {!finalized ? (
          <div className="flex flex-col-reverse sm:flex-row sm:justify-end gap-2">
            <Button
              variant="ghost"
              onClick={onDeny}
              disabled={busy}
              data-testid="approval-deny"
            >
              Deny
            </Button>
            <Button
              variant="primary"
              onClick={onApprove}
              loading={busy}
              data-testid="approval-approve"
            >
              Approve
            </Button>
          </div>
        ) : null}
      </fm.div>
    );
  },
);
ApprovalCard.displayName = "ApprovalCard";
