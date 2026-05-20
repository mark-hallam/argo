/**
 * Notifications API client.
 *
 * Routes through the typed `lib/api/client.ts` wrapper. The list +
 * dismiss endpoints landed via `web-gateway-endpoints-notifications-batch`;
 * `markAsRead`/`markAllAsRead` are still 404 until a follow-up brief
 * lands them server-side. The 404 + NetworkError fallbacks below keep
 * the bell-popover demo working in dev without a gateway running.
 *
 * Acceptance contract:
 * - `fetchNotifications` returns `NotificationItem[]` (the gateway
 *   ships an envelope `{ items, has_more, cursor }`; this wrapper
 *   unwraps `items` for the existing callsites).
 * - `markAsRead`, `markAllAsRead`, `dismissNotification` return
 *   success envelopes.
 * - On 404 from the gateway (route not yet implemented), fall back
 *   to mock data. Other ApiError statuses propagate.
 * - On NetworkError (gateway unreachable), also fall back to mock
 *   data — the dev story is "you don't need a gateway running."
 */

import { apiGet, apiPost } from "@/lib/api/client";
import { ApiError } from "@/lib/api/types";
import { SAMPLE_NOTIFICATIONS, type NotificationItem } from "@/lib/sample-notifications";

interface NotificationsEnvelope {
  items: NotificationItem[];
  has_more: boolean;
  cursor: number | null;
}

function isMissingOrUnreachable(err: unknown): boolean {
  if (err instanceof ApiError) return err.status === 404;
  return err instanceof TypeError;
}

export async function fetchNotifications(): Promise<NotificationItem[]> {
  try {
    const envelope = await apiGet<NotificationsEnvelope>("/api/v1/notifications");
    return envelope.items;
  } catch (err) {
    if (isMissingOrUnreachable(err)) {
      await new Promise((r) => setTimeout(r, 100));
      return SAMPLE_NOTIFICATIONS;
    }
    throw err;
  }
}

export async function markAsRead(
  notificationId: string,
): Promise<{ ok: true; notificationId: string }> {
  try {
    return await apiPost<{ ok: true; notificationId: string }>(
      `/api/v1/notifications/${notificationId}/read`,
    );
  } catch (err) {
    if (isMissingOrUnreachable(err)) {
      await new Promise((r) => setTimeout(r, 40));
      return { ok: true, notificationId };
    }
    throw err;
  }
}

export async function markAllAsRead(): Promise<{ ok: true }> {
  try {
    return await apiPost<{ ok: true }>("/api/v1/notifications/read-all");
  } catch (err) {
    if (isMissingOrUnreachable(err)) {
      await new Promise((r) => setTimeout(r, 40));
      return { ok: true };
    }
    throw err;
  }
}

export async function dismissNotification(
  notificationId: string,
): Promise<{ ok: true; notificationId: string }> {
  try {
    return await apiPost<{ ok: true; notificationId: string }>(
      `/api/v1/notifications/${notificationId}/dismiss`,
    );
  } catch (err) {
    if (isMissingOrUnreachable(err)) {
      await new Promise((r) => setTimeout(r, 40));
      return { ok: true, notificationId };
    }
    throw err;
  }
}
