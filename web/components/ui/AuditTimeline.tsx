import { forwardRef, type HTMLAttributes, type ReactNode } from "react";
import { cn } from "@/lib/utils";

export interface AuditTimelineRow {
  id: string;
  timestamp: string;
  kind: ReactNode;
  payload?: ReactNode;
  tone?: "default" | "success" | "warning" | "error" | "info";
}

export interface AuditTimelineProps extends HTMLAttributes<HTMLOListElement> {
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
    return (
      <ol
        ref={ref}
        data-testid="audit-timeline"
        className={cn("relative flex flex-col gap-4 border-l border-[var(--border)] pl-4", className)}
        {...props}
      >
        {rows.map((row) => (
          <li key={row.id} className="relative" data-testid="audit-timeline-row">
            <span
              aria-hidden
              className={cn(
                "absolute -left-[21px] top-1.5 size-2.5 rounded-full ring-2 ring-[var(--background)]",
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
          </li>
        ))}
      </ol>
    );
  },
);
AuditTimeline.displayName = "AuditTimeline";
