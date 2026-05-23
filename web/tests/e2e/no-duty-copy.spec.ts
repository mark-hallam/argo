import { expect, test } from "@playwright/test";

/**
 * Operator-vocabulary contract — "Duty" / "Duties" are internal-only
 * words; operator-visible SPA copy uses "Schedule" / "Schedules".
 *
 * Locked 2026-05-23 via brief
 * `streams/web-rename-duty-to-schedule-in-ui-copy.md`. Source: operator
 * memory `project_vocabulary_duty_vs_schedule`. The 8 routes below
 * cover the in-product surface area where the SPA previously surfaced
 * the engineering word. The check inspects `document.body.innerText`
 * — i.e. what an operator can actually read — and ignores test-ids,
 * comments, code identifiers, and API paths.
 */

const PAGES: Array<{ name: string; path: string }> = [
  { name: "settings", path: "/settings" },
  { name: "workflows", path: "/workflows" },
  { name: "cohorts", path: "/cohorts" },
  { name: "workers", path: "/workers" },
  { name: "audit", path: "/audit" },
  { name: "approvals", path: "/approvals" },
  { name: "skills", path: "/skills" },
  { name: "hire", path: "/hire" },
];

const FORBIDDEN = /\b(dut(y|ies))\b/i;

for (const { name, path } of PAGES) {
  test(`${name} page contains no operator-visible "Duty" / "Duties" copy`, async ({
    page,
  }) => {
    await page.goto(path);
    // Wait for the body to render — let the router settle.
    await expect(page.locator("body")).toBeVisible();
    const innerText = await page.evaluate(() => document.body.innerText);
    expect(innerText, `${name} page surfaced a forbidden word`).not.toMatch(
      FORBIDDEN,
    );
  });
}

/**
 * Positive complement — the three pages where the rename actively
 * landed must surface "Schedule" / "Schedules" copy in their operator-
 * visible body text. This guards against the easy regression of the
 * forbidden-word check passing simply because the strings vanished
 * entirely.
 */
const POSITIVE: Array<{ name: string; path: string }> = [
  { name: "settings", path: "/settings" },
  { name: "workflows", path: "/workflows" },
  { name: "cohorts", path: "/cohorts" },
];

for (const { name, path } of POSITIVE) {
  test(`${name} page surfaces "Schedule" / "Schedules" copy`, async ({ page }) => {
    await page.goto(path);
    await expect(page.locator("body")).toBeVisible();
    const innerText = await page.evaluate(() => document.body.innerText);
    expect(
      /\b(schedule|schedules)\b/i.test(innerText),
      `${name} page should surface schedule copy after the duty→schedule rename`,
    ).toBe(true);
  });
}
