import { expect, test } from "@playwright/test";

// Verifies the right-click context menu surface on the workflow builder
// canvas: Duplicate clones a step (+1 node count) and the edge-context
// menu's Disconnect removes a single edge while keeping its endpoints.
test.describe("workflow builder — context menu", () => {
  test("right-click on a node opens the menu and Duplicate clones the step", async ({ page }) => {
    await page.goto("/workflows/gtm-engineer/post_listing/build");
    const allNodes = page.locator(".react-flow__node");
    await expect(allNodes.first()).toBeVisible();
    const before = await allNodes.count();

    // Right-click the second node (a regular skill_invoke step).
    await allNodes.nth(1).click({ button: "right" });
    const menu = page.getByTestId("workflow-context-menu");
    await expect(menu).toBeVisible();
    await menu.getByRole("menuitem", { name: /duplicate/i }).click();

    await expect(allNodes).toHaveCount(before + 1);
    await expect(menu).toBeHidden();
  });

  test("right-click on a trigger node shows a disabled Duplicate item", async ({ page }) => {
    await page.goto("/workflows/gtm-engineer/post_listing/build");
    const allNodes = page.locator(".react-flow__node");
    await expect(allNodes.first()).toBeVisible();

    await allNodes.first().click({ button: "right" });
    const menu = page.getByTestId("workflow-context-menu");
    await expect(menu).toBeVisible();
    const duplicateItem = menu.getByRole("menuitem", { name: /duplicate/i });
    await expect(duplicateItem).toBeDisabled();
    await page.keyboard.press("Escape");
    await expect(menu).toBeHidden();
  });

  test("right-click on an edge opens a Disconnect-only menu that removes only the edge", async ({
    page,
  }) => {
    await page.goto("/workflows/gtm-engineer/post_listing/build");
    const nodes = page.locator(".react-flow__node");
    const edges = page.locator(".react-flow__edge");
    await expect(nodes.first()).toBeVisible();
    const beforeNodes = await nodes.count();
    const beforeEdges = await edges.count();
    expect(beforeEdges).toBeGreaterThan(0);

    await edges.first().click({ button: "right" });
    const menu = page.getByTestId("workflow-context-menu");
    await expect(menu).toBeVisible();
    const items = menu.getByRole("menuitem");
    await expect(items).toHaveCount(1);
    await items.first().click();

    // One fewer edge, same node count.
    await expect(edges).toHaveCount(beforeEdges - 1);
    await expect(nodes).toHaveCount(beforeNodes);
  });
});
