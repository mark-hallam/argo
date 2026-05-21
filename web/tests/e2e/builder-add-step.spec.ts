import { expect, test } from "@playwright/test";

// Verifies the Add Step palette adds a real node to the canvas. The
// happy path: click "+ Add step", click "Skill" in the palette, expect
// a new node with the default name "New skill".
test("Add Step palette inserts a new node with the chosen kind", async ({ page }) => {
  await page.goto("/workflows/gtm-engineer/post_listing/build");

  const save = page.getByTestId("builder-save");
  await expect(save).toBeVisible();
  await expect(save).toHaveAttribute("data-dirty", "false");

  const nodesLocator = page.locator(".react-flow__node");
  await expect(nodesLocator.first()).toBeVisible();
  const initialCount = await nodesLocator.count();

  await page.getByTestId("builder-add-step").click();
  await expect(page.getByRole("dialog", { name: "Add step palette" })).toBeVisible();

  await page.getByTestId("palette-kind-skill_invoke").click();

  // Palette closes after selection.
  await expect(page.getByRole("dialog", { name: "Add step palette" })).toBeHidden();

  // A new node has landed on the canvas.
  await expect(nodesLocator).toHaveCount(initialCount + 1);
  await expect(page.getByText("New skill")).toBeVisible();

  // Save button flips to enabled — dirty flag flows from the new node.
  await expect(save).toHaveAttribute("data-dirty", "true");
  await expect(save).toBeEnabled();
});

test("Escape closes the Add Step palette without adding a node", async ({ page }) => {
  await page.goto("/workflows/gtm-engineer/post_listing/build");

  const nodesLocator = page.locator(".react-flow__node");
  await expect(nodesLocator.first()).toBeVisible();
  const initialCount = await nodesLocator.count();

  await page.getByTestId("builder-add-step").click();
  await expect(page.getByRole("dialog", { name: "Add step palette" })).toBeVisible();

  await page.keyboard.press("Escape");
  await expect(page.getByRole("dialog", { name: "Add step palette" })).toBeHidden();
  await expect(nodesLocator).toHaveCount(initialCount);
});
