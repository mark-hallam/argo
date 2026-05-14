import { describe, expect, it } from "vitest";
import { sampleSkills } from "../../packages/skycot-skill-sdk/src";
import type { SkillGraph } from "@/lib/types/skill-ir";
import {
  roundTripSkillGraph,
  sdkSourceToSkillGraph,
  skillGraphToSdkSource,
  stableSkillGraphHash,
} from "./skill-code";

describe("skill-code bridge", () => {
  it("round-trips the five M12 sample skills without changing the IR hash", () => {
    for (const graph of sampleSkills()) {
      const result = roundTripSkillGraph(graph);
      expect(result.outputHash).toBe(result.inputHash);
      expect(result.graph).toEqual(graph);
    }
  });

  it("emits M12 SDK builder source", () => {
    const source = skillGraphToSdkSource(sampleSkills()[0]);
    expect(source).toContain('from "@skycot/skill-sdk"');
    expect(source).toContain(".fetch(");
    expect(source).toContain(".build();");
  });

  it("surfaces syntax errors for non-generated code", () => {
    expect(() => sdkSourceToSkillGraph("export const nope = {};")).toThrow(/createSkill/);
  });

  it("rejects unsupported builder calls", () => {
    const source = skillGraphToSdkSource(sampleSkills()[0]).replace(".fetch(", ".unknown(");
    expect(() => sdkSourceToSkillGraph(source)).toThrow(/Unsupported SDK builder call/);
  });

  it("preserves annotations and tool bindings", () => {
    const graph: SkillGraph = {
      entry: "post",
      nodes: [
        {
          id: "post",
          label: "Post update",
          kind: { type: "action", instruction: "Post the update" },
          annotations: {
            confidence: 0.9,
            policy: "ask",
            toolBinding: { mcpServer: "slack", toolName: "chat.postMessage" },
          },
        },
      ],
      edges: [],
      variables: [],
      metadata: { name: "annotated", version: "1.0.0" },
    };
    expect(roundTripSkillGraph(graph).graph).toEqual(graph);
  });

  it("hashes equal graphs consistently and different graphs differently", () => {
    const [a, b] = sampleSkills();
    expect(stableSkillGraphHash(a)).toBe(stableSkillGraphHash(roundTripSkillGraph(a).graph));
    expect(stableSkillGraphHash(a)).not.toBe(stableSkillGraphHash(b));
  });
});
