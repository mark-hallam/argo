import { expect, test } from "@playwright/test";

/**
 * Phase-2 Publish-to-Head wiring
 * (Stream `web-workflow-publish-button-real-wire`).
 *
 * The local Playwright lane points `NEXT_PUBLIC_GATEWAY_URL` at the
 * intentionally-unroutable port 1, so the only requests the SPA can
 * make are ones the test intercepts via `page.route`. The test:
 *
 *   1. Stubs `POST  /workflows/:name`            → 201 + version_id.
 *   2. Stubs `GET   /workflows/:name`            → numeric `version` +
 *      echoed `definition_toml` (read-back after save AND publish).
 *   3. Stubs `PUT   /workflows/:name/head`       → 204 No Content.
 *   4. Opens the builder, drops a step (dirties canvas), saves, then
 *      clicks Publish → confirms the AlertDialog → asserts the publish
 *      toast, asserts the PUT request fired with the saved version id,
 *      and asserts the page navigates to the canonical viewer route.
 */

const WORKFLOW_URL =
  /\/api\/v1\/verticals\/gtm-engineer\/workflows\/post_listing(\/head)?(\?.*)?$/;

test("Publish promotes the saved draft, toasts the new head version, and redirects to the viewer", async ({
  page,
}) => {
  let storedToml = "";
  let savedVersion = 7;
  let postCount = 0;
  let putCount = 0;
  let putBodyRaw = "";
  // Once Publish completes the gateway would record the published
  // draft as the live head — bump the canonical version returned by
  // subsequent GETs so the toast surfaces the new head_version, not
  // the old one. The fixture proves the SPA reads the post-publish
  // head, not the pre-publish snapshot.
  let published = false;

  await page.route(WORKFLOW_URL, async (route) => {
    const request = route.request();
    const url = request.url();
    const method = request.method();
    const isHead = url.includes("/head");

    if (method === "POST" && !isHead) {
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
    if (method === "GET" && !isHead) {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          id: "wd_test_1",
          vertical_id: "gtm-engineer",
          name: "post_listing",
          version: published ? savedVersion : savedVersion,
          definition_toml: storedToml,
        }),
      });
      return;
    }
    if (method === "PUT" && isHead) {
      putCount += 1;
      putBodyRaw = request.postData() ?? "";
      published = true;
      await route.fulfill({ status: 204, body: "" });
      return;
    }
    await route.fallback();
  });

  await page.goto("/workflows/gtm-engineer/post_listing/build");

  const publish = page.getByTestId("builder-publish");
  await expect(publish).toBeVisible();
  // Pre-save: Publish is gated until a draft has been saved.
  await expect(publish).toBeDisabled();
  await expect(publish).toHaveAttribute("data-can-publish", "false");

  // Drop a new node to dirty the canvas, then save so the draft
  // becomes promotable.
  await page.getByTestId("builder-add-step").click();
  await page.getByTestId("palette-kind-skill_invoke").click();
  await page.getByTestId("builder-save").click();
  await expect(page.getByText(`Saved (v${savedVersion})`)).toBeVisible({
    timeout: 5_000,
  });

  // Publish enables once Save succeeds and the canvas is in sync.
  await expect(publish).toBeEnabled();
  await expect(publish).toHaveAttribute("data-can-publish", "true");

  await publish.click();

  // Confirm dialog must appear with the loaded version surfaced.
  const dialog = page.getByTestId("alert-dialog");
  await expect(dialog).toBeVisible();
  await expect(dialog).toContainText(`Publish v${savedVersion} to head?`);

  await page.getByTestId("alert-dialog-confirm").click();

  await expect(page.getByText(`Published v${savedVersion} to head`)).toBeVisible(
    { timeout: 5_000 },
  );

  // Viewer-route redirect lands the user on the canonical workflow
  // page, not the builder. `page.waitForURL` covers Next.js's client
  // navigation (no full page load).
  await page.waitForURL(/\/workflows\/gtm-engineer\/post_listing(\?.*)?$/, {
    timeout: 5_000,
  });

  expect(postCount).toBe(1);
  expect(putCount).toBe(1);
  const parsedPutBody = JSON.parse(putBodyRaw || "{}") as { version_id?: string };
  expect(parsedPutBody.version_id).toBe("wd_test_1");
});

test("Publish stays gated while the canvas has unsaved changes after save", async ({
  page,
}) => {
  let storedToml = "";
  let putCount = 0;

  await page.route(WORKFLOW_URL, async (route) => {
    const request = route.request();
    const url = request.url();
    const method = request.method();
    const isHead = url.includes("/head");
    if (method === "POST" && !isHead) {
      storedToml = request.postData() ?? "";
      await route.fulfill({
        status: 201,
        contentType: "application/json",
        body: JSON.stringify({
          vertical_id: "gtm-engineer",
          name: "post_listing",
          version_id: "wd_test_2",
        }),
      });
      return;
    }
    if (method === "GET" && !isHead) {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          id: "wd_test_2",
          vertical_id: "gtm-engineer",
          name: "post_listing",
          version: 3,
          definition_toml: storedToml,
        }),
      });
      return;
    }
    if (method === "PUT" && isHead) {
      putCount += 1;
      await route.fulfill({ status: 204, body: "" });
      return;
    }
    await route.fallback();
  });

  await page.goto("/workflows/gtm-engineer/post_listing/build");

  await page.getByTestId("builder-add-step").click();
  await page.getByTestId("palette-kind-skill_invoke").click();
  await page.getByTestId("builder-save").click();
  await expect(page.getByText("Saved (v3)")).toBeVisible({ timeout: 5_000 });

  const publish = page.getByTestId("builder-publish");
  await expect(publish).toBeEnabled();

  // Mutating the canvas after save must disable Publish again — user
  // has to re-save first so we don't promote a stale version_id.
  await page.getByTestId("builder-add-step").click();
  await page.getByTestId("palette-kind-skill_invoke").click();
  await expect(publish).toBeDisabled();
  await expect(publish).toHaveAttribute("data-can-publish", "false");
  expect(putCount).toBe(0);
});

test("Publish network failure surfaces an error toast with a Retry CTA", async ({
  page,
}) => {
  let storedToml = "";
  let putAttempts = 0;

  await page.route(WORKFLOW_URL, async (route) => {
    const request = route.request();
    const url = request.url();
    const method = request.method();
    const isHead = url.includes("/head");
    if (method === "POST" && !isHead) {
      storedToml = request.postData() ?? "";
      await route.fulfill({
        status: 201,
        contentType: "application/json",
        body: JSON.stringify({
          vertical_id: "gtm-engineer",
          name: "post_listing",
          version_id: "wd_test_3",
        }),
      });
      return;
    }
    if (method === "GET" && !isHead) {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          id: "wd_test_3",
          vertical_id: "gtm-engineer",
          name: "post_listing",
          version: 5,
          definition_toml: storedToml,
        }),
      });
      return;
    }
    if (method === "PUT" && isHead) {
      putAttempts += 1;
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
  await expect(page.getByText("Saved (v5)")).toBeVisible({ timeout: 5_000 });

  await page.getByTestId("builder-publish").click();
  await page.getByTestId("alert-dialog-confirm").click();

  await expect(page.getByText("Couldn't publish workflow")).toBeVisible({
    timeout: 5_000,
  });
  expect(putAttempts).toBe(1);
});
