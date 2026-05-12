"use client";

import * as Popover from "@radix-ui/react-popover";
import { useCallback, useMemo, type ReactNode } from "react";
import { Button } from "@/components/ui/Button";
import { cn } from "@/lib/utils";
import {
  useDismiss,
  useMarkAllAsRead,
  useNotifications,
} from "@/lib/hooks/use-notifications";
import {
  groupNotifications,
  type NotificationItem,
} from "@/lib/sample-notifications";
import { NotificationRow } from "./NotificationRow";

const SECTION_LABELS = {
  today: "Today",
  yesterday: "Yesterday",
  earlier: "Earlier",
} as const;

export interface NotificationsPopoverProps {
  /**
   * Optional callback hook for inline-CTA `action: "callback"` flows.
   * Implementations route by notification kind (e.g. open the OAuth
   * re-auth modal for `oauth-needs-reauth`).
   */
  onCtaCallback?: (item: NotificationItem) => void;
  /** Trigger element — typically the bell button. */
  children: ReactNode;
  /** Open the popover immediately on mount (used for lazy-mount path). */
  defaultOpen?: boolean;
  /** Called whenever the popover's open state changes. */
  onOpenChange?: (open: boolean) => void;
}

export function NotificationsPopover({
  onCtaCallback,
  children,
  defaultOpen,
  onOpenChange,
}: NotificationsPopoverProps) {
  const query = useNotifications();
  const markAllAsRead = useMarkAllAsRead();
  const dismiss = useDismiss();

  const items = useMemo(() => query.data ?? [], [query.data]);
  const hasUnread = items.some((n) => !n.read);
  const groups = useMemo(() => groupNotifications(items), [items]);

  const handleOpenChange = useCallback(
    (open: boolean) => {
      if (open && hasUnread) markAllAsRead.mutate();
      onOpenChange?.(open);
    },
    [hasUnread, markAllAsRead, onOpenChange],
  );

  const handleDismiss = useCallback(
    (id: string) => dismiss.mutate(id),
    [dismiss],
  );

  return (
    <Popover.Root defaultOpen={defaultOpen} onOpenChange={handleOpenChange}>
      <Popover.Trigger asChild>{children}</Popover.Trigger>
      <Popover.Portal>
        <Popover.Content
          align="end"
          sideOffset={8}
          collisionPadding={8}
          className={cn(
            "z-50 w-[380px] max-w-[calc(100vw-1rem)] outline-none",
            "rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--card)] shadow-lg",
            "data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=open]:zoom-in-95",
            "data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=closed]:zoom-out-95",
          )}
          data-testid="notifications-popover-content"
        >
          <PopoverHeader
            unreadCount={items.filter((n) => !n.read).length}
            onMarkAllRead={() => markAllAsRead.mutate()}
          />

          <div className="max-h-[60vh] overflow-y-auto">
            {query.isPending ? (
              <LoadingState />
            ) : query.isError ? (
              <ErrorState message={(query.error as Error)?.message ?? "Couldn't load notifications"} />
            ) : items.length === 0 ? (
              <EmptyState />
            ) : (
              <Sections
                groups={groups}
                onDismiss={handleDismiss}
                onCtaCallback={onCtaCallback}
              />
            )}
          </div>
        </Popover.Content>
      </Popover.Portal>
    </Popover.Root>
  );
}

function PopoverHeader({
  unreadCount,
  onMarkAllRead,
}: {
  unreadCount: number;
  onMarkAllRead: () => void;
}) {
  return (
    <div className="flex items-center justify-between border-b border-[var(--border)] px-4 py-3">
      <div className="flex items-center gap-2">
        <h2 className="text-sm font-semibold text-[var(--foreground)]">Notifications</h2>
        {unreadCount > 0 ? (
          <span
            className="rounded-full bg-[var(--primary)]/15 px-2 py-0.5 text-[10px] font-semibold tabular-nums text-[var(--primary)]"
            data-testid="notifications-unread-count"
          >
            {unreadCount}
          </span>
        ) : null}
      </div>
      <Button
        variant="link"
        size="sm"
        className="h-auto p-0 text-xs"
        disabled={unreadCount === 0}
        onClick={onMarkAllRead}
        data-testid="notifications-mark-all-read"
      >
        Mark all read
      </Button>
    </div>
  );
}

function Sections({
  groups,
  onDismiss,
  onCtaCallback,
}: {
  groups: ReturnType<typeof groupNotifications>;
  onDismiss: (id: string) => void;
  onCtaCallback?: (item: NotificationItem) => void;
}) {
  const sectionEntries: ReadonlyArray<[keyof typeof SECTION_LABELS, NotificationItem[]]> = [
    ["today", groups.today],
    ["yesterday", groups.yesterday],
    ["earlier", groups.earlier],
  ];

  return (
    <>
      {sectionEntries.map(([key, list]) =>
        list.length === 0 ? null : (
          <section
            key={key}
            className="border-b border-[var(--border)] last:border-b-0"
            data-testid={`notifications-section-${key}`}
          >
            <div className="px-4 pt-3 pb-1.5 text-[10px] font-semibold uppercase tracking-wider text-[var(--muted-foreground)]">
              {SECTION_LABELS[key]}
            </div>
            <div className="divide-y divide-[var(--border)]/60">
              {list.map((item) => (
                <NotificationRow
                  key={item.id}
                  item={item}
                  onDismiss={onDismiss}
                  onCtaCallback={onCtaCallback}
                />
              ))}
            </div>
          </section>
        ),
      )}
    </>
  );
}

function LoadingState() {
  return (
    <div className="px-4 py-10 text-center text-xs text-[var(--muted-foreground)]">
      Loading…
    </div>
  );
}

function ErrorState({ message }: { message: string }) {
  return (
    <div className="px-4 py-10 text-center text-xs text-[var(--destructive)]">{message}</div>
  );
}

function EmptyState() {
  return (
    <div
      className="px-4 py-10 text-center"
      data-testid="notifications-empty"
    >
      <p className="text-sm font-medium text-[var(--foreground)]">Nothing new.</p>
      <p className="mt-1 text-xs text-[var(--muted-foreground)]">
        You&apos;ll see system notifications here — budget alerts, OAuth renewals, drift
        warnings.
      </p>
    </div>
  );
}

