import { expect, test } from "@playwright/test";

/**
 * Demo-bypass mode must not spew `ClientFetchError` from NextAuth's
 * `SessionProvider` on every navigation.
 *
 * The local playwright lane boots with
 * `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1` (see `playwright.config.ts`),
 * which under the old behavior mounted a stock `<SessionProvider>` that
 * polled `/api/auth/session` and surfaced a config-error fetch in the
 * console on every route change. The fix in
 * `components/skycot/AuthProvider.tsx` short-circuits the polling
 * (`refetchInterval={0}` + `session={null}`) under demo-bypass, so the
 * console must be free of `ClientFetchError` after a multi-page walk.
 *
 * Source: `fix-spa-sessionprovider-disable-in-demo-bypass` /
 * `docs/skycot/ux-audits/2026-05-26-2152/` F2 [P0].
 */

const ROUTES = ["/", "/team", "/audit", "/approvals", "/skills", "/workflows"];

test("demo-bypass walk does not log NextAuth ClientFetchError", async ({
  page,
}) => {
  const fetchErrors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() !== "error") return;
    const text = msg.text();
    if (text.includes("ClientFetchError")) fetchErrors.push(text);
  });
  page.on("pageerror", (err) => {
    if (err.message.includes("ClientFetchError")) {
      fetchErrors.push(err.message);
    }
  });

  for (const route of ROUTES) {
    await page.goto(route, { waitUntil: "domcontentloaded" });
  }

  expect(fetchErrors).toEqual([]);
});
