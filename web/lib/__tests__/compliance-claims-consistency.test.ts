import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

import { VERTICALS } from "@/app/for/_data/verticals";

/**
 * Guards against the 2026-05-27 over-claim where every `/for/<vertical>`
 * page hardcoded `GDPR · HIPAA · AFSL` regardless of vertical, even though
 * SKYCOT does not hold HIPAA Business Associate Agreements or an ASIC
 * Australian Financial Services License. Any regulated badge that appears
 * on a vertical landing page must also appear on `/security` — the legal
 * source of truth for what the company claims.
 *
 * Vitest is invoked from the `web/` directory (see `web/package.json`'s
 * `test` script), so a CWD-relative path resolves the security page
 * deterministically across local dev + CI.
 */

const SECURITY_PAGE_SOURCE = readFileSync("app/security/page.tsx", "utf8");

/**
 * Regulated-or-formal badges that buyers' legal teams treat as load-bearing.
 * Mentioning any of these on a vertical landing page is a legal claim;
 * it must be backed by the security page (and ultimately by an attestation,
 * BAA, license, or similar instrument).
 */
const REGULATED_BADGES = [
  "HIPAA",
  "AFSL",
  "ISO 27001",
  "ISO27001",
  "SOC 2 Type II",
  "SOC 2 Type 2",
  "PCI DSS",
  "PCI-DSS",
  "FedRAMP",
] as const;

describe("/for vertical landing pages — compliance claim consistency", () => {
  it("no vertical claims HIPAA or AFSL (SKYCOT holds neither today)", () => {
    const offenders: Array<{ slug: string; claim: string }> = [];
    for (const vertical of Object.values(VERTICALS)) {
      for (const claim of vertical.compliance) {
        if (/\bHIPAA\b/.test(claim) || /\bAFSL\b/.test(claim)) {
          offenders.push({ slug: vertical.slug, claim });
        }
      }
    }
    expect(offenders).toEqual([]);
  });

  it("every regulated badge on a vertical page also appears on /security", () => {
    const mismatched: Array<{
      slug: string;
      claim: string;
      missingBadge: string;
    }> = [];
    for (const vertical of Object.values(VERTICALS)) {
      for (const claim of vertical.compliance) {
        for (const badge of REGULATED_BADGES) {
          if (!claim.includes(badge)) continue;
          if (!SECURITY_PAGE_SOURCE.includes(badge)) {
            mismatched.push({ slug: vertical.slug, claim, missingBadge: badge });
          }
        }
      }
    }
    expect(mismatched).toEqual([]);
  });

  it("every vertical defines at least one compliance claim", () => {
    for (const vertical of Object.values(VERTICALS)) {
      expect(
        vertical.compliance.length,
        `vertical "${vertical.slug}" has no compliance claims`,
      ).toBeGreaterThan(0);
    }
  });
});
