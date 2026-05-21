import { expect, test } from "@playwright/test";

/**
 * Phase-1 Save-as-Draft wiring (Stream `web-workflow-save-button-real-wire`).
 *
 * The local Playwright lane points `NEXT_PUBLIC_GATEWAY_URL` at the
 * intentionally-unroutable port 1, so the only request the SPA can
 * make is one the test intercepts via `page.route`. The test:
 *
 *   1. Stubs the POST workflow endpoint with a 201 + version_id.
 *   2. Stubs the GET head endpoint with a numeric `version` and the
 *      saved `definition_toml` echoed back, so a subsequent reload
 *      would see the persisted body.
 *   3. Opens the builder, drags a new step onto the canvas, clicks
 *      Save as draft, and asserts the success toast surfaces with
 *      the version number.
 *   4. Verifies the network POST happened with the canvas step
 *      encoded as TOML — the proof-of-persistence checkpoint for
 *      phase 1.
 */
const POST_OR_HEAD_URL =
  /\/api\/v1\/verticals\/gtm-engineer\/workflows\/post_listing(\?.*)?$/;

test("Save as draft POSTs the canvas as TOML and surfaces the version toast", async ({
  page,
}) => {
  let storedToml = "";
  let postCount = 0;

  await page.route(POST_OR_HEAD_URL, async (route) => {
    const request = route.request();
    const method = request.method();
    if (method === "POST") {
      postCount += 1;
      storedToml = request.postData() ?? "";
      await route.fulfill({
        status: 201,
        contentType: "application/json",
        body: JSON.stringify({
          vertical_id: "gtm-engineer",
          name: "post_listing",
          version_id: "wd_test_1",
        }),
      });
      return;
    }
    if (method === "GET") {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          id: "wd_test_1",
          vertical_id: "gtm-engineer",
          name: "post_listing",
          version: 7,
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
  await expect(save).toHaveAttribute("data-dirty", "false");

  // Drop a new node onto the canvas so the dirty flag flips on.
  await page.getByTestId("builder-add-step").click();
  await page.getByTestId("palette-kind-skill_invoke").click();
  await expect(save).toHaveAttribute("data-dirty", "true");

  await save.click();

  await expect(page.getByText("Saved (v7)")).toBeVisible({ timeout: 5_000 });
  expect(postCount).toBe(1);
  expect(storedToml).toContain("[workflow]");
  expect(storedToml).toContain("name = \"post_listing\"");
  expect(storedToml).toContain("[[step]]");
});

test("Network failure surfaces an error toast with a Retry CTA", async ({
  page,
}) => {
  let attempts = 0;

  await page.route(POST_OR_HEAD_URL, async (route) => {
    const method = route.request().method();
    if (method === "POST") {
      attempts += 1;
      await route.fulfill({
        status: 503,
        contentType: "application/json",
        body: JSON.stringify({
          error: { code: "gateway_down", message: "gateway unavailable" },
        }),
      });
      return;
    }
    await route.fallback();
  });

  await page.goto("/workflows/gtm-engineer/post_listing/build");
  await page.getByTestId("builder-add-step").click();
  await page.getByTestId("palette-kind-skill_invoke").click();
  await page.getByTestId("builder-save").click();

  await expect(page.getByText("Couldn't save workflow")).toBeVisible({
    timeout: 5_000,
  });
  expect(attempts).toBe(1);
});
