import { act, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { useEffect } from "react";

import { useWorkflowBuilderState } from "@/lib/hooks/use-workflow-builder-state";
import { UnsavedBanner } from "./UnsavedBanner";

const INITIAL_NODES = [
  { id: "n1", position: { x: 0, y: 0 }, data: { label: "step-1" } },
];
const INITIAL_EDGES: Parameters<typeof useWorkflowBuilderState>[1] = [];

describe("UnsavedBanner (presentational)", () => {
  it("renders nothing when dirty is false", () => {
    const { container } = render(
      <UnsavedBanner dirty={false} onSave={() => {}} onDiscard={() => {}} />,
    );
    expect(container.firstChild).toBeNull();
  });

  it("renders the banner copy + both action buttons when dirty", () => {
    render(<UnsavedBanner dirty onSave={() => {}} onDiscard={() => {}} />);
    const banner = screen.getByTestId("unsaved-banner");
    expect(banner).toHaveTextContent(/You have unsaved changes/);
    expect(screen.getByTestId("unsaved-banner-save")).toHaveTextContent(/Save/);
    expect(screen.getByTestId("unsaved-banner-discard")).toHaveTextContent(/Discard/);
  });

  it("invokes onSave / onDiscard from their respective buttons", async () => {
    const user = userEvent.setup();
    const onSave = vi.fn();
    const onDiscard = vi.fn();
    render(<UnsavedBanner dirty onSave={onSave} onDiscard={onDiscard} />);
    await user.click(screen.getByTestId("unsaved-banner-save"));
    await user.click(screen.getByTestId("unsaved-banner-discard"));
    expect(onSave).toHaveBeenCalledTimes(1);
    expect(onDiscard).toHaveBeenCalledTimes(1);
  });

  it("declares a prefers-reduced-motion guard in its styles", () => {
    const { container } = render(
      <UnsavedBanner dirty onSave={() => {}} onDiscard={() => {}} />,
    );
    const styleNodes = container.ownerDocument.querySelectorAll("style");
    const css = Array.from(styleNodes).map((s) => s.textContent ?? "").join("\n");
    expect(css).toMatch(/prefers-reduced-motion/);
  });
});

/**
 * Harness exposes the live hook to the test so dragging-equivalent
 * mutations (`addNode`) flow through the same `dirty` path the real
 * page uses, and `Discard` calls `reset()` with the initial seed.
 */
function Harness({
  capture,
}: {
  capture: (api: ReturnType<typeof useWorkflowBuilderState>) => void;
}) {
  const builder = useWorkflowBuilderState(INITIAL_NODES, INITIAL_EDGES);
  useEffect(() => {
    capture(builder);
  }, [builder, capture]);
  return (
    <>
      <span data-testid="node-count">{builder.nodes.length}</span>
      <UnsavedBanner
        dirty={builder.dirty}
        onSave={() => {}}
        onDiscard={() => builder.reset(INITIAL_NODES, INITIAL_EDGES)}
      />
    </>
  );
}

describe("UnsavedBanner — hook integration", () => {
  it("mutating the graph flips dirty → banner appears; Discard returns to initial", async () => {
    const user = userEvent.setup();
    let api: ReturnType<typeof useWorkflowBuilderState> | null = null;
    render(<Harness capture={(a) => { api = a; }} />);

    expect(screen.queryByTestId("unsaved-banner")).toBeNull();
    expect(screen.getByTestId("node-count").textContent).toBe("1");

    // Simulate a node drop into the canvas — same surface the real
    // canvas uses when the user adds or drags-in a step.
    act(() => {
      api!.addNode({
        id: "n2",
        position: { x: 100, y: 0 },
        data: { label: "step-2" },
      });
    });

    expect(screen.getByTestId("unsaved-banner")).toBeInTheDocument();
    expect(screen.getByTestId("node-count").textContent).toBe("2");

    await user.click(screen.getByTestId("unsaved-banner-discard"));

    expect(screen.queryByTestId("unsaved-banner")).toBeNull();
    expect(screen.getByTestId("node-count").textContent).toBe("1");
  });
});
