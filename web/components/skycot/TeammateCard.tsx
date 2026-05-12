"use client";

import { ArrowRight } from "lucide-react";
import { useRouter } from "next/navigation";
import { useCallback, useRef, useState, type KeyboardEvent } from "react";
import { AlertDialog } from "@/components/ui/AlertDialog";
import { Avatar } from "@/components/ui/Avatar";
import { buttonVariants } from "@/components/ui/Button";
import { Card } from "@/components/ui/Card";
import { StatusPill, type StatusPillProps } from "@/components/ui/StatusPill";
import { useWorkerActions } from "@/lib/hooks/use-worker-actions";
import { cn } from "@/lib/utils";
import type { Worker, WorkerStatus } from "@/lib/types/worker";
import {
  TeammateCardContextMenu,
  type TeammateCardContextMenuHandle,
} from "./TeammateCardContextMenu";

/** Spec: streams/E3-b-team-teammate-card.md. */

const STATUS_TONES: Record<WorkerStatus, NonNullable<StatusPillProps["tone"]>> = {
  working: "info",
  waiting: "warning",
  idle: "muted",
  blocked: "error",
  paused: "muted",
};

const STATUS_LABELS: Record<WorkerStatus, string> = {
  working: "Working",
  waiting: "Waiting on you",
  idle: "Idle",
  blocked: "Blocked",
  paused: "Paused",
};

const LONG_PRESS_MS = 500;

/** Toast payload emitted to the host page's toast viewport. */
export interface TeammateCardToast {
  id: string;
  title: string;
  tone: "default" | "success";
}

export interface TeammateCardProps {
  worker: Worker;
  /** Host renders a Radix Toast for each emission. Optional in tests. */
  onShowToast?: (toast: TeammateCardToast) => void;
  className?: string;
  /** Forwarded to outer Card for tests / parents that need a handle. */
  "data-testid"?: string;
}

export function TeammateCard({
  worker,
  onShowToast,
  className,
  "data-testid": testId,
}: TeammateCardProps) {
  const router = useRouter();
  const menuRef = useRef<TeammateCardContextMenuHandle>(null);
  const longPressTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const actions = useWorkerActions(worker.id);
  const [confirm, setConfirm] = useState<"pause" | "resume" | null>(null);

  const cardTestId = testId ?? `teammate-${worker.id}`;
  const profileHref = `/workers/${worker.id}`;
  const tone = STATUS_TONES[worker.status];
  const statusLabel = STATUS_LABELS[worker.status];

  const openProfile = useCallback(() => {
    router.push(profileHref);
  }, [router, profileHref]);

  const onCardClick = useCallback(() => {
    openProfile();
  }, [openProfile]);

  const onCardKeyDown = useCallback(
    (e: KeyboardEvent<HTMLDivElement>) => {
      if (e.key !== "Enter" && e.key !== " ") return;
      if (e.target !== e.currentTarget) return;
      e.preventDefault();
      openProfile();
    },
    [openProfile],
  );

  const onContextMenu = useCallback(
    (e: React.MouseEvent<HTMLDivElement>) => {
      e.preventDefault();
      menuRef.current?.open();
    },
    [],
  );

  const startLongPress = useCallback(() => {
    if (longPressTimer.current) clearTimeout(longPressTimer.current);
    longPressTimer.current = setTimeout(() => {
      menuRef.current?.open();
    }, LONG_PRESS_MS);
  }, []);

  const cancelLongPress = useCallback(() => {
    if (longPressTimer.current) {
      clearTimeout(longPressTimer.current);
      longPressTimer.current = null;
    }
  }, []);

  const performAction = useCallback(
    async (kind: "pause" | "resume") => {
      try {
        if (kind === "pause") await actions.pauseWorker(worker.id);
        else await actions.resumeWorker(worker.id);
        onShowToast?.({
          id: `${worker.id}-${kind}-${Date.now()}`,
          title:
            kind === "pause"
              ? `${worker.name} paused. Resume from her profile.`
              : `${worker.name} resumed.`,
          tone: "success",
        });
      } catch {
        onShowToast?.({
          id: `${worker.id}-${kind}-error-${Date.now()}`,
          title:
            kind === "pause"
              ? `Couldn't pause ${worker.name}. Try again.`
              : `Couldn't resume ${worker.name}. Try again.`,
          tone: "default",
        });
      }
    },
    [actions, worker.id, worker.name, onShowToast],
  );

  const bullets = worker.headline?.bullets ?? [];
  const tomorrow = worker.headline?.tomorrow;

  return (
    <>
      <Card
        role="group"
        aria-label={`${worker.name} — ${statusLabel}`}
        tabIndex={0}
        data-testid={cardTestId}
        onClick={onCardClick}
        onKeyDown={onCardKeyDown}
        onContextMenu={onContextMenu}
        onPointerDown={(e) => {
          if (e.pointerType === "touch") startLongPress();
        }}
        onPointerUp={cancelLongPress}
        onPointerCancel={cancelLongPress}
        onPointerLeave={cancelLongPress}
        className={cn(
          "group cursor-pointer p-5 flex flex-col gap-3.5 outline-none",
          "transition-[transform,box-shadow] duration-[var(--duration-fast)]",
          "hover:shadow-md hover:-translate-y-0.5",
          "focus-visible:ring-2 focus-visible:ring-[var(--ring)] focus-visible:ring-offset-2 focus-visible:ring-offset-[var(--background)]",
          className,
        )}
      >
        <div className="flex items-start gap-3">
          <Avatar
            alt={worker.name}
            src={worker.avatarUrl}
            size="lg"
            status={worker.status === "paused" ? undefined : avatarStatus(worker.status)}
          />
          <div className="min-w-0 flex-1">
            <div className="font-semibold text-[var(--foreground)] leading-tight truncate">
              {worker.name}
            </div>
            <div className="mt-1.5 flex items-center gap-2 flex-wrap">
              <StatusPill tone={tone} size="sm" data-testid={`teammate-status-${worker.id}`}>
                {statusLabel}
              </StatusPill>
            </div>
            <div className="mt-1.5 text-xs text-[var(--muted-foreground)] truncate">
              {worker.role.tagline}
            </div>
          </div>
          <TeammateCardContextMenu
            ref={menuRef}
            status={worker.status}
            workerName={worker.name}
            disabled={actions.isPending}
            onPauseRequested={() => setConfirm("pause")}
            onResumeRequested={() => setConfirm("resume")}
            data-testid={worker.id}
          />
        </div>

        {bullets.length > 0 || tomorrow ? (
          <div className="flex flex-col gap-2 text-sm leading-snug">
            {bullets.length > 0 ? (
              <div data-testid={`teammate-today-${worker.id}`}>
                <div className="text-[11px] font-semibold uppercase tracking-wider text-[var(--muted-foreground)]">
                  Today
                </div>
                <ul className="mt-1 space-y-1 text-[var(--foreground)]">
                  {bullets.slice(0, 3).map((b, i) => (
                    <li key={`${worker.id}-bullet-${i}`} className="flex gap-2">
                      <span aria-hidden className="text-[var(--muted-foreground)]">
                        •
                      </span>
                      <span className="min-w-0 flex-1">{b}</span>
                    </li>
                  ))}
                </ul>
              </div>
            ) : null}
            {tomorrow ? (
              <div data-testid={`teammate-tomorrow-${worker.id}`} className="text-xs text-[var(--muted-foreground)]">
                <span className="font-medium text-[var(--foreground)]">Tomorrow:</span>{" "}
                {tomorrow}
              </div>
            ) : null}
          </div>
        ) : null}

        <div className="border-t border-[var(--border)] -mx-5" />

        <a
          href={profileHref}
          data-testid={`teammate-open-profile-${worker.id}`}
          onClick={(e) => {
            e.stopPropagation();
            e.preventDefault();
            openProfile();
          }}
          className={cn(
            buttonVariants({ variant: "ghost", size: "sm" }),
            "-mx-2 -mb-1 justify-center text-[var(--muted-foreground)] hover:text-[var(--foreground)]",
          )}
        >
          Open profile
          <ArrowRight aria-hidden className="size-3.5" />
        </a>
      </Card>

      <AlertDialog
        open={confirm !== null}
        onOpenChange={(next) => {
          if (!next) setConfirm(null);
        }}
        title={
          confirm === "resume"
            ? `Resume ${worker.name}?`
            : `Pause ${worker.name}?`
        }
        description={
          confirm === "resume"
            ? "They'll pick up where they left off."
            : "They'll stop taking new actions until you resume them. Current pending approvals stay where they are."
        }
        confirmLabel={confirm === "resume" ? "Resume" : "Pause"}
        cancelLabel="Cancel"
        confirmVariant={confirm === "resume" ? "primary" : "secondary"}
        onConfirm={() => {
          const kind = confirm;
          setConfirm(null);
          if (kind) void performAction(kind);
        }}
      />
    </>
  );
}

function avatarStatus(
  status: WorkerStatus,
): "idle" | "working" | "waiting" | "blocked" {
  return status === "paused" ? "idle" : status;
}
