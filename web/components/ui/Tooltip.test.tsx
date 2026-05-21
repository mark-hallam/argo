import { render, screen, act } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import {
  HoverHint,
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "./Tooltip";

function renderWithProvider(ui: React.ReactNode, providerProps: { delayDuration?: number } = {}) {
  return render(
    <TooltipProvider delayDuration={providerProps.delayDuration ?? 0}>{ui}</TooltipProvider>,
  );
}

describe("Tooltip primitive", () => {
  it("opens on focus and shows the label", async () => {
    const user = userEvent.setup();
    renderWithProvider(
      <HoverHint label="Copy hash to clipboard">
        <button type="button" data-testid="trigger">copy</button>
      </HoverHint>,
    );
    await user.tab();
    expect(screen.getByTestId("trigger")).toHaveFocus();
    const tooltips = await screen.findAllByText("Copy hash to clipboard");
    expect(tooltips.length).toBeGreaterThan(0);
  });

  it("closes on Escape key after opening on focus", async () => {
    const user = userEvent.setup();
    renderWithProvider(
      <HoverHint label="Press escape">
        <button type="button" data-testid="trigger">x</button>
      </HoverHint>,
    );
    await user.tab();
    await screen.findAllByText("Press escape");
    await user.keyboard("{Escape}");
    await act(async () => {
      await new Promise((r) => setTimeout(r, 0));
    });
    expect(screen.queryByText("Press escape")).toBeNull();
  });

  it("renders TooltipContent default variant with popover tokens", async () => {
    const user = userEvent.setup();
    renderWithProvider(
      <Tooltip>
        <TooltipTrigger asChild>
          <button type="button" data-testid="trigger">btn</button>
        </TooltipTrigger>
        <TooltipContent data-testid="content">default body</TooltipContent>
      </Tooltip>,
    );
    await user.tab();
    const content = await screen.findByTestId("content");
    expect(content.className).toMatch(/bg-\[var\(--popover\)\]/);
    expect(content.className).toMatch(/text-\[var\(--popover-foreground\)\]/);
  });

  it("renders TooltipContent inverted variant with foreground tokens", async () => {
    const user = userEvent.setup();
    renderWithProvider(
      <Tooltip>
        <TooltipTrigger asChild>
          <button type="button" data-testid="trigger">btn</button>
        </TooltipTrigger>
        <TooltipContent data-testid="content" variant="inverted">inv</TooltipContent>
      </Tooltip>,
    );
    await user.tab();
    const content = await screen.findByTestId("content");
    expect(content.className).toMatch(/bg-\[var\(--foreground\)\]/);
    expect(content.className).toMatch(/text-\[var\(--background\)\]/);
  });

  it("HoverHint sugar delegates focus via asChild to the child trigger", async () => {
    const user = userEvent.setup();
    renderWithProvider(
      <HoverHint label="L">
        <button type="button" data-testid="trigger">child</button>
      </HoverHint>,
    );
    expect(screen.getByTestId("trigger").tagName).toBe("BUTTON");
    await user.tab();
    expect(screen.getByTestId("trigger")).toHaveFocus();
  });

  it("HoverHint forwards rich React node labels (not just strings)", async () => {
    const user = userEvent.setup();
    renderWithProvider(
      <HoverHint label={<span data-testid="rich-label">Bold <strong>hint</strong></span>}>
        <button type="button" data-testid="trigger">t</button>
      </HoverHint>,
    );
    await user.tab();
    const labels = await screen.findAllByTestId("rich-label");
    expect(labels.length).toBeGreaterThan(0);
  });

  it("TooltipProvider passes delayDuration through to children without crashing", () => {
    const { container } = render(
      <TooltipProvider delayDuration={250} skipDelayDuration={100}>
        <Tooltip>
          <TooltipTrigger asChild>
            <button type="button">t1</button>
          </TooltipTrigger>
          <TooltipContent>c1</TooltipContent>
        </Tooltip>
        <Tooltip>
          <TooltipTrigger asChild>
            <button type="button">t2</button>
          </TooltipTrigger>
          <TooltipContent>c2</TooltipContent>
        </Tooltip>
      </TooltipProvider>,
    );
    expect(container.querySelectorAll("button").length).toBe(2);
  });

  it("HoverHint accepts a per-instance delayMs override", () => {
    const { container } = renderWithProvider(
      <HoverHint label="L" delayMs={500} side="right">
        <button type="button" data-testid="trigger">t</button>
      </HoverHint>,
    );
    expect(container.querySelector('[data-testid="trigger"]')).toBeTruthy();
  });
});
