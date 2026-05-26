import { expect, test } from "@playwright/test";

// Scene-5 / Track P bridge spec: the workflow builder's sidebar
// palette exposes the 29-connector MCP catalog AND a Computer-use
// action entry, and dropping each spawns a node with the right step
// kind + pre-populated metadata. Without this wire-up an operator
// cannot author a workflow that uses MCP or computer-use from the
// canvas (which is the Scene-5 "compose from canvas" beat).

const BUILDER_ROUTE = "/workflows/gtm-engineer/post_listing/build";

test("sidebar palette surfaces the MCP connectors group with real entries", async ({ page }) => {
  await page.goto(BUILDER_ROUTE);

  // Sidebar palette mounts.
  await expect(page.getByTestId("builder-palette")).toBeVisible();

  // The new "MCP connectors" group header is present and the first
  // few connector cards render with deterministic data-testids.
  await expect(page.getByText("MCP connectors")).toBeVisible();
  await expect(page.getByTestId("builder-palette-item-tool-mcp-airtable")).toBeVisible();
  await expect(page.getByTestId("builder-palette-item-tool-mcp-notion")).toBeVisible();
  await expect(page.getByTestId("builder-palette-item-tool-mcp-slack")).toBeVisible();

  // Computer-use lives under Tools.
  await expect(page.getByTestId("builder-palette-item-tool-cu-action")).toBeVisible();
});

test("clicking the Airtable palette item adds an mcp_invoke step with server prefilled", async ({ page }) => {
  await page.goto(BUILDER_ROUTE);

  const nodes = page.locator(".react-flow__node");
  await expect(nodes.first()).toBeVisible();
  const initialCount = await nodes.count();

  await page.getByTestId("builder-palette-item-tool-mcp-airtable").click();

  await expect(nodes).toHaveCount(initialCount + 1);

  // The new node's inspector shows the locked server badge.
  const newNode = nodes.last();
  await newNode.click();
  await expect(page.getByTestId("builder-mcp-config")).toBeVisible();
  await expect(page.getByTestId("builder-mcp-server")).toHaveText(/Airtable/);

  // Dirty flag flipped — the new step counts as an unsaved edit.
  await expect(page.getByTestId("builder-save")).toHaveAttribute("data-dirty", "true");
});

test("clicking the Computer-use palette item adds a cu_action step with substrate=macos", async ({ page }) => {
  await page.goto(BUILDER_ROUTE);

  const nodes = page.locator(".react-flow__node");
  await expect(nodes.first()).toBeVisible();
  const initialCount = await nodes.count();

  await page.getByTestId("builder-palette-item-tool-cu-action").click();

  await expect(nodes).toHaveCount(initialCount + 1);

  const newNode = nodes.last();
  await newNode.click();
  await expect(page.getByTestId("builder-cu-config")).toBeVisible();
  await expect(page.getByTestId("builder-cu-substrate")).toHaveText("macos");
  await expect(page.getByTestId("builder-cu-app")).toHaveValue("Notes.app");

  await expect(page.getByTestId("builder-save")).toHaveAttribute("data-dirty", "true");
});
