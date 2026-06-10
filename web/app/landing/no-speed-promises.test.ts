import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";
import { join } from "node:path";

/**
 * DEC-048 rule 3 regression: the time-to-hire "speed promises" are DELETED from
 * the user-facing onboarding funnel. The operator's lock — "the 4 minutes
 * promise can be deleted, it's more important that we take the time to build the
 * AI teammate right the first time." This pins the hero surfaces named in the
 * brief; product PERFORMANCE metrics elsewhere (e.g. median reply/resolution
 * time) are a different claim and intentionally out of scope.
 */

// vitest runs with cwd at the `web/` root (where vitest.config.ts lives).
const WEB_ROOT = process.cwd();

const SURFACES = [
  "app/landing/page.tsx",
  "app/auth/signup/page.tsx",
  "app/auth/signup/layout.tsx",
  "app/(hire)/hire/page.tsx",
  "app/(hire)/hire/step/[step]/page.tsx",
  "app/(setup)/onboarding/welcome/page.tsx",
  "components/skycot/genesis/HeroInput.tsx",
];

const SPEED_PROMISE = /four minutes|4 minutes|ten seconds|90 sec|2\.5 min/i;

describe("onboarding funnel carries no speed promises (DEC-048)", () => {
  it.each(SURFACES)("%s is free of time-to-hire promises", (rel) => {
    const src = readFileSync(join(WEB_ROOT, rel), "utf8");
    expect(src).not.toMatch(SPEED_PROMISE);
  });
});
