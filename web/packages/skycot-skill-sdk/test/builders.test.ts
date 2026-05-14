import { describe, expect, it } from "vitest";
import { createSkill, sampleSkills, validateSkillGraph } from "../src";
import type { SkillGraph } from "../src";

describe("SkillBuilder", () => {
  it("builds an action graph using the first node as entry", () => {
    const graph = createSkill({ name: "hello", version: "1" })
      .action("draft", "Draft reply", "Write a reply")
      .build();

    expect(graph.entry).toBe("draft");
    expect(graph.nodes[0].kind.type).toBe("action");
    expect(graph.metadata.name).toBe("hello");
  });

  it("supports explicit entry selection", () => {
    const graph = createSkill()
      .action("a", "A", "first")
      .action("b", "B", "second")
      .start("b")
      .build();

    expect(graph.entry).toBe("b");
  });

  it("builds typed edges and variables", () => {
    const graph = createSkill()
      .fetch("read", "Read", "gmail")
      .transform("clean", "Clean", "{{body}}")
      .edge("read-clean", "read", "clean", { kind: "on-success" })
      .variable("body", "string")
      .build();

    expect(graph.edges[0].cond.kind).toBe("on-success");
    expect(graph.variables[0].scope).toBe("graph");
  });

  it("preserves tool bindings and confidence annotations", () => {
    const graph = createSkill()
      .action("post", "Post", "Publish", {
        annotations: { confidence: 0.8 },
        toolBinding: { mcpServer: "slack", toolName: "chat.postMessage" },
      })
      .build();

    expect(graph.nodes[0].annotations.toolBinding?.toolName).toBe("chat.postMessage");
    expect(graph.nodes[0].annotations.confidence).toBe(0.8);
  });

  it("rejects duplicate node ids", () => {
    expect(() =>
      createSkill().action("same", "A", "one").action("same", "B", "two"),
    ).toThrow(/duplicate node/);
  });

  it("rejects duplicate variable names", () => {
    expect(() => createSkill().action("a", "A", "go").variable("x", "string").variable("x", "number")).toThrow(
      /duplicate variable/,
    );
  });

  it("rejects edges to missing nodes", () => {
    expect(() => createSkill().action("a", "A", "go").edge("bad", "a", "missing").build()).toThrow(
      /missing node/,
    );
  });

  it("rejects invalid confidence scores", () => {
    expect(() =>
      createSkill()
        .action("a", "A", "go", { annotations: { confidence: 1.1 } })
        .build(),
    ).toThrow(/confidence/);
  });

  it("rejects empty branch arms", () => {
    expect(() => createSkill().branch("route", "Route", []).build()).toThrow(/branch node/);
  });

  it("builds five sample skills that validate against the L1 shape", () => {
    const samples = sampleSkills();

    expect(samples).toHaveLength(5);
    for (const graph of samples) {
      const valid: SkillGraph = validateSkillGraph(graph);
      expect(valid.nodes.length).toBeGreaterThan(0);
      expect(valid.metadata.name).toBeTruthy();
    }
  });
});
