/**
 * Acceptance test for the onboarding pick-vertical wizard's catalog
 * source.
 *
 * Originally pinned `fix-onboarding-pick-vertical-skill-counts-and-
 * gateway-catalog`. Updated by `fix-wizard-verticals-rewrite-scope-
 * and-dead-fallback` to flip the fetch path from the bare `/verticals`
 * (not proxied through `web/next.config.ts`'s `/api/v1/*` rewrite, so
 * the SPA was hitting its own dev origin and falling through to
 * SEED_VERTICALS) to `/api/v1/verticals` (which IS proxied and, post-
 * #2456, reads the on-disk pack catalog via `argo_vertical::load_dir`
 * — same data as the legacy route).
 *
 * The brief's load-bearing claims:
 * - `/api/v1/verticals` returns `{verticals: VerticalRow[]}` rows
 *   carrying `skill_count` that matches `skills_added` from
 *   `POST /api/v1/onboarding/install-vertical`.
 * - Wizard fetches the catalog from the gateway via `useVerticals()`
 *   and renders that count on each card.
 * - Mock-offline fallback (`SEED_VERTICALS`) carries skill counts
 *   that match what the on-disk packs actually install.
 *
 * The hook is exercised in `web/lib/api/verticals.ts`; this file
 * pins the fetch path + envelope shape + skill-count parity so the
 * card numbers never drift back into client-side guesses.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  fetchVerticals,
  SEED_VERTICALS,
  type VerticalRow,
} from "@/lib/api/verticals";

const ORIGINAL_FETCH = globalThis.fetch;

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    statusText: `Status ${status}`,
    headers: { "content-type": "application/json" },
  });
}

interface MockOptions {
  verticalsBody?: unknown;
  verticalsStatus?: number;
}

function installFetchMock(opts: MockOptions = {}) {
  const calls: string[] = [];
  const fetchMock = vi.fn((input: RequestInfo | URL) => {
    const url =
      typeof input === "string"
        ? input
        : input instanceof URL
          ? input.toString()
          : input.url;
    calls.push(url);
    if (url.endsWith("/api/auth/session")) {
      return Promise.resolve(jsonResponse({}));
    }
    if (url.endsWith("/api/v1/verticals")) {
      return Promise.resolve(
        jsonResponse(
          opts.verticalsBody ?? { verticals: [] },
          opts.verticalsStatus ?? 200,
        ),
      );
    }
    return Promise.resolve(
      jsonResponse(
        { error: { code: "not_mocked", message: `unexpected ${url}` } },
        500,
      ),
    );
  });
  globalThis.fetch = fetchMock as unknown as typeof fetch;
  return { fetchMock, calls };
}

describe("onboarding pick-vertical catalogue", () => {
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

  it("fetches /api/v1/verticals (not the bare /verticals) for the catalog", async () => {
    const row: VerticalRow = {
      id: "marketplace-seller",
      name: "Marketplace Seller",
      version: "0.1.0",
      description: "Inventory + listings + buyer replies.",
      headline: "Inventory + listings + buyer replies.",
      tags: ["seller"],
      workflow_count: 0,
      action_count: 6,
      role_label: "Marketplace Seller",
      kpi_count: 0,
      skill_count: 2,
    };
    const { calls } = installFetchMock({
      verticalsBody: { verticals: [row] },
    });

    const result = await fetchVerticals();

    // The `/api/v1/*` namespace is the only path proxied through
    // `web/next.config.ts` rewrites in dev. Calling bare `/verticals`
    // hit the SPA's own Next dev origin, returned the HTML 200 page,
    // and JSON-parse fell through to SEED_VERTICALS — the "v1
    // catalogue (offline · 5 verticals)" the camera saw on /onboarding
    // /pick-vertical even though the gateway had 30+ packs loaded.
    // Post-#2456 the canonical `/api/v1/verticals` endpoint reads the
    // pack catalogue from disk via `argo_vertical::load_dir` and
    // returns `{verticals: [...]}` — same data, proxied path.
    expect(calls.some((c) => c.endsWith("/api/v1/verticals"))).toBe(true);
    expect(result.source).toBe("gateway");
    expect(result.verticals).toHaveLength(1);
    expect(result.verticals[0]?.skill_count).toBe(2);
  });

  it("preserves gateway skill_count so card display matches install_vertical", async () => {
    // Mirrors the brief's DOM-evidence table: each row's `skill_count`
    // is what the install-vertical endpoint will actually install.
    const gatewayRows: VerticalRow[] = [
      makeRow("gtm-engineer", "GTM Engineer", 8),
      makeRow("marketing-intern", "Marketing Intern", 5),
      makeRow("marketplace-seller", "Marketplace Seller", 2),
      makeRow("recruiter", "Recruiter", 5),
      makeRow("sales-sdr", "Sales SDR", 5),
      makeRow("customer-support", "Customer Support", 5),
      makeRow("finance-ar", "Accounts Receivable", 5),
    ];
    installFetchMock({
      verticalsBody: { verticals: gatewayRows },
    });

    const result = await fetchVerticals();

    expect(result.source).toBe("gateway");
    const bySkill: Record<string, number | undefined> = {};
    for (const v of result.verticals) {
      bySkill[v.id] = v.skill_count;
    }
    expect(bySkill["marketplace-seller"]).toBe(2);
    expect(bySkill["gtm-engineer"]).toBe(8);
    expect(bySkill.recruiter).toBe(5);
  });

  it("falls back to SEED_VERTICALS when the gateway returns an empty catalog", async () => {
    installFetchMock({
      verticalsBody: { verticals: [] },
    });

    const result = await fetchVerticals();

    expect(result.source).toBe("mock-offline");
    expect(result.verticals).toBe(SEED_VERTICALS);
    expect(result.verticals.length).toBeGreaterThan(0);
  });

  it("falls back when the gateway returns 404", async () => {
    installFetchMock({
      verticalsBody: {
        error: { code: "not_found", message: "no verticals endpoint" },
      },
      verticalsStatus: 404,
    });

    const result = await fetchVerticals();

    expect(result.source).toBe("mock-offline");
    expect(result.verticals).toBe(SEED_VERTICALS);
  });

  it("SEED_VERTICALS skill counts match on-disk install_vertical output", () => {
    // Pinning these here keeps the offline fallback honest. If a
    // pack's `*.skill.md` count changes on disk, update the seed
    // row in the same PR so the wizard stays consistent end-to-end.
    const bySkill: Record<string, number | undefined> = {};
    for (const v of SEED_VERTICALS) {
      bySkill[v.id] = v.skill_count;
    }
    expect(bySkill["gtm-engineer"]).toBe(8);
    expect(bySkill["sales-sdr"]).toBe(5);
    expect(bySkill["customer-support"]).toBe(5);
    expect(bySkill["marketing-intern"]).toBe(5);
    expect(bySkill["finance-ar"]).toBe(5);
  });
});

function makeRow(id: string, name: string, skillCount: number): VerticalRow {
  return {
    id,
    name,
    version: "0.1.0",
    description: name,
    headline: name,
    tags: [id],
    workflow_count: 0,
    action_count: 0,
    role_label: name,
    kpi_count: 0,
    skill_count: skillCount,
  };
}
