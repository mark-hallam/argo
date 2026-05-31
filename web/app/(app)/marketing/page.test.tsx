import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const marketingApi = vi.hoisted(() => ({
  draftResponse: vi.fn(),
}));

vi.mock("@/lib/api/marketing", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/lib/api/marketing")>();
  return { ...actual, ...marketingApi };
});

import MarketingPage from "./page";
import { ApiError } from "@/lib/api/types";

describe("/marketing page", () => {
  beforeEach(() => {
    marketingApi.draftResponse.mockReset();
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it("renders the console with reply + X selected by default and no result", () => {
    render(<MarketingPage />);
    expect(screen.getByTestId("mk-mode-reply")).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByTestId("mk-channel-x")).toHaveAttribute("aria-pressed", "true");
    expect(screen.queryByTestId("mk-result")).not.toBeInTheDocument();
  });

  it("drafts a reply for X and shows the Allow draft + approvals link", async () => {
    marketingApi.draftResponse.mockResolvedValueOnce({
      agent: "Marcus",
      responsibility: "marketing",
      mode: "reply",
      channel: "x",
      draft: "Autocomplete doesn't own a backlog. Teammates do.",
      disposition: "Allow",
      source: "llm",
      model: "claude-sonnet-4-6",
      approval_id: "01J-approval",
      tokens: { input: 100, output: 30 },
    });
    render(<MarketingPage />);
    fireEvent.change(screen.getByTestId("mk-text"), {
      target: { value: "AI agents are just fancy autocomplete." },
    });
    fireEvent.click(screen.getByTestId("mk-submit"));
    await waitFor(() => {
      expect(screen.getByTestId("mk-result")).toBeInTheDocument();
    });
    expect(screen.getByTestId("mk-draft")).toHaveTextContent(
      "Autocomplete doesn't own a backlog",
    );
    expect(screen.getByTestId("mk-pill-disposition")).toHaveTextContent(/brand-safe/i);
    expect(screen.getByTestId("mk-filed")).toBeInTheDocument();
    expect(marketingApi.draftResponse).toHaveBeenCalledWith({
      mode: "reply",
      channel: "x",
      source: "AI agents are just fancy autocomplete.",
      topic: undefined,
    });
  });

  it("switches to article + LinkedIn and sends topic, not source", async () => {
    marketingApi.draftResponse.mockResolvedValueOnce({
      agent: "Marcus",
      responsibility: "marketing",
      mode: "article",
      channel: "linkedin",
      draft: "Ideas are the moat. — Mark + the team",
      disposition: "Allow",
      source: "llm",
    });
    render(<MarketingPage />);
    fireEvent.click(screen.getByTestId("mk-mode-article"));
    fireEvent.click(screen.getByTestId("mk-channel-linkedin"));
    fireEvent.change(screen.getByTestId("mk-text"), {
      target: { value: "Why ideas are the human moat" },
    });
    fireEvent.click(screen.getByTestId("mk-submit"));
    await waitFor(() => {
      expect(screen.getByTestId("mk-result")).toBeInTheDocument();
    });
    expect(marketingApi.draftResponse).toHaveBeenCalledWith({
      mode: "article",
      channel: "linkedin",
      source: undefined,
      topic: "Why ideas are the human moat",
    });
  });

  it("renders a held-for-review pill on Escalate", async () => {
    marketingApi.draftResponse.mockResolvedValueOnce({
      agent: "Marcus",
      responsibility: "marketing",
      mode: "article",
      channel: "linkedin",
      draft: "[Held for founder review — guarded topic (revenue)…]",
      disposition: "Escalate",
      source: "llm",
    });
    render(<MarketingPage />);
    fireEvent.change(screen.getByTestId("mk-text"), {
      target: { value: "our revenue numbers" },
    });
    fireEvent.click(screen.getByTestId("mk-submit"));
    await waitFor(() => {
      expect(screen.getByTestId("mk-pill-disposition")).toHaveTextContent(/held for review/i);
    });
  });

  it("surfaces an error banner on a gateway failure", async () => {
    marketingApi.draftResponse.mockRejectedValueOnce(
      new ApiError(503, "marketing llm transport: down", "service_unavailable"),
    );
    render(<MarketingPage />);
    fireEvent.change(screen.getByTestId("mk-text"), { target: { value: "hi" } });
    fireEvent.click(screen.getByTestId("mk-submit"));
    await waitFor(() => {
      expect(screen.getByTestId("mk-error")).toBeInTheDocument();
    });
    expect(screen.getByTestId("mk-error")).toHaveTextContent(/down/i);
  });
});
