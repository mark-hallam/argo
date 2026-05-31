import { render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { SkillVersionHistoryCard } from "./SkillVersionHistoryCard";
import type { SkillVersionsResponse } from "@/lib/api/skills";

/**
 * Pins that the card renders the REAL version timeline + eval-delta
 * changelog from `/api/v1/skills/<id>/versions`, and falls back to an
 * honest empty / single-version state — never a fabricated history.
 */

const MULTI: SkillVersionsResponse = {
  skill_name: "pricing.competitor_scrape",
  version_count: 2,
  versions: [
    {
      id: "id-2",
      version: "2.0.0",
      trust_grade: "tenant",
      superseded: false,
      lifecycle_state: "active",
      created_at: "2026-05-24T09:00:00.000Z",
      eval: {
        pass_rate: 0.92,
        previous_pass_rate: 0.8,
        delta_pp: 12.0,
        run_count: 2,
        decision: "stable",
        ran_at: "2026-05-24T09:00:00.000Z",
      },
    },
    {
      id: "id-1",
      version: "1.0.0",
      trust_grade: "tenant",
      superseded: true,
      lifecycle_state: "active",
      created_at: "2026-05-10T09:00:00.000Z",
      eval: null,
    },
  ],
};

describe("SkillVersionHistoryCard", () => {
  it("renders the version timeline newest-first with the eval delta", () => {
    render(
      <SkillVersionHistoryCard skillName="pricing.competitor_scrape" versions={MULTI} />,
    );
    const card = screen.getByTestId("skill-version-history-card");
    expect(card).toHaveAttribute("data-state", "populated");
    expect(screen.getByTestId("skill-version-history-subheader")).toHaveTextContent(
      /2 versions/,
    );

    // Newest version first, with its real pass-rate delta.
    const v2 = screen.getByTestId("skill-version-row-0");
    expect(within(v2).getByText(/v2\.0\.0/)).toBeInTheDocument();
    expect(screen.getByTestId("skill-version-eval-0")).toHaveTextContent(
      /\+12\.0pp/,
    );
    expect(within(v2).getByText(/active/)).toBeInTheDocument();

    // Older version: superseded badge + honest "no eval runs yet".
    const v1 = screen.getByTestId("skill-version-row-1");
    expect(within(v1).getByText(/superseded/)).toBeInTheDocument();
    expect(screen.getByTestId("skill-version-eval-1")).toHaveTextContent(
      /No eval runs yet/i,
    );
  });

  it("renders an honest empty state when versions is null", () => {
    render(<SkillVersionHistoryCard skillName="lead.score" versions={null} />);
    const card = screen.getByTestId("skill-version-history-card");
    expect(card).toHaveAttribute("data-state", "empty");
    expect(screen.getByTestId("skill-version-history-empty")).toHaveTextContent(
      /No version history yet/i,
    );
    // No fabricated rows.
    expect(screen.queryByTestId("skill-version-history-timeline")).not.toBeInTheDocument();
  });

  it("reports the single-version case honestly", () => {
    const single: SkillVersionsResponse = {
      skill_name: "lead.score",
      version_count: 1,
      versions: [
        {
          id: "id-1",
          version: "1.0.0",
          trust_grade: "learned_unreviewed",
          superseded: false,
          lifecycle_state: "active",
          created_at: "2026-05-10T09:00:00.000Z",
          eval: null,
        },
      ],
    };
    render(<SkillVersionHistoryCard skillName="lead.score" versions={single} />);
    expect(screen.getByTestId("skill-version-history-subheader")).toHaveTextContent(
      /One version installed/i,
    );
    expect(screen.getByTestId("skill-version-row-0")).toBeInTheDocument();
  });
});
