import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { BundleCard } from "./BundleCard";

function renderBundle(overrides: Partial<React.ComponentProps<typeof BundleCard>> = {}) {
  const onDecide = vi.fn();
  render(
    <BundleCard
      bundleId="b-123"
      label="8 social-media replies"
      count={8}
      members={["a1", "a2", "a3"]}
      onDecide={onDecide}
      {...overrides}
    />,
  );
  return { onDecide };
}

describe("BundleCard", () => {
  it("renders the stack metaphor with two trailing offset layers", () => {
    renderBundle();
    expect(screen.getByTestId("bundle-stack-layer-0")).toBeInTheDocument();
    expect(screen.getByTestId("bundle-stack-layer-1")).toBeInTheDocument();
  });

  it("renders label, count badge, and both decision buttons", () => {
    renderBundle();
    expect(screen.getByText(/8 social-media replies/)).toBeInTheDocument();
    expect(screen.getByTestId("bundle-count-pill")).toHaveTextContent("8 actions");
    expect(screen.getByTestId("bundle-approve")).toBeInTheDocument();
    expect(screen.getByTestId("bundle-refuse")).toBeInTheDocument();
  });

  it("fires onDecide('approve') when Approve bundle is tapped", async () => {
    const user = userEvent.setup();
    const { onDecide } = renderBundle();
    await user.click(screen.getByTestId("bundle-approve"));
    expect(onDecide).toHaveBeenCalledWith("approve");
  });

  it("fires onDecide('refuse') when Refuse bundle is tapped", async () => {
    const user = userEvent.setup();
    const { onDecide } = renderBundle();
    await user.click(screen.getByTestId("bundle-refuse"));
    expect(onDecide).toHaveBeenCalledWith("refuse");
  });

  it("renders post-decision text when controlled decision is set", () => {
    renderBundle({ decision: "approve" });
    expect(screen.getByText(/Approved · 8 actions/)).toBeInTheDocument();
    expect(screen.queryByTestId("bundle-approve")).not.toBeInTheDocument();
  });

  it("reveals member ids when 'What's inside?' tapped", async () => {
    const user = userEvent.setup();
    renderBundle();
    expect(screen.queryByText("a1")).not.toBeInTheDocument();
    await user.click(screen.getByTestId("bundle-toggle-members"));
    const ids = screen.getAllByTestId("bundle-member-id");
    expect(ids).toHaveLength(3);
    expect(ids[0]).toHaveTextContent("a1");
  });

  it("hides the 'What's inside?' affordance when members is empty", () => {
    renderBundle({ members: [] });
    expect(screen.queryByTestId("bundle-toggle-members")).not.toBeInTheDocument();
  });
});
