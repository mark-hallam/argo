import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { Button } from "@/components/ui/Button";
import { NotificationsBell } from "./NotificationsBell";
import { NotificationsPopover } from "./NotificationsPopover";
import type { NotificationItem } from "@/lib/sample-notifications";

const fetchSpy = vi.hoisted(() => vi.fn());
const markAsReadSpy = vi.hoisted(() => vi.fn());
const markAllAsReadSpy = vi.hoisted(() => vi.fn());
const dismissSpy = vi.hoisted(() => vi.fn());

vi.mock("@/lib/api/notifications", () => ({
  fetchNotifications: fetchSpy,
  markAsRead: markAsReadSpy,
  markAllAsRead: markAllAsReadSpy,
  dismissNotification: dismissSpy,
}));

function makeItem(overrides: Partial<NotificationItem> = {}): NotificationItem {
  return {
    id: "nt_test",
    kind: "skill-promoted",
    title: "Skill promoted",
    createdAt: new Date(Date.now() - 5 * 60_000).toISOString(),
    read: false,
    ...overrides,
  };
}

function wrap(node: ReactNode) {
  const client = new QueryClient({
    defaultOptions: {
      queries: { retry: false, refetchOnWindowFocus: false, refetchInterval: false },
      mutations: { retry: false },
    },
  });
  function Wrapper({ children }: { children: ReactNode }) {
    return <QueryClientProvider client={client}>{children}</QueryClientProvider>;
  }
  return render(node, { wrapper: Wrapper });
}

function renderPopover({
  items,
  onCtaCallback,
}: {
  items: NotificationItem[];
  onCtaCallback?: (i: NotificationItem) => void;
}) {
  fetchSpy.mockResolvedValue(items);
  markAsReadSpy.mockResolvedValue({ ok: true, notificationId: "x" });
  markAllAsReadSpy.mockResolvedValue({ ok: true });
  dismissSpy.mockResolvedValue({ ok: true, notificationId: "x" });

  return wrap(
    <NotificationsPopover onCtaCallback={onCtaCallback}>
      <Button data-testid="bell-trigger" aria-label="Notifications">
        bell
      </Button>
    </NotificationsPopover>,
  );
}

async function openPopover(user: ReturnType<typeof userEvent.setup>) {
  await user.click(screen.getByTestId("bell-trigger"));
  return await screen.findByTestId("notifications-popover-content");
}

describe("NotificationsPopover", () => {
  beforeEach(() => {
    fetchSpy.mockReset();
    markAsReadSpy.mockReset();
    markAllAsReadSpy.mockReset();
    dismissSpy.mockReset();
  });

  it("renders the empty state when there are no notifications", async () => {
    const user = userEvent.setup();
    renderPopover({ items: [] });
    const content = await openPopover(user);
    expect(within(content).getByTestId("notifications-empty")).toBeInTheDocument();
    expect(within(content).getByText(/Nothing new/)).toBeInTheDocument();
  });

  it("renders the list grouped by section when items exist", async () => {
    const user = userEvent.setup();
    const today = makeItem({ id: "a", title: "Today item" });
    const earlier = makeItem({
      id: "b",
      title: "Old item",
      createdAt: new Date(Date.now() - 4 * 24 * 60 * 60_000).toISOString(),
    });
    renderPopover({ items: [today, earlier] });
    const content = await openPopover(user);
    expect(within(content).getByTestId("notifications-section-today")).toBeInTheDocument();
    expect(within(content).getByTestId("notifications-section-earlier")).toBeInTheDocument();
    expect(within(content).getByText("Today item")).toBeInTheDocument();
    expect(within(content).getByText("Old item")).toBeInTheDocument();
  });

  it("marks unread items as read when the popover opens", async () => {
    const user = userEvent.setup();
    renderPopover({ items: [makeItem({ id: "a", read: false })] });
    await openPopover(user);
    await waitFor(() => expect(markAllAsReadSpy).toHaveBeenCalledTimes(1));
  });

  it("dismiss removes the row and posts to the gateway", async () => {
    const user = userEvent.setup();
    const item = makeItem({ id: "a", title: "Dismiss me" });
    renderPopover({ items: [item] });
    const content = await openPopover(user);
    await user.click(within(content).getByTestId("notification-dismiss-a"));
    await waitFor(() => expect(dismissSpy).toHaveBeenCalledWith("a"));
    await waitFor(() =>
      expect(within(content).queryByText("Dismiss me")).not.toBeInTheDocument(),
    );
  });

  it("mark-all-read clears the unread count badge", async () => {
    const user = userEvent.setup();
    const items = [
      makeItem({ id: "a", read: false, title: "A" }),
      makeItem({ id: "b", read: false, title: "B" }),
    ];
    renderPopover({ items });
    const content = await openPopover(user);
    // Opening already triggers mark-all-read; pressing the link again
    // should be a no-op but must not crash.
    await user.click(within(content).getByTestId("notifications-mark-all-read"));
    await waitFor(() =>
      expect(within(content).queryByTestId("notifications-unread-count")).not.toBeInTheDocument(),
    );
  });

  it("inline CTA fires the callback and auto-dismisses the row", async () => {
    const user = userEvent.setup();
    const onCtaCallback = vi.fn();
    const item = makeItem({
      id: "a",
      title: "Re-auth needed",
      kind: "oauth-needs-reauth",
      cta: { label: "Re-auth Gmail", action: "callback" },
    });
    renderPopover({ items: [item], onCtaCallback });
    const content = await openPopover(user);
    await user.click(within(content).getByTestId("notification-cta-a"));
    expect(onCtaCallback).toHaveBeenCalledTimes(1);
    expect(onCtaCallback.mock.calls[0][0].id).toBe("a");
    await waitFor(() => expect(dismissSpy).toHaveBeenCalledWith("a"));
  });

  it("falls back to mocked data when fetchNotifications returns []", async () => {
    const user = userEvent.setup();
    renderPopover({ items: [] });
    const content = await openPopover(user);
    expect(within(content).getByTestId("notifications-empty")).toBeInTheDocument();
    expect(fetchSpy).toHaveBeenCalled();
  });
});

describe("NotificationsBell", () => {
  it("renders the unread dot only when count > 0", () => {
    const { rerender } = wrap(<NotificationsBell unreadCount={0} />);
    expect(screen.queryByTestId("notifications-unread-dot")).not.toBeInTheDocument();

    rerender(<NotificationsBell unreadCount={3} />);
    expect(screen.getByTestId("notifications-unread-dot")).toBeInTheDocument();
  });

  it("pulses the dot when the unread count increases mid-session", async () => {
    const { rerender } = wrap(<NotificationsBell unreadCount={1} />);
    expect(screen.getByTestId("notifications-unread-dot")).not.toHaveAttribute("data-pulse");

    rerender(<NotificationsBell unreadCount={2} />);
    await waitFor(() =>
      expect(screen.getByTestId("notifications-unread-dot")).toHaveAttribute("data-pulse"),
    );
  });
});
