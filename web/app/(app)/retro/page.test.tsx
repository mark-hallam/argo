import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import RetroIndexPage from "@/app/(app)/retro/page";
import type { Retro, RetrosListResult } from "@/lib/api/retros";

vi.mock("next/navigation", () => ({
  useRouter: () => ({
    push: vi.fn(),
    replace: vi.fn(),
    prefetch: vi.fn(),
    refresh: vi.fn(),
    back: vi.fn(),
    forward: vi.fn(),
  }),
  usePathname: () => "/retro",
  useSearchParams: () => new URLSearchParams(),
}));

vi.mock("@/lib/api/retros", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api/retros")>(
    "@/lib/api/retros",
  );
  return {
    ...actual,
    fetchRetrosIndex: vi.fn(),
  };
});

import { fetchRetrosIndex } from "@/lib/api/retros";
const mockedFetchRetros = vi.mocked(fetchRetrosIndex);

function sampleRetro(overrides: Partial<Retro> = {}): Retro {
  return {
    retroId: "retro_1",
    workerId: "01900000-0000-7000-8000-000000000001",
    windowStart: 1_700_000_000,
    windowEnd: 1_700_604_800,
    metrics: {
      actions_total: 4,
      actions_approved: 3,
      actions_refused: 1,
      escalations: 0,
      learned_skills_added: 0,
      error_count_by_category: {},
    },
    sections: {
      surface_metrics: "",
      what_went_well: "Great escalation discipline this week.",
      surprises: "",
      what_to_change: "",
      action_items: "",
    },
    createdAt: 1_700_605_000,
    finalizedAt: null,
    finalized: false,
    ...overrides,
  };
}

function Wrapper() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: 0 } } });
  return (
    <QueryClientProvider client={client}>
      <RetroIndexPage />
    </QueryClientProvider>
  );
}

beforeEach(() => {
  mockedFetchRetros.mockResolvedValue({
    items: [],
    source: "mock-offline",
  } satisfies RetrosListResult);
});

afterEach(() => {
  vi.clearAllMocks();
});

describe("RetroIndexPage", () => {
  it("renders the empty-state when no retros come back", async () => {
    render(<Wrapper />);
    expect(await screen.findByTestId("retro-index-empty")).toBeInTheDocument();
  });

  it("renders a card per retro and a finalised badge for locked rows", async () => {
    mockedFetchRetros.mockResolvedValue({
      items: [
        sampleRetro({ retroId: "r1" }),
        sampleRetro({
          retroId: "r2",
          finalized: true,
          finalizedAt: 1_700_605_500,
        }),
      ],
      source: "live",
    });
    render(<Wrapper />);
    expect(await screen.findByTestId("retro-card-r1")).toBeInTheDocument();
    expect(screen.getByTestId("retro-card-r2")).toBeInTheDocument();
    expect(screen.getByTestId("retro-card-finalized")).toBeInTheDocument();
    expect(screen.getByTestId("retro-card-draft")).toBeInTheDocument();
  });

  it("debounce-updates the q query param when the operator types in the search box", async () => {
    render(<Wrapper />);
    const input = await screen.findByTestId("retro-index-search");
    await userEvent.type(input, "escalation");
    expect((input as HTMLInputElement).value).toBe("escalation");
  });

  it("renders a preview snippet on each card, highlighting the first non-empty section", async () => {
    mockedFetchRetros.mockResolvedValue({
      items: [sampleRetro({ retroId: "r1" })],
      source: "mock-offline",
    });
    render(<Wrapper />);
    const preview = await screen.findByTestId("retro-card-preview-r1");
    expect(preview).toHaveTextContent(/escalation discipline/i);
  });
});
