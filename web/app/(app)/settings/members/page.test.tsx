import { act } from "react";
import { hydrateRoot, type Root } from "react-dom/client";
import { renderToString } from "react-dom/server";
import { render, screen, within } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { MemberExpiryPill } from "@/components/skycot/members/MemberExpiryPill";

const workspaceMembersApi = vi.hoisted(() => ({
  inviteWorkspaceMember: vi.fn(),
  listWorkspaceMembers: vi.fn(async () => {
    throw new Error("gateway offline");
  }),
}));

vi.mock("@/lib/api/workspace-members", () => workspaceMembersApi);

import MembersPage from "./page";

function makeQueryClient() {
  return new QueryClient({
    defaultOptions: {
      queries: { retry: false, staleTime: 0 },
      mutations: { retry: false },
    },
  });
}

function MembersHarness() {
  return (
    <QueryClientProvider client={makeQueryClient()}>
      <MembersPage />
    </QueryClientProvider>
  );
}

describe("settings members page", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-05-25T12:00:00Z"));
    workspaceMembersApi.listWorkspaceMembers.mockRejectedValue(new Error("gateway offline"));
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it("renders the locked member-list chrome with role and expiry pills", () => {
    render(<MembersHarness />);

    expect(screen.getByTestId("members-list")).toBeInTheDocument();
    expect(screen.getByTestId("member-row-mbr_naomi")).toHaveClass("member-row");
    expect(screen.getAllByTestId("member-expiry-pill")).toHaveLength(3);
    expect(within(screen.getByTestId("member-row-mbr_naomi")).getByText("Guest")).toHaveClass(
      "member-role-pill",
    );
    expect(
      Array.from(document.querySelectorAll("style")).some((style) =>
        style.textContent?.includes(".member-row"),
      ),
    ).toBe(true);
  });

  it("hydrates expiry pills without locale-sensitive title mismatches", async () => {
    const consoleError = vi.spyOn(console, "error").mockImplementation(() => {});
    const localeFormatter = vi
      .spyOn(Date.prototype, "toLocaleDateString")
      .mockReturnValue("server-only date");
    const element = (
      <MemberExpiryPill expiresAt="2026-06-24T00:00:00.000Z" />
    );
    const html = renderToString(element);
    const host = document.createElement("div");
    host.innerHTML = html;
    document.body.appendChild(host);

    localeFormatter.mockReturnValue("client-only date");

    let root: Root | null = null;
    await act(async () => {
      root = hydrateRoot(host, element);
    });

    const pill = screen.getByTestId("member-expiry-pill");
    expect(pill).toHaveAttribute("title", "Access expires on 24 June 2026");
    expect(localeFormatter).not.toHaveBeenCalled();
    const errors = consoleError.mock.calls.map((call) => call.join(" ")).join("\n");
    expect(errors).not.toContain("hydrated");
    expect(errors).not.toContain("Hydration failed");

    await act(async () => {
      root?.unmount();
    });
  });
});
