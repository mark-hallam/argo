"use client";

import { forwardRef, type ReactNode } from "react";
import { StatusPill, type StatusPillProps } from "@/components/ui/StatusPill";
import { cn } from "@/lib/utils";

/**
 * Atomic table cell. Composes inside a <td> or a CSS grid cell.
 * The audit-timeline + activity-feed surfaces (Track E) will use
 * a thin row component on top of these.
 */

interface BaseCellProps {
  className?: string;
  muted?: boolean;
  align?: "left" | "center" | "right";
}

const baseStyle = (a: BaseCellProps) =>
  cn(
    "text-sm tabular-nums leading-tight",
    a.muted ? "text-[var(--muted-foreground)]" : "text-[var(--foreground)]",
    a.align === "right" && "text-right",
    a.align === "center" && "text-center",
    a.className,
  );

export const TextCell = forwardRef<HTMLDivElement, BaseCellProps & { children: ReactNode }>(
  ({ children, ...rest }, ref) => (
    <div ref={ref} className={baseStyle(rest)}>
      {children}
    </div>
  ),
);
TextCell.displayName = "TextCell";

export const NumberCell = forwardRef<
  HTMLDivElement,
  BaseCellProps & { value: number; format?: (n: number) => string }
>(({ value, format, ...rest }, ref) => (
  <div ref={ref} className={baseStyle({ align: "right", ...rest })}>
    {format ? format(value) : value.toLocaleString()}
  </div>
));
NumberCell.displayName = "NumberCell";

export const TimestampCell = forwardRef<
  HTMLTimeElement,
  BaseCellProps & { iso: string; relative?: boolean }
>(({ iso, relative, ...rest }, ref) => (
  <time
    ref={ref}
    dateTime={iso}
    className={baseStyle({ muted: true, ...rest })}
    title={iso}
  >
    {relative ? toRelative(iso) : new Date(iso).toLocaleString()}
  </time>
));
TimestampCell.displayName = "TimestampCell";

export interface StatusCellProps extends BaseCellProps {
  tone: StatusPillProps["tone"];
  label: ReactNode;
}

export const StatusCell = forwardRef<HTMLDivElement, StatusCellProps>(
  ({ tone, label, ...rest }, ref) => (
    <div ref={ref} className={baseStyle(rest)}>
      <StatusPill tone={tone}>{label}</StatusPill>
    </div>
  ),
);
StatusCell.displayName = "StatusCell";

function toRelative(iso: string): string {
  const then = new Date(iso).getTime();
  const now = Date.now();
  const diffSec = Math.round((now - then) / 1000);
  if (diffSec < 60) return "just now";
  if (diffSec < 3600) return `${Math.round(diffSec / 60)}m ago`;
  if (diffSec < 86400) return `${Math.round(diffSec / 3600)}h ago`;
  return `${Math.round(diffSec / 86400)}d ago`;
}
