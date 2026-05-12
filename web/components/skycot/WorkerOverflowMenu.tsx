"use client";

import * as DropdownMenu from "@radix-ui/react-dropdown-menu";
import { Archive, Download, Eye, PauseCircle, Slash } from "lucide-react";
import type { ReactNode } from "react";
import { cn } from "@/lib/utils";
import type { Worker } from "@/lib/types/worker";

/**
 * Overflow menu split out of `WorkerHero` so the page can
 * lazy-load (`next/dynamic`) the Radix dropdown bundle. The trigger
 * button is still rendered eagerly in the hero; only this menu
 * content + its handler wiring ship on first paint when the user
 * actually opens the menu.
 */

export type WorkerHeroAction =
  | "pause"
  | "disable-duties"
  | "archive"
  | "export"
  | "view-as-operator";

export interface WorkerOverflowMenuProps {
  worker: Worker;
  trigger: ReactNode;
  /** Default open state — used when the menu is lazy-mounted on click. */
  defaultOpen?: boolean;
  onOpenChange?: (open: boolean) => void;
  onAction?: (action: WorkerHeroAction) => void;
}

export function WorkerOverflowMenu({
  worker,
  trigger,
  defaultOpen,
  onOpenChange,
  onAction,
}: WorkerOverflowMenuProps) {
  const perms = worker.permissions;
  const fire = (action: WorkerHeroAction) => onAction?.(action);

  return (
    <DropdownMenu.Root defaultOpen={defaultOpen} onOpenChange={onOpenChange}>
      <DropdownMenu.Trigger asChild>{trigger}</DropdownMenu.Trigger>
      <DropdownMenu.Portal>
        <DropdownMenu.Content
          data-testid="worker-overflow-menu"
          align="end"
          sideOffset={4}
          className="z-50 min-w-[200px] rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--popover)] text-[var(--popover-foreground)] shadow-md p-1 text-sm"
        >
          <Item
            testid="worker-action-pause"
            icon={<PauseCircle className="size-4" />}
            disabled={perms ? !perms.canPauseResume : false}
            onSelect={() => fire("pause")}
          >
            {worker.status === "paused" ? "Resume" : "Pause"}
          </Item>
          <Item
            testid="worker-action-disable-duties"
            icon={<Slash className="size-4" />}
            disabled={perms ? !perms.canEditDuties : false}
            onSelect={() => fire("disable-duties")}
          >
            Disable duties
          </Item>
          <Item
            testid="worker-action-archive"
            icon={<Archive className="size-4" />}
            disabled={perms ? !perms.canArchive : false}
            onSelect={() => fire("archive")}
          >
            Archive
          </Item>
          <DropdownMenu.Separator className="my-1 h-px bg-[var(--border)]" />
          <Item
            testid="worker-action-export"
            icon={<Download className="size-4" />}
            onSelect={() => fire("export")}
          >
            Export data
          </Item>
          <Item
            testid="worker-action-view-as-operator"
            icon={<Eye className="size-4" />}
            onSelect={() => fire("view-as-operator")}
          >
            View as Operator
            <span className="ml-auto text-[10px] uppercase tracking-wider text-[var(--muted-foreground)]">
              soon
            </span>
          </Item>
        </DropdownMenu.Content>
      </DropdownMenu.Portal>
    </DropdownMenu.Root>
  );
}

interface ItemProps {
  testid: string;
  icon: ReactNode;
  disabled?: boolean;
  onSelect: () => void;
  children: ReactNode;
}

function Item({ testid, icon, disabled, onSelect, children }: ItemProps) {
  return (
    <DropdownMenu.Item
      data-testid={testid}
      disabled={disabled}
      onSelect={onSelect}
      className={cn(
        "flex items-center gap-2 rounded-[var(--radius-sm)] px-2 py-1.5 outline-none cursor-default select-none",
        "data-[highlighted]:bg-[var(--accent)] data-[highlighted]:text-[var(--accent-foreground)]",
        "data-[disabled]:opacity-40 data-[disabled]:pointer-events-none",
      )}
    >
      {icon}
      {children}
    </DropdownMenu.Item>
  );
}
