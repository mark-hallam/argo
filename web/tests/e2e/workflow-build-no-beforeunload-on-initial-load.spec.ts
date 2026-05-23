import { expect, test } from "@playwright/test";

/**
 * Regression for the round-25 walkthrough finding:
 *
 *   "🟡 P2 workflow build editor fires beforeunload on INITIAL load
 *    with no edits (dirty-flag seeded wrong)"
 *
 * The builder used to seed `useWorkflowBuilderState` with empty
 * `[]`/`[]` on the first render (workflow still loading), then a
 * second render arrived with the loaded spec and `dirty` flipped
 * true without a single user interaction — making the browser show
 * its "Discard changes?" prompt on the first refresh or back-button.
 *
 * The fix re-seeds state when the initial arrays change identity.
 * This spec proves the visible symptom is gone: after the canvas
 * paints, dispatching a `beforeunload` event must NOT be cancelled
 * (no handler registered while `dirty=false`).
 */
test("workflow builder does not arm beforeunload on initial load", async ({ page }) => {
  await page.goto("/workflows/gtm-engineer/post_listing/build");

  // Toolbar mount is the cheapest signal that the page has reached the
  // builder render (past suspense/skeleton/empty states); then give the
  // workflow fetch + any post-mount ReactFlow callbacks a beat to
  // settle. Without the fix in #use-workflow-builder-state.ts, dirty
  // would still flip true after this point.
  const save = page.getByTestId("builder-save");
  await expect(save).toBeVisible();
  await page.waitForTimeout(1500);
  await expect(save).toHaveAttribute("data-dirty", "false");

  const prevented = await page.evaluate(() => {
    const evt = new Event("beforeunload", { cancelable: true });
    window.dispatchEvent(evt);
    return evt.defaultPrevented;
  });
  expect(prevented).toBe(false);
});
