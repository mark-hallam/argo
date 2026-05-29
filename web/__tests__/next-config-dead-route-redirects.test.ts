import { describe, expect, it } from "vitest";

import nextConfig from "../next.config";

/**
 * Pin: the dead-route catcher redirects for `/workflows/runs` and
 * `/workflows/edit` stay configured in `web/next.config.ts`. Pair-test
 * for the `scripts/check-no-dead-spa-routes.sh` lint — the lint
 * prevents new in-product links to the dead routes, this test
 * prevents the redirect rules from being silently removed.
 *
 * Closes UX-AUDIT-T0519 F11 jointly with the lint script.
 */

interface RedirectEntry {
  source: string;
  destination: string;
  permanent: boolean;
}

describe("next.config redirects() — dead-route catchers", () => {
  it("redirects /workflows/runs to the audit feed filtered to workflow.execute", async () => {
    expect(nextConfig.redirects).toBeDefined();
    const redirects = (await nextConfig.redirects!()) as RedirectEntry[];
    const entry = redirects.find((r) => r.source === "/workflows/runs");
    expect(entry, "/workflows/runs redirect must be present").toBeDefined();
    expect(entry!.destination).toBe(
      "/audit?range=7d&action_type=workflow.execute",
    );
    expect(entry!.permanent).toBe(false);
  });

  it("redirects /workflows/edit to the workflows catalog", async () => {
    const redirects = (await nextConfig.redirects!()) as RedirectEntry[];
    const entry = redirects.find((r) => r.source === "/workflows/edit");
    expect(entry, "/workflows/edit redirect must be present").toBeDefined();
    expect(entry!.destination).toBe("/workflows");
    expect(entry!.permanent).toBe(false);
  });
});
