import { describe, expect, it } from "vitest";

import { decodeStepsFromToml, encodeStepsToToml } from "../step-encoder";
import type { StepKind, WorkflowStep } from "../types";

const ALL_STEP_KINDS: StepKind[] = [
  "trigger",
  "skill_invoke",
  "approval",
  "tool_call",
  "notify",
  "decision",
  "data",
  "loop",
  "wait",
  "sub_workflow",
];

function stepFor(kind: StepKind, idx: number): WorkflowStep {
  return {
    id: `step-${idx + 1}`,
    kind,
    name: `Step ${idx + 1} ${kind}`,
    description: `${kind} description with detail`,
  };
}

describe("step-encoder round-trip", () => {
  it("survives every StepKind", () => {
    const steps = ALL_STEP_KINDS.map((kind, idx) => stepFor(kind, idx));
    const toml = encodeStepsToToml(steps, {
      workflowName: "round_trip_demo",
      workflowDescription: "Encoder/decoder identity test",
    });
    const decoded = decodeStepsFromToml(toml);
    expect(decoded.workflowName).toBe("round_trip_demo");
    expect(decoded.workflowDescription).toBe("Encoder/decoder identity test");
    expect(decoded.steps).toEqual(steps);
  });

  it("emits a [workflow] header even without description", () => {
    const toml = encodeStepsToToml([], { workflowName: "empty" });
    expect(toml).toContain("[workflow]");
    expect(toml).toContain("name = \"empty\"");
    expect(toml).not.toContain("description");
  });

  it("preserves embedded quotes, backslashes, and newlines through round-trip", () => {
    const tricky: WorkflowStep[] = [
      {
        id: "step-1",
        kind: "tool_call",
        name: "Quotes and backslashes",
        description: "She said \"hi\" — path is C:\\Users\\demo\nLine two",
      },
    ];
    const toml = encodeStepsToToml(tricky, { workflowName: "edge" });
    const decoded = decodeStepsFromToml(toml);
    expect(decoded.steps).toEqual(tricky);
  });

  it("falls back to tool_call when the kind tag is unknown", () => {
    const fakeToml = [
      "[workflow]",
      "name = \"unknown_kind\"",
      "",
      "[[step]]",
      "id = \"step-1\"",
      "name = \"Mystery\"",
      "kind = \"made_up_kind\"",
      "description = \"\"",
      "",
    ].join("\n");
    const decoded = decodeStepsFromToml(fakeToml);
    expect(decoded.steps).toEqual([
      { id: "step-1", kind: "tool_call", name: "Mystery", description: "" },
    ]);
  });

  it("ignores comments and unknown sections", () => {
    const toml = [
      "# top-level comment",
      "[workflow]",
      "name = \"with_extras\"  # inline comment",
      "",
      "[unknown.section]",
      "foo = \"bar\"",
      "",
      "[[step]]",
      "id = \"step-1\"",
      "name = \"Real step\"",
      "kind = \"notify\"",
    ].join("\n");
    const decoded = decodeStepsFromToml(toml);
    expect(decoded.workflowName).toBe("with_extras");
    expect(decoded.steps).toEqual([
      { id: "step-1", kind: "notify", name: "Real step", description: "" },
    ]);
  });
});
