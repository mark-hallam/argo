import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { Node, NodeKind } from "@/lib/types/skill-ir";
import { NodeInspector, validateNodeKind } from "./NodeInspector";

const cases: Array<{ kind: NodeKind; field: string; next: string }> = [
  { kind: { type: "action", instruction: "draft" }, field: "action-instruction", next: "send" },
  { kind: { type: "condition", expression: "ok" }, field: "condition-expression", next: "ready" },
  { kind: { type: "branch", arms: [{ label: "yes", pattern: "ok" }] }, field: "branch-arms", next: "no: fail" },
  { kind: { type: "await-approval", approverRole: "operator" }, field: "approval-role", next: "manager" },
  { kind: { type: "fetch", target: "mcp://crm", method: "GET" }, field: "fetch-target", next: "mcp://erp" },
  { kind: { type: "transform", template: "{{input}}" }, field: "transform-template", next: "{{name}}" },
  {
    kind: { type: "sub-skill-call", target: { skillId: "skill-a", version: "v1" } },
    field: "sub-skill-id",
    next: "skill-b",
  },
  { kind: { type: "loop", iterVar: "item", body: ["n2"] }, field: "loop-iter-var", next: "row" },
  { kind: { type: "try-catch", tryBody: ["n2"], catchBody: ["n3"] }, field: "try-body", next: "n2, n3" },
];

describe("NodeInspector", () => {
  it.each(cases)("renders and edits %s", async (item) => {
    const onChange = vi.fn();
    render(
      <NodeInspector
        node={node(item.kind)}
        graphNodeIds={["n1", "n2", "n3"]}
        onChange={onChange}
      />,
    );

    expect(screen.getByTestId(`${item.kind.type}-node-primitive`)).toBeInTheDocument();
    fireEvent.change(screen.getByTestId(item.field), { target: { value: item.next } });
    expect(onChange).toHaveBeenCalled();
  });

  it("edits shared labels without changing node kind", () => {
    const onChange = vi.fn();
    render(<NodeInspector node={node({ type: "action", instruction: "draft" })} onChange={onChange} />);

    fireEvent.change(screen.getByTestId("node-label"), { target: { value: "New label" } });

    expect(onChange).toHaveBeenLastCalledWith(
      expect.objectContaining({ label: "New label", kind: { type: "action", instruction: "draft" } }),
    );
  });

  it("surfaces inline schema validation errors", () => {
    render(<NodeInspector node={node({ type: "fetch", target: "", method: "get" })} onChange={vi.fn()} />);

    expect(screen.getByText("Required by the L1 Skill IR schema.")).toBeInTheDocument();
    expect(screen.getByText("Method must be uppercase.")).toBeInTheDocument();
    expect(screen.getByRole("alert")).toHaveTextContent(/schema errors/);
  });

  it("validates structural node id references", () => {
    expect(
      validateNodeKind({ type: "try-catch", tryBody: ["missing"], catchBody: [] }, ["known"]),
    ).toEqual({ tryBody: "Unknown node id: missing" });
  });
});

function node(kind: NodeKind): Node {
  return {
    id: "n1",
    label: "Start",
    kind,
    annotations: {},
  };
}
