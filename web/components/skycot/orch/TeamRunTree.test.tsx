import { render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { TeamRunTree } from "./TeamRunTree";
import { SAMPLE_TEAM_RUNS, groupTeamRuns } from "@/lib/api/orchestration";

describe("TeamRunTree", () => {
  it("renders nothing when there are no team runs", () => {
    const { container } = render(<TeamRunTree teams={[]} />);
    expect(container).toBeEmptyDOMElement();
  });

  it("renders the sample team runs as lead-with-children trees", () => {
    const { teams } = groupTeamRuns(SAMPLE_TEAM_RUNS);
    render(<TeamRunTree teams={teams} />);
    expect(screen.getByTestId("team-run-tree")).toBeInTheDocument();
    // Two leads in the sample → at least two cards.
    expect(screen.getAllByTestId("team-run-card").length).toBeGreaterThanOrEqual(2);
    // Lead workers are surfaced.
    expect(screen.getByText("Mariah")).toBeInTheDocument();
    expect(screen.getByText("Patrick")).toBeInTheDocument();
  });

  it("shows each child delegation with a clean task label and its worker", () => {
    const lead = {
      ...SAMPLE_TEAM_RUNS[0],
      id: "lead",
      skill: "[team:run-1] lead synthesis — Analysis",
    };
    const child = {
      ...SAMPLE_TEAM_RUNS[1],
      id: "child",
      worker: "Nora",
      status: "done" as const,
      skill: "[team:run-1] survey → Survey the competitor landscape",
    };
    const { teams } = groupTeamRuns([lead, child]);
    render(<TeamRunTree teams={teams} />);
    const card = screen.getByTestId("team-run-card");
    expect(within(card).getByText("Survey the competitor landscape")).toBeInTheDocument();
    expect(within(card).getByText("Nora")).toBeInTheDocument();
    // Child + lead both report "Done"/their status via a chip.
    expect(within(card).getAllByText("Done").length).toBeGreaterThan(0);
    expect(within(card).getByText("1/1 done")).toBeInTheDocument();
  });

  it("surfaces an empty-children note for an unattributed lead node", () => {
    render(
      <TeamRunTree
        teams={[{ key: "k", lead: SAMPLE_TEAM_RUNS[0], title: "Lonely lead", children: [] }]}
      />,
    );
    expect(
      screen.getByText(/no child delegations attributed to this lead yet/i),
    ).toBeInTheDocument();
  });
});
