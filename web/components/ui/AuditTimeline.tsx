"use client";

import { AnimatePresence, motion as fm, type Variants } from "motion/react";
import { forwardRef, type HTMLAttributes, type ReactNode } from "react";
import { useMotion } from "@/lib/motion/provider";
import { cn } from "@/lib/utils";

export interface AuditTimelineRow {
  id: string;
  timestamp: string;
  kind: ReactNode;
  payload?: ReactNode;
  tone?: "default" | "success" | "warning" | "error" | "info";
}

export interface AuditTimelineProps extends Omit<HTMLAttributes<HTMLOListElement>, "onAnimationStart" | "onDragStart" | "onDragEnd" | "onDrag" | "onAnimationEnd" | "onAnimationIteration"> {
  rows: AuditTimelineRow[];
  emptyMessage?: ReactNode;
}

const dotByTone: Record<NonNullable<AuditTimelineRow["tone"]>, string> = {
  default: "bg-[var(--muted-foreground)]",
  success: "bg-[var(--success)]",
  warning: "bg-[var(--warning)]",
  error: "bg-[var(--destructive)]",
  info: "bg-[var(--info)]",
};

export const AuditTimeline = forwardRef<HTMLOListElement, AuditTimelineProps>(
  ({ className, rows, emptyMessage = "No audit entries yet.", ...props }, ref) => {
    if (rows.length === 0) {
      return (
        <p className="text-sm text-[var(--muted-foreground)]" data-testid="audit-timeline-empty">
          {emptyMessage}
        </p>
      );
    }
    const m = useMotion();
    const listVariants: Variants = {
      initial: {},
      animate: { transition: m.staggerChildren.animate?.transition },
    };
    const rowVariants: Variants = {
      initial: m.fadeUp.initial ?? {},
      animate: m.fadeUp.animate ?? {},
    };
    return (
      <fm.ol
        ref={ref}
        data-testid="audit-timeline"
        initial="initial"
        animate="animate"
        variants={listVariants}
        className={cn("relative flex flex-col gap-4 border-l border-[var(--border)] pl-4", className)}
        {...props}
      >
        <AnimatePresence initial={false}>
        {rows.map((row) => (
          <fm.li
            key={row.id}
            className="relative"
            data-testid="audit-timeline-row"
            variants={rowVariants}
            exit={m.fadeUp.exit}
            transition={m.fadeUp.transition}
          >
            <span
              aria-hidden
              className={cn(
                "absolute -left-[21px] top-1 size-2.5 rounded-full ring-2 ring-[var(--background)]",
                dotByTone[row.tone ?? "default"],
              )}
            />
            <div className="flex flex-col gap-1">
              <div className="flex items-center gap-2 flex-wrap">
                <time
                  dateTime={row.timestamp}
                  className="text-xs font-mono text-[var(--muted-foreground)]"
                >
                  {new Date(row.timestamp).toLocaleString()}
                </time>
                <span className="text-sm font-medium text-[var(--foreground)]">{row.kind}</span>
              </div>
              {row.payload ? (
                <div className="text-xs text-[var(--muted-foreground)] leading-relaxed">
                  {row.payload}
                </div>
              ) : null}
            </div>
          </fm.li>
        ))}
        </AnimatePresence>
      </fm.ol>
    );
  },
);
AuditTimeline.displayName = "AuditTimeline";
