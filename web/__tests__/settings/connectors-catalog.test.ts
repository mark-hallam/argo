import { describe, expect, it } from "vitest";

import { CONNECTOR_CATALOG, findConnectorById } from "@/app/(app)/settings/connectors/connector-catalog";
import { tierFor } from "@/app/(app)/settings/connectors/tier";

/**
 * Snapshot guards on the operator-facing connector catalog.
 *
 * The 29-row count is load-bearing: the 2026-05-23 inventory
 * (`docs/skycot/evals/scheduled/2026-05-23/eval-mcp-connector-real-vs-stub-inventory.md`)
 * found 29 REAL connectors. If `argo-mcp-connector-*` crates ship or
 * sunset, the inventory + this test should move together.
 */
describe("settings/connectors catalog", () => {
  it("ships exactly 29 connector entries (matches 2026-05-23 inventory)", () => {
    expect(CONNECTOR_CATALOG).toHaveLength(29);
  });

  it("uses unique connector ids", () => {
    const ids = CONNECTOR_CATALOG.map((c) => c.id);
    expect(new Set(ids).size).toBe(ids.length);
  });

  it("declares an auth type for every entry", () => {
    for (const c of CONNECTOR_CATALOG) {
      expect(["oauth", "apikey"]).toContain(c.authType);
    }
  });

  it("provides a name + description for every entry (no empty cards)", () => {
    for (const c of CONNECTOR_CATALOG) {
      expect(c.name.trim().length).toBeGreaterThan(0);
      expect(c.description.trim().length).toBeGreaterThan(0);
    }
  });

  it("includes every connector listed in the F-01 evaluator finding", () => {
    const required = [
      "notion",
      "linear",
      "pagerduty",
      "github-issues",
      "asana",
      "airtable",
      "zendesk",
      "discord",
      "stripe",
      "trello",
      "typeform",
      "figma",
      "google-meet",
      "calendly",
      "mailchimp",
      "sendgrid",
      "postmark",
      "monday",
      "fireflies",
      "clickup",
      "imessage",
      "onedrive",
      "salesforce",
    ];
    for (const id of required) {
      expect(findConnectorById(id), `missing connector: ${id}`).toBeDefined();
    }
  });

  it("auth type of every WIRED connector matches the backend connector_authorize_specs.rs source of truth", () => {
    // The gateway's `connector_authorize_specs.rs` is the source of truth for
    // the 17 wired connectors' auth kind: OAuth (browser redirect dance) vs
    // Token (an apikey POST). The catalog's `authType` drives which connect
    // flow the SPA offers, so a mismatch sends a non-dev down the wrong path.
    // Pinned to prevent the drift this fix corrected from recurring
    // (slack/jira/asana/linear/monday/discord were mislabeled "oauth";
    // notion was mislabeled "apikey").
    const backendAuth: Record<string, "oauth" | "apikey"> = {
      // OAuth (5) — need the gateway /connect redirect route (item ④)
      gmail: "oauth",
      gcal: "oauth",
      hubspot: "oauth",
      notion: "oauth",
      salesforce: "oauth",
      // Token / apikey (12) — connect via a credential POST today
      slack: "apikey",
      jira: "apikey",
      asana: "apikey",
      linear: "apikey",
      monday: "apikey",
      pagerduty: "apikey",
      fireflies: "apikey",
      airtable: "apikey",
      stripe: "apikey",
      zendesk: "apikey",
      clickup: "apikey",
      discord: "apikey",
    };
    for (const [id, expected] of Object.entries(backendAuth)) {
      const c = findConnectorById(id);
      expect(c, `wired connector missing from catalog: ${id}`).toBeDefined();
      expect(c?.authType, `${id} authType must match backend specs`).toBe(expected);
    }
  });

  it("maps Pro tier to slack / imessage / gmail and Enterprise everywhere else", () => {
    expect(tierFor("slack")).toBe("pro");
    expect(tierFor("imessage")).toBe("pro");
    expect(tierFor("gmail")).toBe("pro");
    expect(tierFor("notion")).toBe("enterprise");
    expect(tierFor("salesforce")).toBe("enterprise");
  });
});
