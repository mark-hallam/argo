import { render, screen, fireEvent, waitFor, act } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import NotifyChannelsPage from "@/app/(app)/settings/notify-channels/page";
import { NOTIFY_CHANNELS, fallbackStatusFor } from "@/lib/notify-channels/registry";

/**
 * Render-and-snapshot guards on `/settings/notify-channels`.
 *
 * The 11-channel inventory is pinned by
 * `crates/argo-channels/tests/channels_v1_inventory_pin_test.rs`; this
 * test mirrors that pin on the web side so a delete/rename surfaces in
 * both languages at once.
 */

const originalFetch = globalThis.fetch;

afterEach(() => {
  globalThis.fetch = originalFetch;
  vi.clearAllMocks();
});

beforeEach(() => {
  globalThis.fetch = vi
    .fn()
    .mockResolvedValue(new Response("", { status: 404 }));
});

async function renderPage() {
  // Drain the initial `useEffect` doctor fetch so the loading state
  // settles before assertions run.
  await act(async () => {
    render(<NotifyChannelsPage />);
  });
}

describe("/settings/notify-channels page", () => {
  it("renders the hero, search bar, banner, and all 11 channel cards", async () => {
    await renderPage();

    expect(screen.getByTestId("notify-channels-page")).toBeInTheDocument();
    expect(screen.getByTestId("notify-search")).toBeInTheDocument();
    expect(screen.getByTestId("notify-pro-banner")).toBeInTheDocument();
    expect(screen.getByTestId("notify-count")).toHaveTextContent("11 total");

    for (const c of NOTIFY_CHANNELS) {
      expect(
        screen.getByTestId(`notify-card-${c.id}`),
        `missing card for ${c.id}`,
      ).toBeInTheDocument();
    }
  });

  it("pins the Pro-tier banner to slack, imessage, and email", async () => {
    await renderPage();

    const banner = screen.getByTestId("notify-pro-banner");
    expect(banner).toHaveTextContent(/Slack/);
    expect(banner).toHaveTextContent(/iMessage/);
    expect(banner).toHaveTextContent(/Email/);
  });

  it("renders a sales-promise warning note on the iMessage card", async () => {
    await renderPage();

    expect(screen.getByTestId("notify-card-note-imessage")).toBeInTheDocument();
  });

  it("falls back to the static audit verdict when the gateway 404s", async () => {
    await renderPage();

    const fallbackBanner = await screen.findByTestId("notify-fallback-banner");
    expect(fallbackBanner).toHaveTextContent(/audit fallback/i);

    // Live verdicts → OK; deferred → WARN.
    expect(screen.getByTestId("notify-card-slack")).toHaveAttribute(
      "data-status",
      fallbackStatusFor("live"),
    );
    expect(screen.getByTestId("notify-card-slack")).toHaveAttribute("data-source", "fallback");
    expect(screen.getByTestId("notify-card-imessage")).toHaveAttribute(
      "data-status",
      fallbackStatusFor("deferred"),
    );
  });

  it("narrows the grid when the operator searches by env-var name", async () => {
    await renderPage();

    fireEvent.change(screen.getByTestId("notify-search"), {
      target: { value: "BLUE_BUBBLES" },
    });

    expect(screen.getByTestId("notify-card-imessage")).toBeInTheDocument();
    expect(screen.queryByTestId("notify-card-slack")).toBeNull();
  });

  it("shows the empty-filter message when nothing matches", async () => {
    await renderPage();

    fireEvent.change(screen.getByTestId("notify-search"), {
      target: { value: "no-such-channel-zzz" },
    });

    expect(screen.getByTestId("notify-empty-filter")).toBeInTheDocument();
  });

  it("expands env vars on click and shows BLUE_BUBBLES_URL for iMessage", async () => {
    await renderPage();

    fireEvent.click(screen.getByTestId("notify-card-expand-imessage"));

    const envs = screen.getByTestId("notify-card-envs-imessage");
    expect(envs).toHaveTextContent("BLUE_BUBBLES_URL");
    expect(envs).toHaveTextContent("BLUE_BUBBLES_PASSWORD");
  });

  it("re-fetches the doctor endpoint when Run probe is clicked", async () => {
    await renderPage();

    const initialCalls = vi.mocked(globalThis.fetch).mock.calls.length;

    fireEvent.click(screen.getByTestId("notify-run-probe-all"));

    await waitFor(() => {
      expect(vi.mocked(globalThis.fetch).mock.calls.length).toBeGreaterThan(initialCalls);
    });
  });

  it("uses live probe data when /api/doctor/channels returns 200", async () => {
    globalThis.fetch = vi.fn().mockResolvedValue(
      new Response(
        JSON.stringify({
          email: { status: "OK", detail: "Postmark reachable" },
          slack: { status: "MISSING", detail: "no token" },
        }),
        { status: 200, headers: { "content-type": "application/json" } },
      ),
    );

    await renderPage();

    await waitFor(() => {
      expect(screen.getByTestId("notify-card-email")).toHaveAttribute("data-source", "live");
    });

    expect(screen.getByTestId("notify-card-email")).toHaveAttribute("data-status", "OK");
    expect(screen.getByTestId("notify-card-slack")).toHaveAttribute("data-status", "MISSING");
  });
});
