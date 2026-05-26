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
    fetchLearnedThisMonth: vi.fn(),
  };
});

import SkillsPage from "@/app/(app)/skills/page";
import * as api from "@/lib/api/skills";

function wrap(ui: ReactNode) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false, gcTime: 0 }, mutations: { retry: false } },
  });
  return <QueryClientProvider client={client}>{ui}</QueryClientProvider>;
}

beforeEach(() => {
  vi.mocked(api.fetchLearnedThisMonth).mockResolvedValue({
    learned: { since: "2026-05-01T00:00:00.000Z", count: 0, skill_ids: [] },
    source: "mock-offline",
  });
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

    expect(
      await screen.findByTestId("skills-workspace-empty"),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: /no skills installed yet/i }),
    ).toBeInTheDocument();
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
