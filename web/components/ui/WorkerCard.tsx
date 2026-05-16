import { forwardRef, type HTMLAttributes, type ReactNode } from "react";
import { Avatar, type WorkerStatus } from "@/components/ui/Avatar";
import { StatusPill } from "@/components/ui/StatusPill";
import { cn } from "@/lib/utils";

export interface WorkerCardProps extends Omit<HTMLAttributes<HTMLDivElement>, "role"> {
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
  ({ className, name, role, status, avatarSrc, duties, footer, ...props }, ref) => (
    <div
      ref={ref}
      data-testid="worker-card"
      data-status={status}
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
    </div>
  ),
);
WorkerCard.displayName = "WorkerCard";
