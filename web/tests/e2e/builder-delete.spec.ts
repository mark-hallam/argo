import { expect, test } from "@playwright/test";

// Verifies the workflow builder removes nodes (and their incident edges)
// when the user selects a node and presses Delete or Backspace, and that
// trigger nodes refuse deletion with a guard toast.
test.describe("workflow builder — delete", () => {
  test("selecting a non-trigger node and pressing Delete removes node + incident edges", async ({
    page,
  }) => {
    await page.goto("/workflows/gtm-engineer/post_listing/build");

    const allNodes = page.locator(".react-flow__node");
    await expect(allNodes.first()).toBeVisible();
    const before = await allNodes.count();
    expect(before).toBeGreaterThan(1);

    const beforeEdges = await page.locator(".react-flow__edge").count();

    // Click the second node (s2 — Validate inventory; the first is the
    // trigger which is delete-protected).
    const target = allNodes.nth(1);
    await target.click();
    await page.keyboard.press("Delete");

    await expect(allNodes).toHaveCount(before - 1);
    // Removing a middle node also removes the two edges touching it.
    const afterEdges = await page.locator(".react-flow__edge").count();
    expect(afterEdges).toBeLessThan(beforeEdges);

    // Undo via the toast action restores the node.
    await page.getByRole("button", { name: /undo/i }).click();
    await expect(allNodes).toHaveCount(before);
  });

  test("Backspace works as an alias for Delete", async ({ page }) => {
    await page.goto("/workflows/gtm-engineer/post_listing/build");
    const allNodes = page.locator(".react-flow__node");
    await expect(allNodes.first()).toBeVisible();
    const before = await allNodes.count();

    await allNodes.nth(1).click();
    await page.keyboard.press("Backspace");
    await expect(allNodes).toHaveCount(before - 1);
  });

  test("trigger nodes refuse deletion and remain on the canvas", async ({ page }) => {
    await page.goto("/workflows/gtm-engineer/post_listing/build");
    const allNodes = page.locator(".react-flow__node");
    await expect(allNodes.first()).toBeVisible();
    const before = await allNodes.count();

    // The first node is the trigger (Seller submits inventory).
    await allNodes.first().click();
    await page.keyboard.press("Delete");

    // Count is unchanged — addNode restored the optimistic removal.
    await expect(allNodes).toHaveCount(before);
    await expect(page.getByText(/trigger steps can't be deleted/i)).toBeVisible();
  });
});
