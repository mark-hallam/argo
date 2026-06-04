/**
 * Surface-contract test for the canonical audit-API endpoint paths.
 *
 * Pins the canonical 3-row contract exported by
 * `web/lib/api/audit-detail.ts` (`AUDIT_ENDPOINT_PATHS`) and asserts the
 * two deprecated paths (`/api/v1/audit/actions`, `/api/v1/audit/events`)
 * never reappear as runtime call paths anywhere in the SPA API client.
 *
 * Why this exists: the audit contract drifted before (bare-array
 * tolerance on `/api/v1/workers`, undocumented legacy audit paths). A
 * surface-contract test makes the contract a hard, reviewable boundary
 * rather than a convention that erodes silently.
 */
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import {
  AUDIT_ENDPOINT_PATHS,
  DEPRECATED_AUDIT_ENDPOINT_PATHS,
} from "@/lib/api/audit-detail";

const API_DIR = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "../lib/api",
);

// The contract lives in `audit-detail.ts` (the `DEPRECATED_*` constant
// literally names the forbidden strings) and the test files document
// the same intent — exclude them so the scan flags only real runtime
// call paths.
const SCAN_EXCLUDED = new Set(["audit-detail.ts"]);

function apiClientSources(): { rel: string; content: string }[] {
  return fs
    .readdirSync(API_DIR)
    .filter((name) => name.endsWith(".ts"))
    .filter((name) => !name.endsWith(".test.ts"))
    .filter((name) => !SCAN_EXCLUDED.has(name))
    .map((name) => ({
      rel: `lib/api/${name}`,
      content: fs.readFileSync(path.join(API_DIR, name), "utf8"),
    }));
}

describe("audit-API surface contract", () => {
  it("pins the canonical 3-row audit endpoint contract", () => {
    expect(AUDIT_ENDPOINT_PATHS.feed).toBe("/api/v1/audit");
    expect(AUDIT_ENDPOINT_PATHS.detail("act_42")).toBe("/api/v1/audit/act_42");
    expect(AUDIT_ENDPOINT_PATHS.replay("act_42")).toBe(
      "/api/v1/audit/act_42/replay",
    );
  });

  it("URL-encodes the action id in detail + replay paths", () => {
    expect(AUDIT_ENDPOINT_PATHS.detail("a/b 7")).toBe("/api/v1/audit/a%2Fb%207");
    expect(AUDIT_ENDPOINT_PATHS.replay("a/b 7")).toBe(
      "/api/v1/audit/a%2Fb%207/replay",
    );
  });

  it("names exactly the two deprecated paths", () => {
    expect([...DEPRECATED_AUDIT_ENDPOINT_PATHS]).toEqual([
      "/api/v1/audit/actions",
      "/api/v1/audit/events",
    ]);
  });

  it("never reintroduces a deprecated audit path in the API client", () => {
    const offenders: string[] = [];
    for (const { rel, content } of apiClientSources()) {
      for (const deprecated of DEPRECATED_AUDIT_ENDPOINT_PATHS) {
        if (content.includes(deprecated)) {
          offenders.push(`${rel} → ${deprecated}`);
        }
      }
    }
    expect(offenders).toEqual([]);
  });

  it("routes the audit-detail drill-in through AUDIT_ENDPOINT_PATHS.detail", () => {
    const detailSource = fs.readFileSync(
      path.join(API_DIR, "audit-detail.ts"),
      "utf8",
    );
    expect(detailSource).toContain("AUDIT_ENDPOINT_PATHS.detail(actionId)");
  });
});
