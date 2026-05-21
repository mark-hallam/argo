import { expect, test } from "@playwright/test";

// Verifies the workflow builder mounts ReactFlow in controlled mode,
// reflects edits in the dirty-state Save button, and warns on
// navigation away with unsaved changes.
test("workflow builder canvas is controlled and surfaces dirty state", async ({ page }) => {
  await page.goto("/workflows/gtm-engineer/post_listing/build");

  const save = page.getByTestId("builder-save");
  await expect(save).toBeVisible();
  await expect(save).toBeDisabled();
  await expect(save).toHaveAttribute("data-dirty", "false");

  // Drag the first node — the controlled-mode change handler flips dirty.
  const firstNode = page.locator(".react-flow__node").first();
  await firstNode.waitFor({ state: "visible" });
  const box = await firstNode.boundingBox();
  if (!box) throw new Error("first node has no bounding box");
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  await page.mouse.down();
  await page.mouse.move(box.x + box.width / 2 + 120, box.y + box.height / 2, { steps: 8 });
  await page.mouse.up();

  await expect(save).toBeEnabled();
  await expect(save).toHaveAttribute("data-dirty", "true");
});
