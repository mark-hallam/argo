import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { ApprovalCard } from "./ApprovalCard";
import { REDACTED_TOOLTIP_COPY } from "./RedactedText";
import { POLICY_MASK_ALL, POLICY_SHOW_ALL } from "@/lib/redaction/redact";

function renderCard(overrides: Partial<React.ComponentProps<typeof ApprovalCard>> = {}) {
  const onDecide = vi.fn();
  const onUndo = vi.fn();
  render(
    <ApprovalCard
      worker={{ name: "Mariah" }}
      verb="wants to send this listing"
      detail="Bauer Vapor X5 · $90"
      consequenceIfYes="goes to Facebook Marketplace."
      consequenceIfNo="waits for direction."
      reason="you said 'always check counter-offers'."
      onDecide={onDecide}
      onUndo={onUndo}
      {...overrides}
    />,
  );
  return { onDecide, onUndo };
}

describe("ApprovalCard", () => {
  it("renders the load-bearing structure in order", () => {
    renderCard();
    expect(screen.getByText(/Mariah/)).toBeInTheDocument();
    expect(screen.getByText(/wants to send this listing/)).toBeInTheDocument();
    expect(screen.getByText(/Bauer Vapor X5/)).toBeInTheDocument();
    expect(screen.getByText(/goes to Facebook Marketplace/)).toBeInTheDocument();
    expect(screen.getByText(/waits for direction/)).toBeInTheDocument();
    expect(screen.getByTestId("approval-approve")).toBeInTheDocument();
    expect(screen.getByTestId("approval-refuse")).toBeInTheDocument();
    expect(screen.getByTestId("approval-why")).toBeInTheDocument();
  });

  it("calls onDecide on approve", async () => {
    const user = userEvent.setup();
    const { onDecide } = renderCard();
    await user.click(screen.getByTestId("approval-approve"));
    expect(onDecide).toHaveBeenCalledWith("approve");
  });

  it("calls onDecide on refuse", async () => {
    const user = userEvent.setup();
    const { onDecide } = renderCard();
    await user.click(screen.getByTestId("approval-refuse"));
    expect(onDecide).toHaveBeenCalledWith("refuse");
  });

  it("reveals the reason when Why? tapped", async () => {
    const user = userEvent.setup();
    renderCard();
    expect(screen.queryByText(/always check counter-offers/)).not.toBeInTheDocument();
    await user.click(screen.getByTestId("approval-why"));
    expect(screen.getByText(/always check counter-offers/)).toBeInTheDocument();
  });

  it("renders the post-decision bar when controlled decision is set", () => {
    renderCard({ decision: "approve" });
    expect(screen.getByText(/Approved/)).toBeInTheDocument();
    expect(screen.getByTestId("approval-undo")).toBeInTheDocument();
  });

  it("hides the Why button when no reason is provided", () => {
    renderCard({ reason: undefined });
    expect(screen.queryByTestId("approval-why")).not.toBeInTheDocument();
  });

  it("renders raw card text when redactionPolicy = show-all", () => {
    renderCard({
      detail: "contact me at sam@acmeco.com",
      consequenceIfYes: "we'll email sam@acmeco.com.",
      redactionPolicy: POLICY_SHOW_ALL,
    });
    expect(screen.getByText(/contact me at sam@acmeco\.com/)).toBeInTheDocument();
    expect(screen.queryByText("[EMAIL]")).not.toBeInTheDocument();
  });

  it("renders sentinels when redactionPolicy = mask-all", () => {
    renderCard({
      detail: "contact me at sam@acmeco.com about +64 21 555 0123",
      consequenceIfYes: "we'll email sam@acmeco.com tomorrow.",
      consequenceIfNo: "we'll archive the request.",
      redactionPolicy: POLICY_MASK_ALL,
    });
    expect(screen.queryByText(/sam@acmeco\.com/)).not.toBeInTheDocument();
    expect(screen.queryByText(/\+64 21 555 0123/)).not.toBeInTheDocument();
    // Sentinels render as masked spans; there can be multiple instances.
    expect(screen.getAllByText("[EMAIL]").length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByText("[PHONE]").length).toBeGreaterThanOrEqual(1);
  });

  it("attaches the role-hint tooltip to masked sentinels", () => {
    renderCard({
      detail: "contact me at sam@acmeco.com",
      redactionPolicy: POLICY_MASK_ALL,
    });
    const masked = screen.getAllByText("[EMAIL]")[0];
    expect(masked).toHaveAttribute("title", REDACTED_TOOLTIP_COPY);
    expect(masked).toHaveAttribute("data-redacted", "email");
  });

  it("renders nothing for the draft region when draftContent is absent", () => {
    renderCard({ draftContent: undefined });
    expect(screen.queryByTestId("approval-draft")).not.toBeInTheDocument();
    expect(screen.queryByTestId("approval-draft-toggle")).not.toBeInTheDocument();
  });

  it("renders the draft collapsed by default with a Show draft label", () => {
    renderCard({
      draftContent: "# Q3 Roadmap\n\nThe full draft body the approver should read.",
    });
    const details = screen.getByTestId("approval-draft");
    expect(details).toBeInTheDocument();
    expect(details).not.toHaveAttribute("open");
    expect(screen.getByTestId("approval-draft-toggle")).toHaveTextContent(/Show draft/i);
  });

  it("flips the toggle label to Hide draft when expanded", async () => {
    const user = userEvent.setup();
    renderCard({
      draftContent: "Body of the draft that the approver should read before saying yes.",
    });
    const toggle = screen.getByTestId("approval-draft-toggle");
    expect(toggle).toHaveTextContent(/Show draft/i);
    await user.click(toggle);
    expect(toggle).toHaveTextContent(/Hide draft/i);
    expect(screen.getByTestId("approval-draft-body")).toBeInTheDocument();
  });

  it("redacts draftContent under POLICY_MASK_ALL the same way detail is redacted", () => {
    renderCard({
      draftContent: "Reply-to sam@acmeco.com about +64 21 555 0123",
      redactionPolicy: POLICY_MASK_ALL,
    });
    // Body content lives inside the <details>; jsdom renders the
    // <summary> + body regardless of `open` so the sentinels are
    // queryable in the DOM tree even before the user clicks.
    expect(screen.queryByText(/sam@acmeco\.com/)).not.toBeInTheDocument();
    expect(screen.queryByText(/\+64 21 555 0123/)).not.toBeInTheDocument();
    expect(screen.getAllByText("[EMAIL]").length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByText("[PHONE]").length).toBeGreaterThanOrEqual(1);
  });

  it("renders the severity pill when severity is provided", () => {
    renderCard({ severity: "p0" });
    expect(screen.getByTestId("severity-pill-p0")).toBeInTheDocument();
  });

  it("hides the severity pill when omitted (backward-compat)", () => {
    renderCard({});
    expect(screen.queryByTestId("severity-pill-p0")).not.toBeInTheDocument();
    expect(screen.queryByTestId("severity-pill-p1")).not.toBeInTheDocument();
    expect(screen.queryByTestId("severity-pill-p2")).not.toBeInTheDocument();
  });

  it("renders the metrics row when any ops-dashboard signal is set", () => {
    renderCard({ confidence: 0.85 });
    expect(screen.getByTestId("approval-metrics")).toBeInTheDocument();
    expect(screen.getByTestId("confidence-donut")).toBeInTheDocument();
  });

  it("renders $ at risk when dollarAtRisk is non-zero", () => {
    renderCard({ dollarAtRisk: 1500 });
    expect(screen.getByTestId("dollar-at-risk")).toHaveTextContent("$1,500");
  });

  it("omits the metrics row when no signals are present (legacy cards stay clean)", () => {
    renderCard({});
    expect(screen.queryByTestId("approval-metrics")).not.toBeInTheDocument();
  });
});
