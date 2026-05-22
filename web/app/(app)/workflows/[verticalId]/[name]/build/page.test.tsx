/**
 * Coverage for the gateway-vs-seed swap in the builder page.
 *
 * The full builder page mounts React Flow + TanStack Query + the Next
 * router which would test infrastructure rather than the literal-swap
 * behaviour the brief is paying for (same rationale as the viewer
 * `page.test.tsx` next door — see its top comment). We test the pure
 * `resolveBuilderWorkflowSource` resolver directly: gateway hit wins,
 * gateway miss + demo mode falls back to `SEED_WORKFLOWS`, gateway miss
 * outside demo mode returns `undefined` so the page can show its
 * empty / not-found state.
 */
import { describe, expect, it } from "vitest";

import { resolveBuilderWorkflowSource } from "./resolve-workflow";
import { SEED_WORKFLOWS } from "./seed-workflows";
import type { WorkflowSpec } from "./types";

const GATEWAY_WORKFLOW: WorkflowSpec = {
  name: "post_listing",
  vertical: "gtm-engineer",
  description: "Gateway-decoded workflow",
  version: "v3",
  risk: "approval-required",
  steps: [
    { id: "step-1", kind: "trigger", name: "from gateway", description: "" },
  ],
};

describe("resolveBuilderWorkflowSource", () => {
  it("renders gateway data when present (not the seed)", () => {
    const result = resolveBuilderWorkflowSource({
      verticalId: "gtm-engineer",
      name: "post_listing",
      templateId: null,
      data: { workflow: GATEWAY_WORKFLOW },
      demoMode: false,
    });
    expect(result).toBe(GATEWAY_WORKFLOW);
    expect(result?.steps[0].name).toBe("from gateway");
    // Negative — the seed for this key has different step names.
    expect(result).not.toBe(SEED_WORKFLOWS["gtm-engineer/post_listing"]);
  });

  it("falls back to SEED_WORKFLOWS on gateway 404 when demo mode is on", () => {
    const result = resolveBuilderWorkflowSource({
      verticalId: "gtm-engineer",
      name: "post_listing",
      templateId: null,
      data: { workflow: null },
      demoMode: true,
    });
    expect(result).toBe(SEED_WORKFLOWS["gtm-engineer/post_listing"]);
    expect(result?.steps.length).toBeGreaterThan(0);
  });

  it("returns undefined on gateway 404 outside demo mode", () => {
    const result = resolveBuilderWorkflowSource({
      verticalId: "gtm-engineer",
      name: "post_listing",
      templateId: null,
      data: { workflow: null },
      demoMode: false,
    });
    expect(result).toBeUndefined();
  });

  it("returns undefined while the query is still loading (data === undefined)", () => {
    const result = resolveBuilderWorkflowSource({
      verticalId: "gtm-engineer",
      name: "post_listing",
      templateId: null,
      data: undefined,
      demoMode: false,
    });
    expect(result).toBeUndefined();
  });

  it("uses ?from_template= over the gateway when set", () => {
    const result = resolveBuilderWorkflowSource({
      verticalId: "gtm-engineer",
      name: "post_listing",
      // Real id from `lib/workflow-templates.ts` (sales-qualify-lead is
      // the first entry); keeps the test grounded if the template seed
      // list ever rotates.
      templateId: "sales-qualify-lead",
      data: { workflow: GATEWAY_WORKFLOW },
      demoMode: false,
    });
    expect(result).not.toBe(GATEWAY_WORKFLOW);
    expect(result?.version).toBe("draft");
    expect(result?.name).toBe("qualify_lead");
  });

  it("returns undefined when templateId points to an unknown template", () => {
    const result = resolveBuilderWorkflowSource({
      verticalId: "gtm-engineer",
      name: "post_listing",
      templateId: "no.such.template.exists",
      data: { workflow: GATEWAY_WORKFLOW },
      demoMode: true,
    });
    expect(result).toBeUndefined();
  });
});
