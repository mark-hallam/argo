import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import {
  addNode,
  connectNodes,
  deleteNode,
  EditMode,
  validateSkillGraphForSave,
} from "./EditMode";
import type { SkillGraph } from "@/lib/types/skill-ir";

function graph(): SkillGraph {
  return {
    entry: "a",
    nodes: [
      {
        id: "a",
        label: "Start",
        kind: { type: "action", instruction: "start" },
        annotations: {},
      },
      {
        id: "b",
        label: "Finish",
        kind: { type: "action", instruction: "finish" },
        annotations: {},
      },
    ],
    edges: [{ id: "ab", from: "a", to: "b", cond: { kind: "always" } }],
    variables: [],
    metadata: {},
  };
}

describe("validateSkillGraphForSave", () => {
  it("rejects a missing entry node", () => {
    const g = graph();
    g.entry = "missing";
    expect(validateSkillGraphForSave(g)).toMatch(/entry/);
  });

  it("rejects graph without a terminal node", () => {
    const g = graph();
    g.edges.push({ id: "ba", from: "b", to: "a", cond: { kind: "always" } });
    expect(validateSkillGraphForSave(g)).toMatch(/terminal/);
  });

  it("rejects orphaned nodes", () => {
    const g = graph();
    g.nodes.push({
      id: "orphan",
      label: "Orphan",
      kind: { type: "action", instruction: "x" },
      annotations: {},
    });
    expect(validateSkillGraphForSave(g)).toMatch(/orphaned/);
  });

  it("treats loop and try bodies as reachable graph links", () => {
    const g = graph();
    g.nodes[1] = {
      id: "b",
      label: "Loop",
      kind: { type: "loop", iterVar: "item", body: ["c"] },
      annotations: {},
    };
    g.nodes.push({
      id: "c",
      label: "Try",
      kind: { type: "try-catch", tryBody: ["d"], catchBody: ["e"] },
      annotations: {},
    });
    g.nodes.push({
      id: "d",
      label: "Do",
      kind: { type: "action", instruction: "do it" },
      annotations: {},
    });
    g.nodes.push({
      id: "e",
      label: "Recover",
      kind: { type: "action", instruction: "recover" },
      annotations: {},
    });

    expect(validateSkillGraphForSave(g)).toBeNull();
  });
});

describe("EditMode transforms", () => {
  it("adds a node from the palette", () => {
    const next = addNode(graph(), "fetch");
    expect(next.nodes.some((node) => node.kind.type === "fetch")).toBe(true);
  });

  it("deleteNode removes incident edges", () => {
    const next = deleteNode(graph(), "b");
    expect(next.nodes.map((node) => node.id)).toEqual(["a"]);
    expect(next.edges).toHaveLength(0);
  });

  it("connectNodes adds one edge and deduplicates repeats", () => {
    const g = addNode(graph(), "transform");
    const once = connectNodes(g, "b", "n_transform_3");
    const twice = connectNodes(once, "b", "n_transform_3");
    expect(once.edges).toHaveLength(2);
    expect(twice.edges).toHaveLength(2);
  });
});

describe("EditMode UI", () => {
  it("inserts, deletes, reroutes, and toggles policy", async () => {
    const user = userEvent.setup();
    render(<EditMode skillId="skill-1" graph={graph()} initialVersion="v1" />);

    await user.click(screen.getByTestId("palette-transform"));
    expect(screen.getByTestId("edit-node-n_transform_3")).toBeInTheDocument();

    await user.selectOptions(screen.getByTestId("edge-source"), "b");
    await user.selectOptions(screen.getByTestId("edge-target"), "n_transform_3");
    await user.click(screen.getByTestId("connect-nodes"));
    expect(screen.getByTestId("edge-list")).toHaveTextContent("b -> n_transform_3");

    await user.selectOptions(screen.getByTestId("policy-a"), "ask");
    expect(screen.getByTestId("policy-a")).toHaveValue("ask");

    await user.click(screen.getByTestId("delete-node-n_transform_3"));
    expect(screen.queryByTestId("edit-node-n_transform_3")).not.toBeInTheDocument();
  });

  it("blocks save when topology is invalid", async () => {
    const user = userEvent.setup();
    const saveGraph = vi.fn();
    render(
      <EditMode
        skillId="skill-1"
        graph={graph()}
        initialVersion="v1"
        saveGraph={saveGraph}
      />,
    );

    await user.click(screen.getByTestId("palette-action"));
    await user.click(screen.getByTestId("save-ir"));

    expect(saveGraph).not.toHaveBeenCalled();
    expect(screen.getByTestId("save-error")).toHaveTextContent(/orphaned/);
  });

  it("successful save appends history and surfaces AB scheduling", async () => {
    const user = userEvent.setup();
    const saveGraph = vi.fn().mockResolvedValue({
      skillId: "skill-1",
      version: "canvas-2",
      graph: graph(),
      abTestScheduled: true,
      abTestRunId: "run-1",
    });
    render(
      <EditMode
        skillId="skill-1"
        graph={graph()}
        initialVersion="v1"
        saveGraph={saveGraph}
      />,
    );

    await user.selectOptions(screen.getByTestId("policy-a"), "ask");
    await user.click(screen.getByTestId("save-ir"));

    expect(saveGraph).toHaveBeenCalledOnce();
    expect(await screen.findByTestId("ab-scheduled")).toBeInTheDocument();
    expect(within(screen.getByTestId("edit-history")).getByText(/canvas-2/)).toBeInTheDocument();
  });
});
