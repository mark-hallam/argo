import { describe, expect, it } from "vitest";

import {
  TEMPLATE_COUNT,
  WORKFLOW_TEMPLATES,
  getTemplate,
} from "@/lib/workflow-templates";

/**
 * Shape + differentiator-coverage tests for the workflow-templates seed.
 *
 * Two of these tests are grep-guards added by the scene-4-and-5 F16 fix:
 * the template marketplace must surface Skycot's two distinguishing step
 * kinds (mcp_invoke + cu_action). If they drop back to zero, this file
 * fails — preventing silent regression of the differentiator surface.
 */
describe("workflow-templates catalog", () => {
  it("every template has a unique id", () => {
    const ids = WORKFLOW_TEMPLATES.map((tpl) => tpl.id);
    const seen = new Set(ids);
    expect(seen.size).toBe(ids.length);
  });

  it("every template has at least one step", () => {
    for (const tpl of WORKFLOW_TEMPLATES) {
      expect(tpl.steps.length, `template ${tpl.id} has no steps`).toBeGreaterThan(0);
    }
  });

  it("getTemplate(id) resolves every catalog entry", () => {
    for (const tpl of WORKFLOW_TEMPLATES) {
      expect(getTemplate(tpl.id)?.id).toBe(tpl.id);
    }
  });

  it("TEMPLATE_COUNT matches WORKFLOW_TEMPLATES.length", () => {
    expect(TEMPLATE_COUNT).toBe(WORKFLOW_TEMPLATES.length);
  });

  it("includes at least 2 templates that use the mcp_invoke step kind", () => {
    const count = WORKFLOW_TEMPLATES.filter((tpl) =>
      tpl.steps.some((step) => step.kind === "mcp_invoke"),
    ).length;
    expect(count, "MCP connector differentiator missing from template gallery").toBeGreaterThanOrEqual(2);
  });

  it("includes at least 1 template that uses the cu_action step kind", () => {
    const count = WORKFLOW_TEMPLATES.filter((tpl) =>
      tpl.steps.some((step) => step.kind === "cu_action"),
    ).length;
    expect(count, "Computer-use differentiator missing from template gallery").toBeGreaterThanOrEqual(1);
  });
});
