/**
 * Pin test for `web-audit-export-format-strings-fix`.
 *
 * The four compliance-shaped format strings the SPA's evidence-bundle
 * dialog exposes (`soc2-type-ii` / `gdpr-article-15` /
 * `hipaa-disclosure-log` / `generic`) MUST be mapped to the gateway's
 * `tar` envelope before the POST body leaves the browser. The
 * gateway's `parse_format` (crates/argo-cli/src/handlers/gateway/
 * routes_audit_export.rs) accepts only `json | csv | tar` and returns
 * 400 `bad_request: unknown format` on any of the compliance strings.
 *
 * Regression contract: before the fix, the API client passed
 * `params.format` straight through to the POST body. Replaying the
 * pre-fix behaviour against this test asserts the unmapped string
 * would be on the wire — which is exactly what produced the 400 in
 * production.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import {
  complianceFormatToBundleFormat,
  generateExport,
  previewExport,
} from "./audit-export";
import type { ExportFormat, ExportParams } from "@/lib/sample-export";

const COMPLIANCE_FORMATS: ExportFormat[] = [
  "soc2-type-ii",
  "gdpr-article-15",
  "hipaa-disclosure-log",
  "generic",
];

const ORIGINAL_FETCH = globalThis.fetch;

function makeJsonResponse(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

interface CapturedCall {
  url: string;
  body: unknown;
}

function installFetchCapture(
  response: (url: string) => Response,
): { calls: CapturedCall[]; fetchMock: ReturnType<typeof vi.fn> } {
  const calls: CapturedCall[] = [];
  const fetchMock = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
    const url =
      typeof input === "string"
        ? input
        : input instanceof URL
          ? input.toString()
          : input.url;
    if (url.endsWith("/api/auth/session")) {
      return makeJsonResponse(200, {});
    }
    let parsedBody: unknown = undefined;
    if (typeof init?.body === "string") {
      try {
        parsedBody = JSON.parse(init.body);
      } catch {
        parsedBody = init.body;
      }
    }
    calls.push({ url, body: parsedBody });
    return response(url);
  });
  globalThis.fetch = fetchMock as unknown as typeof fetch;
  return { calls, fetchMock };
}

function paramsFor(format: ExportFormat): ExportParams {
  return {
    scope: { kind: "filtered" },
    format,
    tenantId: "tenant_acme",
  };
}

describe("complianceFormatToBundleFormat", () => {
  it.each(COMPLIANCE_FORMATS)(
    "maps %s to the gateway tar envelope",
    (format) => {
      expect(complianceFormatToBundleFormat(format)).toBe("tar");
    },
  );
});

describe("audit-export API client: compliance-format → tar mapping", () => {
  const originalEnv = { ...process.env };

  beforeEach(() => {
    process.env.NEXT_PUBLIC_GATEWAY_URL = "http://gw.test";
    vi.stubEnv("NODE_ENV", "test");
    delete process.env.NEXT_PUBLIC_ENV;
    delete process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS;
  });

  afterEach(() => {
    globalThis.fetch = ORIGINAL_FETCH;
    process.env = { ...originalEnv };
    vi.unstubAllEnvs();
    vi.restoreAllMocks();
  });

  it.each(COMPLIANCE_FORMATS)(
    "generateExport POSTs format=tar (not %s) so the gateway returns 200",
    async (format) => {
      const { calls } = installFetchCapture((url) => {
        if (url.endsWith("/api/v1/audit/export")) {
          return makeJsonResponse(201, {
            kind: "immediate",
            url: "/mock-exports/x.zip",
            fileName: "x.zip",
            sizeEstimate: 1234,
          });
        }
        if (url.endsWith("/api/v1/audit")) {
          return makeJsonResponse(200, {});
        }
        return makeJsonResponse(404, { error: { code: "not_found", message: "n/a" } });
      });

      await generateExport(paramsFor(format));

      const exportCall = calls.find((c) =>
        c.url.endsWith("/api/v1/audit/export"),
      );
      expect(exportCall, "expected POST to /api/v1/audit/export").toBeDefined();
      const body = exportCall!.body as { format?: string };
      expect(body.format).toBe("tar");
      expect(body.format).not.toBe(format);
    },
  );

  it.each(COMPLIANCE_FORMATS)(
    "previewExport POSTs format=tar (not %s)",
    async (format) => {
      const { calls } = installFetchCapture(() =>
        makeJsonResponse(200, { count: 1, sizeEstimate: 1 }),
      );

      await previewExport(paramsFor(format));

      const previewCall = calls.find((c) =>
        c.url.endsWith("/api/v1/audit/export/preview"),
      );
      expect(previewCall).toBeDefined();
      const body = previewCall!.body as { format?: string };
      expect(body.format).toBe("tar");
    },
  );

  it("does not leak the compliance label onto the gateway wire body", async () => {
    const { calls } = installFetchCapture(() =>
      makeJsonResponse(201, {
        kind: "immediate",
        url: "/mock.zip",
        fileName: "mock.zip",
        sizeEstimate: 0,
      }),
    );

    await generateExport(paramsFor("soc2-type-ii"));

    const exportCall = calls.find((c) => c.url.endsWith("/api/v1/audit/export"));
    expect(exportCall).toBeDefined();
    const serialised = JSON.stringify(exportCall!.body);
    for (const compliance of COMPLIANCE_FORMATS) {
      expect(serialised).not.toContain(compliance);
    }
  });
});
