import { expect, test } from "@playwright/test";

/**
 * Stream `web-workflow-rollback-ui-button` — E2E for the Versions
 * panel + Rollback button on the workflow viewer.
 *
 * The local Playwright lane points `NEXT_PUBLIC_GATEWAY_URL` at the
 * intentionally-unroutable port 1, so the only requests the SPA can
 * make are ones the test intercepts via `page.route`. The test stubs:
 *
 *   GET  /workflows/:name              → workflow head (echoes the
 *                                        current head version).
 *   GET  /workflows/:name/versions     → version list (v1..v3).
 *   POST /workflows/:name/rollback     → returns the new head id +
 *                                        version, bumps the in-memory
 *                                        head so the next GET reflects
 *                                        the rolled-back state.
 *
 * Then walks: open viewer → click "Rollback to this version" on v1 →
 * confirm dialog text matches brief verbiage → confirm → success
 * toast → assert the head pill on v4 (the new head, copied from v1).
 */

const WORKFLOW_BASE =
  /\/api\/v1\/verticals\/pr_checker\/workflows\/standard_review(\/versions|\/rollback)?(\?.*)?$/;

const TOML_V1 = `[workflow]\ndescription = "Initial rule-based review"\n\n[[step]]\nname = "scan_diff"\nkind = "tool_call"\n`;
const TOML_V2 = `[workflow]\ndescription = "Add reviewer summary"\n\n[[step]]\nname = "scan_diff"\nkind = "tool_call"\n\n[[step]]\nname = "summarise"\nkind = "skill_invoke"\n`;
const TOML_V3 = `[workflow]\ndescription = "Add approval gate before merge"\n\n[[step]]\nname = "scan_diff"\nkind = "tool_call"\n\n[[step]]\nname = "summarise"\nkind = "skill_invoke"\n\n[[step]]\nname = "approve_merge"\nkind = "approval"\n`;

interface VersionFixture {
  id: string;
  version: number;
  parent_id: string | null;
  lifecycle_state: "active" | "archived";
  created_at: string;
  modified_by: string;
  definition_toml: string;
}

test("rollback button: confirm dialog → success toast → new head matches selected", async ({
  page,
}) => {
  const versions: VersionFixture[] = [
    {
      id: "wd_v1",
      version: 1,
      parent_id: null,
      lifecycle_state: "archived",
      created_at: "2026-05-19T09:00:00Z",
      modified_by: "alex@skycot.com",
      definition_toml: TOML_V1,
    },
    {
      id: "wd_v2",
      version: 2,
      parent_id: "wd_v1",
      lifecycle_state: "archived",
      created_at: "2026-05-20T12:30:00Z",
      modified_by: "alex@skycot.com",
      definition_toml: TOML_V2,
    },
    {
      id: "wd_v3",
      version: 3,
      parent_id: "wd_v2",
      lifecycle_state: "active",
      created_at: "2026-05-21T15:45:00Z",
      modified_by: "alex@skycot.com",
      definition_toml: TOML_V3,
    },
  ];
  let rollbackCalls = 0;
  let lastRollbackBody = "";

  await page.route(WORKFLOW_BASE, async (route) => {
    const request = route.request();
    const url = request.url();
    const method = request.method();

    if (method === "GET" && url.endsWith("/versions")) {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          vertical_id: "pr_checker",
          name: "standard_review",
          count: versions.length,
          items: versions,
        }),
      });
      return;
    }

    if (method === "POST" && url.endsWith("/rollback")) {
      rollbackCalls += 1;
      lastRollbackBody = request.postData() ?? "";
      const parsed = JSON.parse(lastRollbackBody || "{}") as {
        target_version_id?: string;
      };
      const source = versions.find((v) => v.id === parsed.target_version_id);
      if (!source) {
        await route.fulfill({
          status: 404,
          contentType: "application/json",
          body: JSON.stringify({
            error: { code: "not_found", message: "version not found" },
          }),
        });
        return;
      }
      const nextVersion = versions[versions.length - 1].version + 1;
      const newHeadId = `wd_v${nextVersion}`;
      // Demote prior head, append the new head copied from the source.
      for (const v of versions) v.lifecycle_state = "archived";
      versions.push({
        id: newHeadId,
        version: nextVersion,
        parent_id: source.id,
        lifecycle_state: "active",
        created_at: "2026-05-22T10:00:00Z",
        modified_by: "alex@skycot.com",
        definition_toml: source.definition_toml,
      });
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ id: newHeadId, version: nextVersion }),
      });
      return;
    }

    if (method === "GET" && !url.includes("/versions") && !url.includes("/rollback")) {
      const head = versions.find((v) => v.lifecycle_state === "active") ?? versions[0];
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          id: head.id,
          vertical_id: "pr_checker",
          name: "standard_review",
          version: head.version,
          parent_id: head.parent_id,
          lifecycle_state: head.lifecycle_state,
          created_at: head.created_at,
          modified_by: head.modified_by,
          definition_toml: head.definition_toml,
        }),
      });
      return;
    }

    await route.fallback();
  });

  await page.goto("/workflows/pr_checker/standard_review");

  // Live source pill confirms we're on the gateway-mocked path.
  const sourcePill = page.getByTestId("wfd-source-pill");
  await expect(sourcePill).toBeVisible();
  await expect(sourcePill).toHaveAttribute("data-source", "gateway");

  // Panel renders with 3 rows; v3 is head, v1/v2 expose rollback buttons.
  const panel = page.getByTestId("wfd-versions-panel");
  await expect(panel).toBeVisible();
  await expect(page.getByTestId("wfd-versions-row-3")).toBeVisible();
  await expect(page.getByTestId("wfd-versions-head-pill-3")).toBeVisible();

  // Roll back to v1.
  await page.getByTestId("wfd-versions-rollback-1").click();

  const modal = page.getByTestId("wfd-versions-confirm-modal");
  await expect(modal).toBeVisible();
  await expect(page.getByTestId("wfd-versions-confirm-title")).toHaveText(
    "Roll back to v1?",
  );
  await expect(page.getByTestId("wfd-versions-confirm-body")).toHaveText(
    "Current head is v3, drafted v4 will move forward.",
  );

  await page.getByTestId("wfd-versions-confirm-submit").click();

  // Success toast with the new head version surfaced.
  await expect(
    page.getByText("Rolled back to v1. New head is v4."),
  ).toBeVisible({ timeout: 5_000 });

  // Modal closes; panel re-fetches and v4 is the new head, copied from v1.
  await expect(modal).toBeHidden();
  await expect(page.getByTestId("wfd-versions-head-pill-4")).toBeVisible();

  // Backend assertions: exactly one rollback call, body carried the
  // selected target id.
  expect(rollbackCalls).toBe(1);
  const parsed = JSON.parse(lastRollbackBody || "{}") as {
    target_version_id?: string;
  };
  expect(parsed.target_version_id).toBe("wd_v1");
});

test("rollback failure: 5xx surfaces error toast with retry, modal stays open", async ({
  page,
}) => {
  let rollbackAttempts = 0;

  await page.route(WORKFLOW_BASE, async (route) => {
    const request = route.request();
    const url = request.url();
    const method = request.method();

    if (method === "GET" && url.endsWith("/versions")) {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          vertical_id: "pr_checker",
          name: "standard_review",
          count: 2,
          items: [
            {
              id: "wd_v1",
              version: 1,
              parent_id: null,
              lifecycle_state: "archived",
              created_at: "2026-05-19T09:00:00Z",
              modified_by: "alex@skycot.com",
              definition_toml: TOML_V1,
            },
            {
              id: "wd_v2",
              version: 2,
              parent_id: "wd_v1",
              lifecycle_state: "active",
              created_at: "2026-05-20T12:30:00Z",
              modified_by: "alex@skycot.com",
              definition_toml: TOML_V2,
            },
          ],
        }),
      });
      return;
    }

    if (method === "POST" && url.endsWith("/rollback")) {
      rollbackAttempts += 1;
      await route.fulfill({
        status: 503,
        contentType: "application/json",
        body: JSON.stringify({
          error: { code: "gateway_down", message: "gateway unavailable" },
        }),
      });
      return;
    }

    if (method === "GET" && !url.includes("/versions") && !url.includes("/rollback")) {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          id: "wd_v2",
          vertical_id: "pr_checker",
          name: "standard_review",
          version: 2,
          definition_toml: TOML_V2,
        }),
      });
      return;
    }
    await route.fallback();
  });

  await page.goto("/workflows/pr_checker/standard_review");
  await expect(page.getByTestId("wfd-versions-panel")).toBeVisible();

  await page.getByTestId("wfd-versions-rollback-1").click();
  await page.getByTestId("wfd-versions-confirm-submit").click();

  await expect(page.getByText("Rollback failed")).toBeVisible({ timeout: 5_000 });
  // Modal stays open with the error banner — retries still possible.
  await expect(page.getByTestId("wfd-versions-confirm-modal")).toBeVisible();
  await expect(page.getByTestId("wfd-versions-confirm-error")).toBeVisible();

  expect(rollbackAttempts).toBe(1);
});
