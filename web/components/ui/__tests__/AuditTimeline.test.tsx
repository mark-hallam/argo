import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { AuditTimeline } from "../AuditTimeline";

describe("AuditTimeline", () => {
  it("renders a row per audit entry with kind + timestamp", () => {
    render(
      <AuditTimeline
        rows={[
          {
            id: "1",
            timestamp: "2026-05-16T10:00:00Z",
            kind: "approval.granted",
            payload: "by alice@",
            tone: "success",
          },
          {
            id: "2",
            timestamp: "2026-05-16T11:00:00Z",
            kind: "approval.denied",
            tone: "error",
          },
        ]}
      />,
    );
    expect(screen.getAllByTestId("audit-timeline-row")).toHaveLength(2);
    expect(screen.getByText("approval.granted")).toBeInTheDocument();
    expect(screen.getByText("approval.denied")).toBeInTheDocument();
  });

  it("renders an empty message when rows are empty", () => {
    render(<AuditTimeline rows={[]} emptyMessage="Nothing logged yet" />);
    expect(screen.getByText("Nothing logged yet")).toBeInTheDocument();
  });
});
