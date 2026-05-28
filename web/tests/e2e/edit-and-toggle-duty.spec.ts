import { expect, test } from "@playwright/test";

/**
 * Worker-detail Edit + Toggle on the Duties card (brief
 * `fix-worker-detail-edit-toggle-stub-toasts`).
 *
 * The local Playwright lane points the SPA at an unroutable gateway so
 * the worker overview falls back to bundled sample data
 * (`sample-worker-overview.ts` → Mariah's duty list). We intercept the
 * PATCH `/api/v1/workers/<id>/duties/<duty-id>` requests directly with
 * `page.route()` so the test exercises the live wire shape without
 * needing a real gateway.
 *
 * Anti-leak guard: assertion (4) walks the visible card text and
 * refuses any `E<digits>-<letter>` engineering brief-ID (the earlier
 * stub-toast leak was `"Edit schedule ships in E4-h"`).
 */

// Mariah — `019e6785-928a-7270-a766-f945b6cd7161` in
// `web/lib/sample-personas.ts`. Her overview ships with 4 duties; the
// first is "d-list" with schedule "as items arrive".
const MARIAH_ID = "019e6785-928a-7270-a766-f945b6cd7161";
const DUTY_ID = "d-list";

const PATCH_PATH_RE = new RegExp(
  `/api/v1/workers/${MARIAH_ID}/duties/${DUTY_ID}$`,
);

type PatchCall = { method: string; body: unknown };

test("toggle on a duty row fires a real PATCH with enabled=false", async ({
  page,
}) => {
  const calls: PatchCall[] = [];
  await page.route(PATCH_PATH_RE, async (route) => {
    const req = route.request();
    const body = req.postData() ? JSON.parse(req.postData()!) : null;
    calls.push({ method: req.method(), body });
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        duty: {
          id: DUTY_ID,
          worker_id: MARIAH_ID,
          cron: "*/15 * * * *",
          entry_action: "list_inventory",
          label: "List new inventory with photos + tags",
          enabled: false,
          paused_until: null,
          last_fired_at: null,
          created_at: "2026-05-20T00:00:00Z",
        },
      }),
    });
  });

  await page.goto(`/workers/${MARIAH_ID}/overview`);
  const toggle = page.getByTestId(`duty-toggle-${DUTY_ID}`);
  await expect(toggle).toBeVisible();
  await expect(toggle).not.toBeDisabled();
  await toggle.click();
  await expect.poll(() => calls.length).toBeGreaterThanOrEqual(1);
  expect(calls[0]?.method).toBe("PATCH");
  expect(calls[0]?.body).toMatchObject({ enabled: false });
});

test("clicking the duty body opens the EditDutyDialog", async ({ page }) => {
  await page.goto(`/workers/${MARIAH_ID}/overview`);
  const body = page.getByTestId(`duty-edit-${DUTY_ID}`);
  await expect(body).toBeVisible();
  await expect(body).not.toBeDisabled();
  await body.click();
  await expect(page.getByTestId("edit-duty-dialog")).toBeVisible();
});

test("saving the dialog fires PATCH with the edited fields", async ({
  page,
}) => {
  const calls: PatchCall[] = [];
  await page.route(PATCH_PATH_RE, async (route) => {
    const req = route.request();
    const body = req.postData() ? JSON.parse(req.postData()!) : null;
    calls.push({ method: req.method(), body });
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        duty: {
          id: DUTY_ID,
          worker_id: MARIAH_ID,
          cron: "0 7 * * *",
          entry_action: "list_inventory",
          label: "List new inventory each morning",
          enabled: true,
          paused_until: null,
          last_fired_at: null,
          created_at: "2026-05-20T00:00:00Z",
        },
      }),
    });
  });

  await page.goto(`/workers/${MARIAH_ID}/overview`);
  await page.getByTestId(`duty-edit-${DUTY_ID}`).click();
  const dialog = page.getByTestId("edit-duty-dialog");
  await expect(dialog).toBeVisible();
  const labelInput = dialog.getByTestId("edit-duty-label");
  await labelInput.fill("List new inventory each morning");
  const cronInput = dialog.getByTestId("edit-duty-cron");
  await cronInput.fill("0 7 * * *");
  await dialog.getByTestId("edit-duty-save").click();
  await expect.poll(() => calls.length).toBeGreaterThanOrEqual(1);
  expect(calls[0]?.method).toBe("PATCH");
  expect(calls[0]?.body).toMatchObject({
    label: "List new inventory each morning",
    schedule: "0 7 * * *",
  });
  await expect(dialog).toBeHidden();
});

test("the Duties card never renders an engineering brief-ID", async ({
  page,
}) => {
  await page.goto(`/workers/${MARIAH_ID}/overview`);
  const card = page.getByTestId("card-duties");
  await expect(card).toBeVisible();
  const text = (await card.textContent()) ?? "";
  // E4-h was the prior leak; this matches any /E\d+-[a-z]/ pattern.
  expect(text).not.toMatch(/\bE\d+-[a-z]\b/);
});
