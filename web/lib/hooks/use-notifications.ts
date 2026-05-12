"use client";

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  dismissNotification,
  fetchNotifications,
  markAllAsRead,
  markAsRead,
} from "@/lib/api/notifications";
import type { NotificationItem } from "@/lib/sample-notifications";

const NOTIFICATIONS_KEY = ["notifications"] as const;

/**
 * The notifications inbox feeding the bell popover.
 *
 * 60s polling when the tab is focused; polling is suspended when the
 * tab is backgrounded (matches "polling cadence" notes in the brief).
 */
export function useNotifications() {
  return useQuery({
    queryKey: NOTIFICATIONS_KEY,
    queryFn: fetchNotifications,
    refetchInterval: 60_000,
    refetchIntervalInBackground: false,
    refetchOnWindowFocus: true,
  });
}

/** Mark a single notification as read. Optimistic — rolls back on error. */
export function useMarkAsRead() {
  const qc = useQueryClient();

  return useMutation({
    mutationFn: (notificationId: string) => markAsRead(notificationId),
    onMutate: async (notificationId: string) => {
      await qc.cancelQueries({ queryKey: NOTIFICATIONS_KEY });
      const prev = qc.getQueryData<NotificationItem[]>(NOTIFICATIONS_KEY);
      qc.setQueryData<NotificationItem[]>(NOTIFICATIONS_KEY, (items) =>
        (items ?? []).map((n) => (n.id === notificationId ? { ...n, read: true } : n)),
      );
      return { prev };
    },
    onError: (_err, _vars, ctx) => {
      if (ctx?.prev) qc.setQueryData(NOTIFICATIONS_KEY, ctx.prev);
    },
  });
}

/** Mark every notification in the cached list as read. */
export function useMarkAllAsRead() {
  const qc = useQueryClient();

  return useMutation({
    mutationFn: markAllAsRead,
    onMutate: async () => {
      await qc.cancelQueries({ queryKey: NOTIFICATIONS_KEY });
      const prev = qc.getQueryData<NotificationItem[]>(NOTIFICATIONS_KEY);
      qc.setQueryData<NotificationItem[]>(NOTIFICATIONS_KEY, (items) =>
        (items ?? []).map((n) => ({ ...n, read: true })),
      );
      return { prev };
    },
    onError: (_err, _vars, ctx) => {
      if (ctx?.prev) qc.setQueryData(NOTIFICATIONS_KEY, ctx.prev);
    },
  });
}

/** Convenience selector: live unread count. */
export function useUnreadNotificationCount(): number {
  const query = useNotifications();
  return (query.data ?? []).filter((n) => !n.read).length;
}

/** Permanently remove a notification (audit-logged server-side). */
export function useDismiss() {
  const qc = useQueryClient();

  return useMutation({
    mutationFn: (notificationId: string) => dismissNotification(notificationId),
    onMutate: async (notificationId: string) => {
      await qc.cancelQueries({ queryKey: NOTIFICATIONS_KEY });
      const prev = qc.getQueryData<NotificationItem[]>(NOTIFICATIONS_KEY);
      qc.setQueryData<NotificationItem[]>(NOTIFICATIONS_KEY, (items) =>
        (items ?? []).filter((n) => n.id !== notificationId),
      );
      return { prev };
    },
    onError: (_err, _vars, ctx) => {
      if (ctx?.prev) qc.setQueryData(NOTIFICATIONS_KEY, ctx.prev);
    },
  });
}
