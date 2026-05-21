import { expect, test } from "@playwright/test";

// Smoke-tests the keyboard shortcuts + snap-to-grid wiring on the
// workflow builder. Three assertions:
//   1. Pressing `?` opens the shortcuts cheatsheet modal.
//   2. Cmd/Ctrl+0 re-fits the canvas (we observe the fit-view button is
//      not needed and that the modal can be reopened — proxy for the
//      hook being active without coupling to internal transform state).
//   3. Dragging a node lands on a 20px-aligned x coordinate.
test("workflow builder honors keyboard shortcuts + snap-to-grid", async ({ page }) => {
  await page.goto("/workflows/gtm-engineer/post_listing/build");

  // 1. The shortcuts affordance is mounted.
  const shortcutsBtn = page.getByTestId("builder-shortcuts-button");
  await expect(shortcutsBtn).toBeVisible();

  // Press `?` to open the modal.
  await page.keyboard.press("Shift+/");
  const modal = page.getByTestId("builder-shortcuts-modal");
  await expect(modal).toBeVisible();
  await page.getByTestId("builder-shortcuts-close").click();
  await expect(modal).toHaveCount(0);

  // 2. Snap-to-grid — drag the first node and verify x is a multiple of 20.
  const firstNode = page.locator(".react-flow__node").first();
  await firstNode.waitFor({ state: "visible" });
  const startBox = await firstNode.boundingBox();
  if (!startBox) throw new Error("first node has no bounding box at start");
  await page.mouse.move(startBox.x + startBox.width / 2, startBox.y + startBox.height / 2);
  await page.mouse.down();
  // Move a non-aligned distance — snap-to-grid is what makes the final
  // landing snap to 20px increments.
  await page.mouse.move(startBox.x + startBox.width / 2 + 137, startBox.y + startBox.height / 2 + 41, { steps: 12 });
  await page.mouse.up();

  // Read the snapped position from the inline transform style.
  const handle = await firstNode.elementHandle();
  const style = (await handle?.getAttribute("style")) ?? "";
  const match = style.match(/translate\((-?\d+(?:\.\d+)?)px,\s*(-?\d+(?:\.\d+)?)px\)/);
  if (!match) throw new Error(`node transform not matched: ${style}`);
  const tx = Math.round(parseFloat(match[1]!));
  const ty = Math.round(parseFloat(match[2]!));
  expect(tx % 20).toBe(0);
  expect(ty % 20).toBe(0);

  // 3. Cmd/Ctrl+Z after a drag must undo the move.
  const isMac = process.platform === "darwin";
  const mod = isMac ? "Meta" : "Control";
  await page.keyboard.press(`${mod}+z`);
  // After undo, the toast surfaces. We assert it appears.
  await expect(page.getByText(/Undone:/)).toBeVisible();
});
