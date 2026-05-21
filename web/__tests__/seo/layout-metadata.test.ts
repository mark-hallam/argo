import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

import { SEED_CUSTOMERS } from "@/lib/content/customers";

// Closes web-seo-canonical-and-twitter-handle: every public marketing
// layout must declare an `alternates.canonical` pointing at its public
// URL path, every description must fit inside Google's ~155-char SERP
// truncation cap, and the twitter:site handle must thread through both
// the root layout and the `customers/[slug]` layout (which redefines
// `twitter`, so the root value is otherwise lost during shallow merge).

const THIS_DIR = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = join(THIS_DIR, "..", "..", "..");
const WEB_DIR = join(REPO_ROOT, "web");
const APP_DIR = join(WEB_DIR, "app");

const DESCRIPTION_MAX = 155;
const TWITTER_SITE = "@skycot_ai";

// Public marketing surfaces. Excludes `(app)`, `(hire)`, `(setup)`,
// `auth/*`, and `api/*` routes — those are not indexable and don't
// need canonical URLs. Excludes `customers/[slug]` because that uses
// `generateMetadata` (covered separately below).
const PUBLIC_LAYOUTS: { layout: string; canonical: string }[] = [
  { layout: "landing/layout.tsx", canonical: "/landing" },
  { layout: "customers/layout.tsx", canonical: "/customers" },
  { layout: "compare/layout.tsx", canonical: "/compare" },
  { layout: "pricing/layout.tsx", canonical: "/pricing" },
  { layout: "changelog/layout.tsx", canonical: "/changelog" },
  { layout: "docs/layout.tsx", canonical: "/docs" },
  { layout: "privacy/layout.tsx", canonical: "/privacy" },
  { layout: "terms/layout.tsx", canonical: "/terms" },
  { layout: "security/layout.tsx", canonical: "/security" },
  { layout: "help/layout.tsx", canonical: "/help" },
  { layout: "for/sales/layout.tsx", canonical: "/for/sales" },
  { layout: "for/support/layout.tsx", canonical: "/for/support" },
  { layout: "for/marketing/layout.tsx", canonical: "/for/marketing" },
  { layout: "for/ops/layout.tsx", canonical: "/for/ops" },
  { layout: "for/finance/layout.tsx", canonical: "/for/finance" },
  { layout: "for/recruiting/layout.tsx", canonical: "/for/recruiting" },
];

function readLayout(relPath: string): string {
  return readFileSync(join(APP_DIR, relPath), "utf8");
}

function extractStringValue(text: string, key: string): string | null {
  const re = new RegExp(`${key}:\\s*\\n?\\s*"([^"]*)"`, "m");
  const m = text.match(re);
  return m ? m[1] : null;
}

describe("SEO metadata — canonical, twitter:site, description length", () => {
  it("root layout declares alternates.canonical, twitter.site, and ≤155-char description", () => {
    const text = readLayout("layout.tsx");

    expect(text, "root layout must set alternates.canonical").toMatch(
      /alternates:\s*\{\s*canonical:\s*"\/"/,
    );

    expect(
      text,
      `root layout must set twitter.site to ${TWITTER_SITE}`,
    ).toMatch(new RegExp(`site:\\s*"${TWITTER_SITE.replace("@", "@")}"`));

    const description = extractStringValue(text, "description");
    expect(description, "root layout must declare a description").not.toBeNull();
    expect(
      description!.length,
      `root description length ${description!.length} exceeds ${DESCRIPTION_MAX}`,
    ).toBeLessThanOrEqual(DESCRIPTION_MAX);
  });

  it.each(PUBLIC_LAYOUTS)(
    "$layout declares canonical $canonical and a ≤155-char description",
    ({ layout, canonical }) => {
      const text = readLayout(layout);

      const re = new RegExp(
        `alternates:\\s*\\{\\s*canonical:\\s*"${canonical}"`,
      );
      expect(text, `${layout} must set alternates.canonical to ${canonical}`).toMatch(re);

      const description = extractStringValue(text, "description");
      if (description !== null) {
        expect(
          description.length,
          `${layout} description length ${description.length} exceeds ${DESCRIPTION_MAX}`,
        ).toBeLessThanOrEqual(DESCRIPTION_MAX);
      }
    },
  );

  it("customers/[slug] layout declares dynamic canonical + twitter.site + ≤155-char description for every seed", () => {
    const text = readLayout("customers/[slug]/layout.tsx");

    expect(text, "must set dynamic canonical from slug").toMatch(
      /canonical:\s*`\/customers\/\$\{slug\}`/,
    );

    expect(text, `must set twitter.site to ${TWITTER_SITE}`).toMatch(
      new RegExp(`site:\\s*"${TWITTER_SITE.replace("@", "@")}"`),
    );

    for (const story of SEED_CUSTOMERS) {
      // Mirrors the formula in customers/[slug]/layout.tsx. If that
      // formula changes, update here so the cap stays enforced.
      const description = `${story.name} (${story.industry}) — ${story.headlineKpi}`;
      expect(
        description.length,
        `customer ${story.slug} description length ${description.length} exceeds ${DESCRIPTION_MAX}: ${description}`,
      ).toBeLessThanOrEqual(DESCRIPTION_MAX);
    }
  });
});
