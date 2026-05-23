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

  it("maps Pro tier to slack / imessage / gmail and Enterprise everywhere else", () => {
    expect(tierFor("slack")).toBe("pro");
    expect(tierFor("imessage")).toBe("pro");
    expect(tierFor("gmail")).toBe("pro");
    expect(tierFor("notion")).toBe("enterprise");
    expect(tierFor("salesforce")).toBe("enterprise");
  });
});
