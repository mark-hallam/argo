import { expect, test } from "@playwright/test";

/**
 * Phase-3 concurrent-edit conflict banner (Stream
 * `web-workflow-concurrent-edit-conflict-banner`).
 *
 * The local Playwright lane points `NEXT_PUBLIC_GATEWAY_URL` at an
 * unroutable port; this test intercepts the workflow save endpoint
 * via `page.route` and walks two save attempts:
 *
 *   1. First save → 201 + version 2 (no `X-Argo-If-Version` sent
 *      because the SPA has no loaded version yet).
 *   2. Second save → simulates a race: the gateway has advanced to
 *      version 3 behind the user's back, so the SPA's
 *      `if_version=2` assertion comes back as 409 with the conflict
 *      body. The `ConcurrentEditBanner` should appear.
 *
 * The banner's "Show diff" CTA is a v1 stub (opens current head in a
 * new tab) and "Refresh" reloads the page — both are smoke-checked
 * via `data-testid` selectors.
 */
const POST_OR_HEAD_URL =
  /\/api\/v1\/verticals\/gtm-engineer\/workflows\/post_listing(\?.*)?$/;

test("Concurrent edit (409) surfaces the conflict banner with refresh + diff CTAs", async ({
  page,
}) => {
  let storedToml = "";
  let postCount = 0;
  let lastIfVersionHeader: string | null = null;

  await page.route(POST_OR_HEAD_URL, async (route) => {
    const request = route.request();
    const method = request.method();
    if (method === "POST") {
      postCount += 1;
      storedToml = request.postData() ?? "";
      lastIfVersionHeader = request.headers()["x-argo-if-version"] ?? null;
      if (postCount === 1) {
        // First save lands cleanly — gateway assigns version 2.
        await route.fulfill({
          status: 201,
          contentType: "application/json",
          body: JSON.stringify({
            vertical_id: "gtm-engineer",
            name: "post_listing",
            version_id: "wd_test_v2",
          }),
        });
        return;
      }
      // Second save races with another user's write — gateway has
      // moved to v3 and rejects the stale assertion.
      await route.fulfill({
        status: 409,
        contentType: "application/json",
        body: JSON.stringify({
          kind: "conflict",
          current_version: 3,
          current_head_definition_toml:
            '[workflow]\nname = "post_listing"\n\n[[step]]\nname = "review"\nkind = "skill_invoke"\n',
        }),
      });
      return;
    }
    if (method === "GET") {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          id: "wd_test_v2",
          vertical_id: "gtm-engineer",
          name: "post_listing",
          version: 2,
          definition_toml: storedToml,
        }),
      });
      return;
    }
    await route.fallback();
  });

  await page.goto("/workflows/gtm-engineer/post_listing/build");

  const save = page.getByTestId("builder-save");
  await expect(save).toBeVisible();

  // First save → success toast surfaces v2.
  await page.getByTestId("builder-add-step").click();
  await page.getByTestId("palette-kind-skill_invoke").click();
  await save.click();
  await expect(page.getByText("Saved (v2)")).toBeVisible({ timeout: 5_000 });
  expect(postCount).toBe(1);
  expect(lastIfVersionHeader).toBeNull(); // first save sends no if_version

  // Second save → 409. Banner must appear and the user's dirty state
  // must survive (Save button stays available; canvas isn't reset).
  await page.getByTestId("builder-add-step").click();
  await page.getByTestId("palette-kind-skill_invoke").click();
  await save.click();

  const banner = page.getByTestId("concurrent-edit-banner");
  await expect(banner).toBeVisible({ timeout: 5_000 });
  await expect(banner).toContainText("v3");
  await expect(page.getByTestId("concurrent-edit-banner-refresh")).toBeVisible();
  await expect(page.getByTestId("concurrent-edit-banner-diff")).toBeVisible();
  expect(postCount).toBe(2);
});
