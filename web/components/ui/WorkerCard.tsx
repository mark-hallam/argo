"use client";

import { motion as fm, type Variants } from "motion/react";
import { forwardRef, type HTMLAttributes, type ReactNode } from "react";
import { Avatar, type WorkerStatus } from "@/components/ui/Avatar";
import { StatusPill } from "@/components/ui/StatusPill";
import { MotionList } from "@/lib/motion/primitives";
import { useMotion } from "@/lib/motion/provider";
import { cn } from "@/lib/utils";

export interface WorkerCardProps extends Omit<HTMLAttributes<HTMLDivElement>, "role" | "onAnimationStart" | "onDragStart" | "onDragEnd" | "onDrag" | "onAnimationEnd" | "onAnimationIteration"> {
  name: string;
  role: ReactNode;
  status: WorkerStatus;
  avatarSrc?: string;
  duties?: ReactNode[];
  footer?: ReactNode;
}

const statusToneByStatus: Record<WorkerStatus, "success" | "info" | "warning" | "error"> = {
  idle: "info",
  working: "success",
  waiting: "warning",
  blocked: "error",
};

export const WorkerCard = forwardRef<HTMLDivElement, WorkerCardProps>(
  ({ className, name, role, status, avatarSrc, duties, footer, ...props }, ref) => {
    const m = useMotion();
    const variants: Variants = {
      initial: m.fadeUp.initial ?? {},
      animate: m.fadeUp.animate ?? {},
    };
    return (
    <fm.div
      ref={ref}
      data-testid="worker-card"
      data-status={status}
      variants={variants}
      initial="initial"
      animate="animate"
      transition={m.fadeUp.transition}
      className={cn(
        "flex flex-col gap-3 p-5 rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--card)] text-[var(--card-foreground)] shadow-sm",
        className,
      )}
      {...props}
    >
      <div className="flex items-center gap-3">
        <Avatar size="md" alt={name} src={avatarSrc} status={status} />
        <div className="flex-1 min-w-0">
          <p className="text-sm font-semibold truncate text-[var(--foreground)]">{name}</p>
          <p className="text-xs text-[var(--muted-foreground)] truncate">{role}</p>
        </div>
        <StatusPill tone={statusToneByStatus[status]} size="sm">
          {status}
        </StatusPill>
      </div>
      {duties && duties.length > 0 ? (
        <ul className="flex flex-col gap-1">
          {duties.map((duty, i) => (
            <li key={i} className="text-xs text-[var(--muted-foreground)] flex gap-2">
              <span aria-hidden className="text-[var(--brand-primary)]">·</span>
              <span>{duty}</span>
            </li>
          ))}
        </ul>
      ) : null}
      {footer ? <div className="pt-1">{footer}</div> : null}
    </fm.div>
    );
  },
);
WorkerCard.displayName = "WorkerCard";

/**
 * Stagger wrapper for a grid/list of `WorkerCard`s. Each child fades
 * up sequentially using the brand motion vocabulary. Drop in where
 * the consumer previously rendered a plain `<div>` or `<ul>`.
 */
export function WorkerCardList({
  children,
  className,
  as,
}: {
  children: ReactNode;
  className?: string;
  as?: "div" | "ul" | "ol";
}) {
  return (
    <MotionList as={as} className={className}>
      {children}
    </MotionList>
  );
}
