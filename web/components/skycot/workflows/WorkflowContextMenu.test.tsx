import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Copy, Trash2 } from "lucide-react";
import { describe, expect, it, vi } from "vitest";

import { WorkflowContextMenu, type ContextMenuItem } from "./WorkflowContextMenu";

function makeItems(overrides: Partial<ContextMenuItem>[] = []): ContextMenuItem[] {
  const defaults: ContextMenuItem[] = [
    { label: "Duplicate", Icon: Copy, onClick: vi.fn() },
    { label: "Delete", Icon: Trash2, onClick: vi.fn(), destructive: true },
  ];
  overrides.forEach((o, i) => {
    defaults[i] = { ...defaults[i], ...o };
  });
  return defaults;
}

describe("WorkflowContextMenu", () => {
  it("renders all items with role=menuitem and the menu container has role=menu", () => {
    const items = makeItems();
    render(<WorkflowContextMenu x={100} y={120} items={items} onClose={vi.fn()} />);
    expect(screen.getByRole("menu")).toBeInTheDocument();
    const menuItems = screen.getAllByRole("menuitem");
    expect(menuItems).toHaveLength(2);
    expect(menuItems[0]).toHaveTextContent("Duplicate");
    expect(menuItems[1]).toHaveTextContent("Delete");
  });

  it("invokes onClose when the user clicks outside the menu", async () => {
    const onClose = vi.fn();
    const user = userEvent.setup();
    render(
      <div>
        <button type="button" data-testid="outside">Outside</button>
        <WorkflowContextMenu x={50} y={50} items={makeItems()} onClose={onClose} />
      </div>,
    );
    await user.click(screen.getByTestId("outside"));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("invokes onClose when the user presses Escape", async () => {
    const onClose = vi.fn();
    const user = userEvent.setup();
    render(<WorkflowContextMenu x={0} y={0} items={makeItems()} onClose={onClose} />);
    await user.keyboard("{Escape}");
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("invokes the item's onClick handler and then onClose when an item is clicked", async () => {
    const onClose = vi.fn();
    const onDuplicate = vi.fn();
    const user = userEvent.setup();
    const items = makeItems([{ onClick: onDuplicate }]);
    render(<WorkflowContextMenu x={0} y={0} items={items} onClose={onClose} />);
    await user.click(screen.getByRole("menuitem", { name: /duplicate/i }));
    expect(onDuplicate).toHaveBeenCalledTimes(1);
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("marks destructive items with data-destructive='true' and skips disabled clicks", async () => {
    const onClose = vi.fn();
    const onDelete = vi.fn();
    const user = userEvent.setup();
    const items: ContextMenuItem[] = [
      { label: "Delete", Icon: Trash2, onClick: onDelete, destructive: true, disabled: true },
    ];
    render(<WorkflowContextMenu x={0} y={0} items={items} onClose={onClose} />);
    const button = screen.getByRole("menuitem", { name: /delete/i });
    expect(button).toHaveAttribute("data-destructive", "true");
    expect(button).toBeDisabled();
    await user.click(button);
    expect(onDelete).not.toHaveBeenCalled();
    expect(onClose).not.toHaveBeenCalled();
  });
});
