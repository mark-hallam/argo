import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import {
  TEMPLATE_COUNT,
  TEMPLATE_VERTICALS,
  VERTICAL_LABEL,
  WORKFLOW_TEMPLATES,
  getTemplate,
  getTemplatesByVertical,
} from "@/lib/workflow-templates";
import WorkflowTemplatesPage from "@/app/(app)/workflows/templates/page";

describe("workflow-templates seed data", () => {
  it("ships at least 50 templates", () => {
    expect(TEMPLATE_COUNT).toBeGreaterThanOrEqual(50);
    expect(WORKFLOW_TEMPLATES.length).toBe(TEMPLATE_COUNT);
  });

  it("has unique template ids", () => {
    const ids = WORKFLOW_TEMPLATES.map((t) => t.id);
    expect(new Set(ids).size).toBe(ids.length);
  });

  it("covers every declared vertical with at least 4 templates", () => {
    const by = getTemplatesByVertical();
    for (const v of TEMPLATE_VERTICALS) {
      expect(by[v], `vertical=${v} should have templates`).toBeDefined();
      expect(by[v].length).toBeGreaterThanOrEqual(4);
    }
  });

  it("every template has steps and a non-empty description", () => {
    for (const tpl of WORKFLOW_TEMPLATES) {
      expect(tpl.steps.length, `${tpl.id} should have steps`).toBeGreaterThan(0);
      expect(tpl.description.length).toBeGreaterThan(20);
      expect(tpl.name).toMatch(/^[a-z][a-z0-9_]+$/);
    }
  });

  it("approval-required templates have at least one approval step", () => {
    for (const tpl of WORKFLOW_TEMPLATES.filter((t) => t.risk === "approval-required")) {
      const hasGate = tpl.steps.some((s) => s.kind === "approval");
      expect(hasGate, `${tpl.id} marked approval-required must include an approval step`).toBe(true);
    }
  });

  it("getTemplate returns matching record by id", () => {
    const sample = WORKFLOW_TEMPLATES[0];
    expect(getTemplate(sample.id)).toEqual(sample);
    expect(getTemplate("does-not-exist")).toBeUndefined();
  });
});

describe("WorkflowTemplatesPage gallery", () => {
  it("renders all templates by default and shows the total count", () => {
    render(<WorkflowTemplatesPage />);
    const grid = screen.getByTestId("templates-grid");
    expect(grid.children.length).toBe(WORKFLOW_TEMPLATES.length);
    expect(screen.getByText(/TEMPLATE GALLERY/)).toBeInTheDocument();
  });

  it("filters by vertical chip click", async () => {
    const user = userEvent.setup();
    render(<WorkflowTemplatesPage />);
    await user.click(screen.getByTestId("templates-vertical-sales"));
    const grid = screen.getByTestId("templates-grid");
    const salesCount = WORKFLOW_TEMPLATES.filter((t) => t.vertical === "sales").length;
    expect(grid.children.length).toBe(salesCount);
  });

  it("filters by risk chip click", async () => {
    const user = userEvent.setup();
    render(<WorkflowTemplatesPage />);
    await user.click(screen.getByTestId("templates-risk-autonomous"));
    const grid = screen.getByTestId("templates-grid");
    const autonomousCount = WORKFLOW_TEMPLATES.filter((t) => t.risk === "autonomous").length;
    expect(grid.children.length).toBe(autonomousCount);
  });

  it("narrows results by free-text search", async () => {
    const user = userEvent.setup();
    render(<WorkflowTemplatesPage />);
    const search = screen.getByTestId("templates-search");
    await user.type(search, "invoice");
    const grid = screen.getByTestId("templates-grid");
    const expected = WORKFLOW_TEMPLATES.filter((t) =>
      [t.name, t.description, ...t.steps.map((s) => s.name + s.description)]
        .join(" ")
        .toLowerCase()
        .includes("invoice"),
    ).length;
    expect(grid.children.length).toBe(expected);
  });

  it("shows the vertical label for every declared vertical somewhere on the page", () => {
    render(<WorkflowTemplatesPage />);
    for (const v of TEMPLATE_VERTICALS) {
      expect(screen.getAllByText(VERTICAL_LABEL[v]).length).toBeGreaterThan(0);
    }
  });

  it("renders an empty state when filters match nothing", async () => {
    const user = userEvent.setup();
    render(<WorkflowTemplatesPage />);
    const search = screen.getByTestId("templates-search");
    await user.type(search, "qqzzz_nothing_matches_this");
    expect(screen.queryByTestId("templates-grid")).toBeNull();
    expect(screen.getByText(/No templates match those filters/)).toBeInTheDocument();
  });
});
