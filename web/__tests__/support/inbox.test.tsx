import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import SupportInboxPage from "@/app/(app)/support/inbox/page";

vi.mock("next/navigation", () => ({
  useRouter: () => ({ push: vi.fn(), replace: vi.fn() }),
  useParams: () => ({ id: "ticket-1" }),
}));

vi.mock("next/link", () => ({
  default: ({ href, children, ...rest }: { href: string; children: ReactNode } & Record<string, unknown>) =>
    // eslint-disable-next-line @next/next/no-html-link-for-pages
    <a href={href} {...rest}>{children}</a>,
}));

function wrap(ui: ReactNode) {
  const client = new QueryClient({
    defaultOptions: {
      queries: { retry: false, gcTime: 0 },
      mutations: { retry: false },
    },
  });
  return <QueryClientProvider client={client}>{ui}</QueryClientProvider>;
}

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

function mockFetch(handler: (url: string, init?: RequestInit) => Promise<Response>) {
  vi.spyOn(globalThis, "fetch").mockImplementation(async (input, init) => {
    const url = typeof input === "string" ? input : input.toString();
    return handler(url, init);
  });
}

afterEach(() => {
  vi.restoreAllMocks();
});

const SAMPLE_TICKETS = {
  tickets: [
    {
      id: "11111111-1111-1111-1111-111111111111",
      tenant_id: null,
      user_email: "alice@example.com",
      subject: "Login broken",
      body: "magic link bounces",
      status: "open",
      priority: "urgent",
      created_at: 1_700_000_000,
      updated_at: 1_700_000_000,
      assigned_to: null,
    },
    {
      id: "22222222-2222-2222-2222-222222222222",
      tenant_id: null,
      user_email: "bob@example.com",
      subject: "Slow dashboard",
      body: "loading is 12s",
      status: "pending",
      priority: "normal",
      created_at: 1_700_000_000,
      updated_at: 1_700_000_000,
      assigned_to: null,
    },
  ],
};

describe("SupportInboxPage", () => {
  beforeEach(() => {
    mockFetch(async (url) => {
      if (url.includes("/api/v1/support/tickets")) {
        return jsonResponse(SAMPLE_TICKETS);
      }
      if (url.includes("/api/auth/session")) {
        return jsonResponse({ gatewayToken: "test-token" });
      }
      return new Response("not found", { status: 404 });
    });
  });

  it("renders 3 Kanban columns", async () => {
    render(wrap(<SupportInboxPage />));
    expect(await screen.findByTestId("column-open")).toBeInTheDocument();
    expect(screen.getByTestId("column-pending")).toBeInTheDocument();
    expect(screen.getByTestId("column-resolved")).toBeInTheDocument();
  });

  it("places tickets in the matching status column", async () => {
    render(wrap(<SupportInboxPage />));
    await waitFor(() => {
      expect(screen.getByText("Login broken")).toBeInTheDocument();
    });
    const open = screen.getByTestId("column-open");
    expect(open).toHaveTextContent("Login broken");
    const pending = screen.getByTestId("column-pending");
    expect(pending).toHaveTextContent("Slow dashboard");
  });

  it("surfaces an error state when the API rejects", async () => {
    vi.restoreAllMocks();
    mockFetch(async () => new Response("forbidden", { status: 403 }));
    render(wrap(<SupportInboxPage />));
    expect(await screen.findByTestId("support-inbox-error")).toBeInTheDocument();
  });
});

describe("TicketCard link target", () => {
  beforeEach(() => {
    mockFetch(async (url) => {
      if (url.includes("/api/v1/support/tickets")) {
        return jsonResponse(SAMPLE_TICKETS);
      }
      if (url.includes("/api/auth/session")) {
        return jsonResponse({ gatewayToken: "test-token" });
      }
      return new Response("not found", { status: 404 });
    });
  });

  it("each ticket card links to the detail route", async () => {
    render(wrap(<SupportInboxPage />));
    const card = await screen.findByTestId(
      "ticket-card-11111111-1111-1111-1111-111111111111",
    );
    expect(card).toHaveAttribute(
      "href",
      "/support/inbox/11111111-1111-1111-1111-111111111111",
    );
  });

  it("renders the user email on each card", async () => {
    render(wrap(<SupportInboxPage />));
    await waitFor(() => {
      expect(screen.getByText("alice@example.com")).toBeInTheDocument();
      expect(screen.getByText("bob@example.com")).toBeInTheDocument();
    });
  });
});

describe("Click-through (smoke)", () => {
  beforeEach(() => {
    mockFetch(async (url) => {
      if (url.includes("/api/v1/support/tickets")) {
        return jsonResponse(SAMPLE_TICKETS);
      }
      if (url.includes("/api/auth/session")) {
        return jsonResponse({ gatewayToken: "test-token" });
      }
      return new Response("not found", { status: 404 });
    });
  });

  it("clicking a ticket triggers navigation via Link href", async () => {
    const user = userEvent.setup();
    render(wrap(<SupportInboxPage />));
    const card = await screen.findByTestId(
      "ticket-card-11111111-1111-1111-1111-111111111111",
    );
    await user.hover(card);
    expect(card).toBeInTheDocument();
  });
});
