import { expect, test, type WebSocketRoute } from "@playwright/test";

/**
 * Approvals inbox — WebSocket live-updates (E1-f).
 *
 * Uses Playwright's [`page.routeWebSocket`] (1.48+) to intercept the
 * SPA's connection to `/inbox/ws` before it hits the network, so the
 * test runs against the same auth-bypass mock dataset as the other
 * e2e specs (no real gateway required).
 *
 * Asserts:
 *   1. LiveIndicator flips from `connecting` → `live` once the WS
 *      handshake completes (we simulate it by registering an
 *      `onMessage` handler — the route becomes active on first send).
 *   2. An `approval.expired` event with `approval_id = ap_01H9Z7`
 *      surgically removes that approval from the rendered list while
 *      leaving the other six mock cards in place.
 */
test("WS approval.expired surgically removes the matching card", async ({ page }) => {
  let route: WebSocketRoute | null = null;
  await page.routeWebSocket(/\/inbox\/ws/, (ws) => {
    route = ws;
    // Server-side hello frame — matches the J3 protocol envelope so
    // the wrapper treats this as a healthy live connection.
    ws.send(
      JSON.stringify({
        type: "hello",
        protocol_version: "1.0",
        capabilities: [
          "approval.created",
          "approval.decided",
          "approval.undone",
          "approval.expired",
        ],
        seq: 0,
      }),
    );
    // Echo pings so the wrapper's heartbeat deadline never trips.
    ws.onMessage((msg) => {
      try {
        const parsed = JSON.parse(typeof msg === "string" ? msg : msg.toString());
        if (parsed?.type === "ping") ws.send(JSON.stringify({ type: "pong" }));
      } catch { /* ignore */ }
    });
  });

  await page.goto("/");

  const indicator = page.getByTestId("live-indicator");
  await expect(indicator).toBeVisible();
  await expect(indicator).toHaveAttribute("data-status", "live");

  // Mariah's "wants to send this listing" card (ap_01H9Z7) renders
  // from the bundled mock dataset (see `sample-approvals.ts`).
  const targetVerb = page.getByText("wants to send this listing");
  await expect(targetVerb.first()).toBeVisible();
  const initialApproveCount = await page.getByTestId("approval-approve").count();

  // Fire the surgical event. The hook's `applyEvent()` filters the
  // matching id out of the TanStack Query cache; React re-renders the
  // page with one fewer card.
  expect(route).not.toBeNull();
  route!.send(
    JSON.stringify({
      type: "approval.expired",
      payload: { approval_id: "ap_01H9Z7" },
      seq: 1,
    }),
  );

  await expect(targetVerb).toHaveCount(0);
  const remainingApproveCount = await page.getByTestId("approval-approve").count();
  expect(remainingApproveCount).toBe(initialApproveCount - 1);
});
