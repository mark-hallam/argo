"use client";

import * as DropdownMenu from "@radix-ui/react-dropdown-menu";
import { MoreVertical, Pause, Play } from "lucide-react";
import {
  forwardRef,
  useImperativeHandle,
  useState,
  type ReactNode,
} from "react";
import { cn } from "@/lib/utils";
import type { WorkerStatus } from "@/lib/types/worker";

/**
 * Drop-down menu attached to a TeammateCard. Owns:
 *  - the `⋮` icon trigger button (top-right of the card)
 *  - the menu's open/closed state
 *  - the menu entries themselves (pause / resume)
 *
 * The card host opens the menu programmatically (right-click or
 * long-press) via the imperative ref handle. Click-to-open works
 * through the Radix trigger.
 *
 * The menu only emits "request" callbacks — the host owns the
 * confirm-dialog + mutation wiring so the menu stays stateless.
 */
export interface TeammateCardContextMenuHandle {
  /** Programmatically open the menu. Used by right-click + long-press. */
  open: () => void;
}

export interface TeammateCardContextMenuProps {
  status: WorkerStatus;
  workerName: string;
  /** Disable menu trigger (pending mutation, etc.). */
  disabled?: boolean;
  /** Fired when the user activates the Pause entry. Host shows confirm. */
  onPauseRequested: () => void;
  /** Fired when the user activates the Resume entry. Host shows confirm. */
  onResumeRequested: () => void;
  /** Test id suffix; defaults to the worker name. */
  "data-testid"?: string;
}

const PAUSABLE_STATUSES: WorkerStatus[] = ["working", "waiting", "idle", "blocked"];

function canPause(status: WorkerStatus): boolean {
  // Brief: visible when status ∈ {working, waiting, idle}. Blocked is
  // also a pausable surface in v1 — pausing a blocked worker prevents
  // its background retries — though the wireframe doesn't list it.
  // Sensible deviation flagged in the PR body.
  return PAUSABLE_STATUSES.includes(status);
}

function canResume(status: WorkerStatus): boolean {
  return status === "paused";
}

export const TeammateCardContextMenu = forwardRef<
  TeammateCardContextMenuHandle,
  TeammateCardContextMenuProps
>(function TeammateCardContextMenu(
  {
    status,
    workerName,
    disabled,
    onPauseRequested,
    onResumeRequested,
    "data-testid": testId,
  },
  ref,
) {
  const [open, setOpen] = useState(false);

  useImperativeHandle(
    ref,
    () => ({
      open: () => {
        if (!disabled) setOpen(true);
      },
    }),
    [disabled],
  );

  const triggerTestId = testId
    ? `teammate-card-menu-trigger-${testId}`
    : "teammate-card-menu-trigger";

  return (
    <DropdownMenu.Root open={open} onOpenChange={setOpen}>
      <DropdownMenu.Trigger asChild>
        <button
          type="button"
          aria-label={`Open actions for ${workerName}`}
          data-testid={triggerTestId}
          disabled={disabled}
          onClick={(e) => e.stopPropagation()}
          className={cn(
            "inline-flex size-7 items-center justify-center rounded-[var(--radius-sm)]",
            "text-[var(--muted-foreground)] hover:bg-[var(--accent)] hover:text-[var(--accent-foreground)]",
            "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)] focus-visible:ring-offset-1 focus-visible:ring-offset-[var(--card)]",
            "disabled:opacity-50 disabled:pointer-events-none",
            "transition-colors duration-[var(--duration-fast)]",
          )}
        >
          <MoreVertical aria-hidden className="size-4" />
        </button>
      </DropdownMenu.Trigger>

      <DropdownMenu.Portal>
        <DropdownMenu.Content
          align="end"
          sideOffset={6}
          collisionPadding={8}
          data-testid={testId ? `teammate-card-menu-${testId}` : "teammate-card-menu"}
          className={cn(
            "z-50 min-w-[180px] overflow-hidden rounded-[var(--radius-md)]",
            "border border-[var(--border)] bg-[var(--card)] shadow-lg outline-none",
            "data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=open]:zoom-in-95",
            "data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=closed]:zoom-out-95",
          )}
        >
          {canPause(status) ? (
            <MenuItem
              icon={<Pause className="size-4" />}
              testId={testId ? `teammate-card-menu-pause-${testId}` : "teammate-card-menu-pause"}
              onSelect={onPauseRequested}
            >
              Pause {workerName}
            </MenuItem>
          ) : null}
          {canResume(status) ? (
            <MenuItem
              icon={<Play className="size-4" />}
              testId={testId ? `teammate-card-menu-resume-${testId}` : "teammate-card-menu-resume"}
              onSelect={onResumeRequested}
            >
              Resume {workerName}
            </MenuItem>
          ) : null}
        </DropdownMenu.Content>
      </DropdownMenu.Portal>
    </DropdownMenu.Root>
  );
});

interface MenuItemProps {
  icon: ReactNode;
  children: ReactNode;
  onSelect: () => void;
  testId: string;
}

function MenuItem({ icon, children, onSelect, testId }: MenuItemProps) {
  return (
    <DropdownMenu.Item
      data-testid={testId}
      onSelect={onSelect}
      className={cn(
        "flex w-full cursor-pointer items-center gap-2 px-3 py-2 text-sm leading-none outline-none",
        "text-[var(--foreground)] data-[highlighted]:bg-[var(--accent)] data-[highlighted]:text-[var(--accent-foreground)]",
        "transition-colors duration-[var(--duration-fast)]",
      )}
    >
      <span className="text-[var(--muted-foreground)]" aria-hidden>
        {icon}
      </span>
      <span>{children}</span>
    </DropdownMenu.Item>
  );
}
