import { describe, expect, it } from "vitest";
import { diffSkillGraphs, formatDiffSummary } from "./diff-engine";
import type { Edge, Node, SkillGraph } from "@/lib/types/skill-ir";

function node(id: string, label = id): Node {
  return {
    id,
    label,
    kind: { type: "action", instruction: `do ${id}` },
    annotations: { confidence: 0.8, sampleCount: 2 },
  };
}

function edge(id: string, from: string, to: string): Edge {
  return { id, from, to, cond: { kind: "always" } };
}

function graph(nodes: Node[], edges: Edge[] = []): SkillGraph {
  return { entry: nodes[0]?.id ?? "n0", nodes, edges, variables: [], metadata: {} };
}

describe("diffSkillGraphs", () => {
  it("returns an empty summary when graphs are identical", () => {
    const left = graph([node("a"), node("b")], [edge("ab", "a", "b")]);
    const diff = diffSkillGraphs(left, structuredClone(left));

    expect(diff.nodeDiffs).toHaveLength(0);
    expect(diff.edgeDiffs).toHaveLength(0);
    expect(diff.annotationDiffs).toHaveLength(0);
    expect(formatDiffSummary(diff.summary)).toBe("No structural changes");
  });

  it("classifies added and removed nodes by stable id", () => {
    const diff = diffSkillGraphs(graph([node("a"), node("b")]), graph([node("b"), node("c")]));

    expect(diff.summary.nodesAdded).toBe(1);
    expect(diff.summary.nodesRemoved).toBe(1);
    expect(diff.nodeDiffs.map((d) => [d.id, d.state])).toEqual([
      ["a", "removed"],
      ["c", "added"],
    ]);
  });

  it("marks label changes as node modifications", () => {
    const diff = diffSkillGraphs(graph([node("a", "Old")]), graph([node("a", "New")]));

    expect(diff.summary.nodesModified).toBe(1);
    expect(diff.nodeDiffs[0]).toMatchObject({
      id: "a",
      state: "modified",
      changedFields: ["label"],
    });
  });

  it("marks kind payload changes as node modifications", () => {
    const right = node("a");
    right.kind = { type: "fetch", target: "crm:accounts", method: "GET" };

    const diff = diffSkillGraphs(graph([node("a")]), graph([right]));

    expect(diff.summary.nodesModified).toBe(1);
    expect(diff.nodeDiffs[0]?.changedFields).toContain("kind");
  });

  it("tracks annotation shifts separately from structural node changes", () => {
    const right = node("a");
    right.annotations = { confidence: 0.42, sampleCount: 9, policy: "ask" };

    const diff = diffSkillGraphs(graph([node("a")]), graph([right]));

    expect(diff.summary.nodesModified).toBe(0);
    expect(diff.summary.annotationsChanged).toBe(1);
    expect(diff.annotationDiffs[0]?.nodeId).toBe("a");
  });

  it("classifies added and removed edges", () => {
    const diff = diffSkillGraphs(
      graph([node("a"), node("b"), node("c")], [edge("ab", "a", "b")]),
      graph([node("a"), node("b"), node("c")], [edge("bc", "b", "c")]),
    );

    expect(diff.summary.edgesAdded).toBe(1);
    expect(diff.summary.edgesRemoved).toBe(1);
    expect(diff.edgeDiffs.map((d) => [d.id, d.state])).toEqual([
      ["ab", "removed"],
      ["bc", "added"],
    ]);
  });

  it("classifies same-id endpoint changes as rewired edges", () => {
    const diff = diffSkillGraphs(
      graph([node("a"), node("b"), node("c")], [edge("e1", "a", "b")]),
      graph([node("a"), node("b"), node("c")], [edge("e1", "a", "c")]),
    );

    expect(diff.summary.edgesRewired).toBe(1);
    expect(diff.edgeDiffs[0]).toMatchObject({
      id: "e1",
      state: "rewired",
      changedFields: ["endpoints"],
    });
  });

  it("classifies condition changes as modified edges", () => {
    const right = edge("e1", "a", "b");
    right.cond = { kind: "on-pattern", pattern: "approved" };

    const diff = diffSkillGraphs(
      graph([node("a"), node("b")], [edge("e1", "a", "b")]),
      graph([node("a"), node("b")], [right]),
    );

    expect(diff.summary.edgesModified).toBe(1);
    expect(diff.edgeDiffs[0]?.changedFields).toContain("condition");
  });

  it("diffs a 100-node graph well below the 200ms budget", () => {
    const leftNodes = Array.from({ length: 100 }, (_, idx) => node(`n${idx}`));
    const rightNodes = leftNodes.map((n, idx) =>
      idx % 10 === 0 ? { ...n, label: `${n.label} changed` } : n,
    );
    const edges = Array.from({ length: 99 }, (_, idx) => edge(`e${idx}`, `n${idx}`, `n${idx + 1}`));
    const started = performance.now();
    const diff = diffSkillGraphs(graph(leftNodes, edges), graph(rightNodes, edges));
    const elapsed = performance.now() - started;

    expect(diff.summary.nodesModified).toBe(10);
    expect(elapsed).toBeLessThan(200);
  });
});
