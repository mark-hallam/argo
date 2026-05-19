import { render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { WorkflowDiff, type WorkflowDiffProps } from "./WorkflowDiff";

const FROM_META = { id: "v1", label: "v1" } as const;
const TO_META = { id: "v2", label: "v2" } as const;

function renderDiff(overrides: Partial<WorkflowDiffProps> = {}) {
  return render(
    <WorkflowDiff
      fromToml=""
      toToml=""
      fromMeta={FROM_META}
      toMeta={TO_META}
      {...overrides}
    />,
  );
}

describe("WorkflowDiff", () => {
  it("renders the no-changes empty state when from === to", () => {
    const toml = "[workflow]\nname = \"intake\"\n";
    renderDiff({ fromToml: toml, toToml: toml });
    expect(screen.getByTestId("workflow-diff-empty")).toBeInTheDocument();
    expect(screen.getByTestId("workflow-diff-empty")).toHaveAttribute("role", "status");
    expect(screen.getByTestId("workflow-diff-stats")).toHaveTextContent(/no changes/i);
    expect(screen.queryByTestId("workflow-diff-hunk")).not.toBeInTheDocument();
  });

  it("renders added lines with green emerald tone in the right pane", () => {
    const from = "[workflow]\nname = \"intake\"\n";
    const to = "[workflow]\nname = \"intake\"\nowner = \"peta\"\n";
    renderDiff({ fromToml: from, toToml: to });

    const rightPane = screen.getByTestId("workflow-diff-side-to");
    const addedRow = within(rightPane).getByText("owner = \"peta\"");
    const li = addedRow.closest("li");
    expect(li).not.toBeNull();
    expect(li!.className).toMatch(/emerald-50/);
    expect(screen.getByTestId("workflow-diff-stats")).toHaveTextContent(/\+1/);
  });

  it("renders removed lines with red rose tone in the left pane", () => {
    const from = "[workflow]\nname = \"intake\"\nowner = \"peta\"\n";
    const to = "[workflow]\nname = \"intake\"\n";
    renderDiff({ fromToml: from, toToml: to });

    const leftPane = screen.getByTestId("workflow-diff-side-from");
    const removedRow = within(leftPane).getByText("owner = \"peta\"");
    const li = removedRow.closest("li");
    expect(li).not.toBeNull();
    expect(li!.className).toMatch(/rose-50/);
    expect(screen.getByTestId("workflow-diff-stats")).toHaveTextContent(/−1/);
  });

  it("renders mixed hunks with correct line numbers", () => {
    const from = ["a", "b", "c", "d", "e"].join("\n") + "\n";
    const to = ["a", "B", "c", "d", "E"].join("\n") + "\n";
    renderDiff({ fromToml: from, toToml: to });

    const hunks = screen.getAllByTestId("workflow-diff-hunk");
    expect(hunks.length).toBeGreaterThanOrEqual(1);

    const leftPane = screen.getByTestId("workflow-diff-side-from");
    expect(within(leftPane).getByText("b")).toBeInTheDocument();
    const rightPane = screen.getByTestId("workflow-diff-side-to");
    expect(within(rightPane).getByText("B")).toBeInTheDocument();

    const hunkHeader = hunks[0].querySelector(".font-mono");
    expect(hunkHeader?.textContent).toMatch(/^@@ -\d+,\d+ \+\d+,\d+ @@$/);

    expect(screen.getByTestId("workflow-diff-stats")).toHaveTextContent(/\+2/);
    expect(screen.getByTestId("workflow-diff-stats")).toHaveTextContent(/−2/);
  });

  it("each hunk is an aria-labelled region (a11y)", () => {
    const from = "a\nb\n";
    const to = "a\nB\n";
    renderDiff({ fromToml: from, toToml: to });
    const hunk = screen.getByTestId("workflow-diff-hunk");
    expect(hunk).toHaveAttribute("role", "region");
    expect(hunk.getAttribute("aria-label")).toMatch(/v1.*v2/);
  });
});
