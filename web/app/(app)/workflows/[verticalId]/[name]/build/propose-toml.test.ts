import { describe, expect, it } from "vitest";

import { diffSteps, parseProposalToml } from "./propose-toml";
import type { WorkflowStep } from "./types";

const SAMPLE = `# vertical: ops
schema_version = "1"

[workflow]
name        = "weekly_check_in"
description = "Send Friday status to head of product."

[[step]]
name = "describe_intent"
kind = "prompt"
prompt = "Operator described the weekly cadence."

[[step]]
name = "operator_review"
kind = "approval"
description = "Pause for the operator before posting."
depends_on = ["describe_intent"]
`;

describe("parseProposalToml", () => {
  it("returns empty for an empty input", () => {
    expect(parseProposalToml("")).toEqual({
      workflowName: null,
      workflowDescription: null,
      steps: [],
    });
  });

  it("extracts workflow header + step list from a complete TOML doc", () => {
    const parsed = parseProposalToml(SAMPLE);
    expect(parsed.workflowName).toBe("weekly_check_in");
    expect(parsed.workflowDescription).toBe("Send Friday status to head of product.");
    expect(parsed.steps).toHaveLength(2);
    expect(parsed.steps[0]).toMatchObject({
      id: "describe_intent",
      kind: "skill_invoke",
      name: "describe_intent",
    });
    expect(parsed.steps[1]).toMatchObject({
      id: "operator_review",
      kind: "approval",
      description: "Pause for the operator before posting.",
    });
  });

  it("tolerates a partial mid-stream chunk (only complete steps emitted)", () => {
    const partial = SAMPLE.slice(0, SAMPLE.indexOf("operator_review") + "operator_review".length);
    const parsed = parseProposalToml(partial);
    // First step is complete and parsed. Trailing step has a partial
    // name-only line so it is emitted with that name, no extra keys.
    expect(parsed.steps.length).toBeGreaterThanOrEqual(1);
    expect(parsed.steps[0].name).toBe("describe_intent");
  });

  it("falls back to skill_invoke for unrecognized kinds", () => {
    const parsed = parseProposalToml(`[[step]]
name = "foo"
kind = "rocket"
`);
    expect(parsed.steps[0]).toMatchObject({ kind: "skill_invoke" });
  });
});

describe("diffSteps", () => {
  function s(id: string, kind: WorkflowStep["kind"] = "skill_invoke", name = id): WorkflowStep {
    return { id, kind, name, description: "" };
  }

  it("flags additions and removals by step id", () => {
    const before = [s("a"), s("b")];
    const after = [s("a"), s("c")];
    const diff = diffSteps(before, after);
    expect(diff.added.map((x) => x.id)).toEqual(["c"]);
    expect(diff.removed.map((x) => x.id)).toEqual(["b"]);
    expect(diff.changed).toEqual([]);
    expect(diff.unchangedCount).toBe(1);
  });

  it("flags kind changes as changed (not add+remove)", () => {
    const before = [s("a", "skill_invoke")];
    const after = [s("a", "approval")];
    const diff = diffSteps(before, after);
    expect(diff.changed).toHaveLength(1);
    expect(diff.changed[0].before.kind).toBe("skill_invoke");
    expect(diff.changed[0].after.kind).toBe("approval");
    expect(diff.added).toEqual([]);
    expect(diff.removed).toEqual([]);
  });

  it("returns no diff when proposed matches current", () => {
    const both = [s("a"), s("b")];
    const diff = diffSteps(both, both);
    expect(diff.added).toEqual([]);
    expect(diff.removed).toEqual([]);
    expect(diff.changed).toEqual([]);
    expect(diff.unchangedCount).toBe(2);
  });
});
