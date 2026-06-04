import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

// Mock the API client BEFORE importing the page — the page transitively
// imports `lib/api/skills`, which would otherwise call the real
// fetcher and resolve via seed-fallback during the test.
vi.mock("@/lib/api/skills", async () => {
  const actual =
    await vi.importActual<typeof import("@/lib/api/skills")>(
      "@/lib/api/skills",
    );
  return {
    ...actual,
    fetchSkills: vi.fn(),
    fetchSkillSearch: vi.fn(),
    fetchLearnedThisMonth: vi.fn(),
  };
});

import SkillsPage from "@/app/(app)/skills/page";
import * as api from "@/lib/api/skills";
import type {
  SkillSearchParams,
  SkillSearchResponse,
  SkillSearchResult,
} from "@/lib/api/skills";

function wrap(ui: ReactNode) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false, gcTime: 0 }, mutations: { retry: false } },
  });
  return <QueryClientProvider client={client}>{ui}</QueryClientProvider>;
}

const categoryOf = (name: string): string => {
  const end = name.search(/[.:-]/);
  return end === -1 ? name : name.slice(0, end);
};

/**
 * Server-side-search stand-in: ranks the seed catalog with the same
 * name/description substring + category filter the gateway applies, so
 * the page's server-backed search path is exercised end-to-end without
 * a live gateway. (The real ranking + facets are unit-tested in the
 * Rust route; here we only need a faithful enough shape.)
 */
function searchSeed(params: SkillSearchParams): SkillSearchResponse {
  const q = (params.q ?? "").trim().toLowerCase();
  let rows = api.SEED_SKILLS;
  if (params.category) {
    rows = rows.filter(
      (s) => categoryOf(s.name).toLowerCase() === params.category!.toLowerCase(),
    );
  }
  const matched =
    q.length === 0
      ? rows
      : rows.filter(
          (s) =>
            s.name.toLowerCase().includes(q) ||
            s.description.toLowerCase().includes(q),
        );
  const results: SkillSearchResult[] = matched.map((s) => ({
    ...s,
    category: categoryOf(s.name),
  }));
  return {
    query: params.q ?? "",
    count: results.length,
    results,
    facets: { category: [], source: [], trust: [] },
  };
}

beforeEach(() => {
  vi.mocked(api.fetchLearnedThisMonth).mockResolvedValue({
    learned: { since: "2026-05-01T00:00:00.000Z", count: 0, skill_ids: [] },
    source: "mock-offline",
  });
  // Default: faithful server-search over the seed catalog.
  vi.mocked(api.fetchSkillSearch).mockImplementation(async (params) => ({
    response: searchSeed(params),
    source: "mock-offline",
  }));
});

afterEach(() => {
  vi.clearAllMocks();
});

describe("/skills marketplace page", () => {
  it("renders the marketplace hero on a healthy gateway response", async () => {
    vi.mocked(api.fetchSkills).mockResolvedValue({
      skills: [
        {
          id: "s1",
          name: "approval.route",
          version: "0.1.0",
          description: "Route approvals to the right human.",
          layer: "library",
          provenance_source: "seed",
          trust_grade: "seed",
        },
      ],
      source: "gateway",
    });

    render(wrap(<SkillsPage />));

    expect(
      await screen.findByRole("heading", { level: 1, name: /every skill your team/i }),
    ).toBeInTheDocument();
    await waitFor(() => {
      expect(screen.getByTestId("skills-count")).toHaveTextContent(/1 skills/);
    });
  });

  it("renders the workspace-empty state when the gateway returns zero skills", async () => {
    vi.mocked(api.fetchSkills).mockResolvedValue({
      skills: [],
      source: "gateway",
    });

    render(wrap(<SkillsPage />));

    const empty = await screen.findByTestId("skills-workspace-empty");
    expect(empty).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: /no skills yet/i }),
    ).toBeInTheDocument();
    // Truth-bounded, future-framed copy (WS2) — no fabricated counts.
    expect(
      screen.getByText(/your teammates earn and share skills as they work/i),
    ).toBeInTheDocument();
    // CTA routes forward to the first-hire flow (teammates earn skills).
    expect(
      screen.getByRole("link", { name: /hire your first teammate/i }),
    ).toHaveAttribute("href", "/onboarding/first-hire");
    expect(screen.queryByTestId("skills-error")).not.toBeInTheDocument();
  });

  it("renders the error empty-state when the registry fetch throws", async () => {
    vi.mocked(api.fetchSkills).mockRejectedValue(
      new Error("gateway unreachable"),
    );

    render(wrap(<SkillsPage />));

    expect(await screen.findByTestId("skills-error")).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: /the skill registry didn’t answer/i }),
    ).toBeInTheDocument();
  });

  it("routes the hero search through the server endpoint and surfaces the Scene 4 anchor", async () => {
    vi.mocked(api.fetchSkills).mockResolvedValue({
      skills: api.SEED_SKILLS,
      source: "mock-offline",
    });

    render(wrap(<SkillsPage />));

    await waitFor(() => {
      expect(screen.getAllByTestId("skill-card")).toHaveLength(api.SEED_SKILLS.length);
    });

    const searchInput = screen.getByTestId("skills-search");

    // Full runbook phrase: must surface the one anchor card via the
    // server-side search endpoint.
    fireEvent.change(searchInput, { target: { value: "competitor pricing scrape" } });
    await waitFor(() => {
      expect(screen.getAllByTestId("skill-card")).toHaveLength(1);
    });
    expect(
      screen.getByRole("heading", { name: /pricing competitor scrape/i }),
    ).toBeInTheDocument();
    expect(screen.queryByTestId("skills-empty")).not.toBeInTheDocument();
    // The server endpoint was actually consulted with the typed query.
    expect(api.fetchSkillSearch).toHaveBeenCalledWith(
      expect.objectContaining({ q: "competitor pricing scrape" }),
    );

    // Single-word regression: "competitor" still returns the one card.
    fireEvent.change(searchInput, { target: { value: "competitor" } });
    await waitFor(() => {
      expect(screen.getAllByTestId("skill-card")).toHaveLength(1);
    });

    // Empty query: back to the full catalog from useSkills.
    fireEvent.change(searchInput, { target: { value: "" } });
    await waitFor(() => {
      expect(screen.getAllByTestId("skill-card")).toHaveLength(api.SEED_SKILLS.length);
    });
  });

  describe("count pill copy", () => {
    it("reads '<N> skills' when no filter is active", async () => {
      vi.mocked(api.fetchSkills).mockResolvedValue({
        skills: api.SEED_SKILLS,
        source: "mock-offline",
      });

      render(wrap(<SkillsPage />));

      await waitFor(() => {
        expect(screen.getByTestId("skills-count")).toHaveTextContent(
          new RegExp(`^${api.SEED_SKILLS.length} skills$`),
        );
      });
    });

    it("reads 'N of M skills' when a server search partially matches", async () => {
      vi.mocked(api.fetchSkills).mockResolvedValue({
        skills: api.SEED_SKILLS,
        source: "mock-offline",
      });

      render(wrap(<SkillsPage />));

      await waitFor(() => {
        expect(screen.getAllByTestId("skill-card")).toHaveLength(api.SEED_SKILLS.length);
      });

      fireEvent.change(screen.getByTestId("skills-search"), {
        target: { value: "competitor" },
      });

      await waitFor(() => {
        expect(screen.getByTestId("skills-count")).toHaveTextContent(
          new RegExp(`^1 of ${api.SEED_SKILLS.length} skills$`),
        );
      });
    });

    it("reads '0 of M skills' when the server search excludes everything", async () => {
      vi.mocked(api.fetchSkills).mockResolvedValue({
        skills: api.SEED_SKILLS,
        source: "mock-offline",
      });

      render(wrap(<SkillsPage />));

      await waitFor(() => {
        expect(screen.getAllByTestId("skill-card")).toHaveLength(api.SEED_SKILLS.length);
      });

      fireEvent.change(screen.getByTestId("skills-search"), {
        target: { value: "zzz-no-match-xyzzy" },
      });

      await waitFor(() => {
        expect(screen.getByTestId("skills-count")).toHaveTextContent(
          new RegExp(`^0 of ${api.SEED_SKILLS.length} skills$`),
        );
      });
    });
  });

  it("renders a server-reported facet control and filters by category", async () => {
    vi.mocked(api.fetchSkills).mockResolvedValue({
      skills: api.SEED_SKILLS,
      source: "mock-offline",
    });
    // Server reports an invoice category facet on the empty-query call.
    vi.mocked(api.fetchSkillSearch).mockImplementation(async (params) => {
      const response = searchSeed(params);
      response.facets = {
        category: [{ value: "invoice", count: 2 }],
        source: [],
        trust: [{ value: "published_verified", count: 3 }],
      };
      return { response, source: "mock-offline" };
    });

    render(wrap(<SkillsPage />));

    // The facet control renders from the server's facet values.
    const invoiceFacet = await screen.findByTestId("skills-facet-category-invoice");
    expect(invoiceFacet).toBeInTheDocument();
    expect(screen.getByTestId("skills-facet-trust-published_verified")).toBeInTheDocument();

    // Clicking the category facet drives a server search scoped to it.
    fireEvent.click(invoiceFacet);
    await waitFor(() => {
      expect(api.fetchSkillSearch).toHaveBeenCalledWith(
        expect.objectContaining({ category: "invoice" }),
      );
    });
  });

  it("keeps the learned-this-month filter populated by seed fallback skills", async () => {
    vi.mocked(api.fetchSkills).mockResolvedValue({
      skills: api.SEED_SKILLS,
      source: "mock-offline",
    });
    vi.mocked(api.fetchLearnedThisMonth).mockResolvedValue({
      learned: {
        since: "2026-05-01T00:00:00.000Z",
        count: 2,
        skill_ids: ["support.triage", "lead.enrich"],
      },
      source: "mock-offline",
    });

    render(wrap(<SkillsPage />));

    await waitFor(() => {
      expect(screen.getAllByTestId("skill-card")).toHaveLength(api.SEED_SKILLS.length);
    });

    fireEvent.click(screen.getByTestId("skills-filter-learned-this-month"));

    await waitFor(() => {
      expect(screen.getAllByTestId("skill-card")).toHaveLength(2);
    });
    expect(
      screen.getByRole("heading", { name: /support triage/i }),
    ).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: /lead enrich/i })).toBeInTheDocument();
    expect(
      screen.queryByRole("heading", { name: /approval route/i }),
    ).not.toBeInTheDocument();
  });
});
