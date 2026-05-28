import { expect, test } from "@playwright/test";

/**
 * Error-reporting runaway-loop regression (F16 from UX audit
 * 2026-05-27-0415). The SPA's global `window.error` listener used to
 * refire its own fetch on CORS failure — `/api/v1/meta/error-reporting`
 * returns without `Access-Control-Allow-Origin`, the fetch rejects,
 * the rejection fires `unhandledrejection`, and the handler retries.
 * Four of five audit walks wedged the renderer within ~30s of any
 * 5-page navigation cycle.
 *
 * This spec drives /landing → /skills → /workers → /team → /approvals
 * with the gateway unreachable (the Playwright lane points
 * `NEXT_PUBLIC_GATEWAY_URL` at `http://127.0.0.1:1`) and asserts that
 * the total number of requests aimed at `meta/error-reporting` stays
 * bounded.
 *
 * Brief: streams/fix-spa-error-reporting-init-fetch-loop-on-cors-failure.md
 */

test("five sequential navigations fire at most one error-reporting config probe", async ({
  page,
}) => {
  const reporterHits: string[] = [];
  page.on("request", (req) => {
    const url = req.url();
    if (url.includes("/api/v1/meta/error-reporting")) {
      reporterHits.push(url);
    }
  });

  for (const path of ["/landing", "/skills", "/workers", "/team", "/approvals"]) {
    await page.goto(path, { waitUntil: "domcontentloaded" });
  }
  await page.waitForTimeout(750);

  // The reporter is mount-once-per-session. Across 5 navigations we
  // expect at most one config probe (the empty-deps useEffect from
  // ErrorReportingInit in app/layout.tsx). React strict mode can
  // double-fire in dev, so allow up to 2; the documented loop was
  // 20+ per cycle.
  expect(reporterHits.length).toBeLessThanOrEqual(2);
});
