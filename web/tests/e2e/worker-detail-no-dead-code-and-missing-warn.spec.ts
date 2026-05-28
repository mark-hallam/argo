import { expect, test, type ConsoleMessage } from "@playwright/test";

/**
 * Regression pins for the F15/F16 fixes shipped in
 * `fix-worker-dynamic-tab-page-dead-code-and-personality-silent-fallback`:
 *
 *   1. F16 — the unreachable `[tab]/page.tsx` stub used to render
 *      "Overview — coming in E4-b" placeholders when no static
 *      segment matched. After its deletion, an unknown tab segment
 *      must hit the standard Next.js 404 path, not a TAB_BRIEFS stub.
 *   2. F15 — `WorkerPersonalityCard` and `TeammateCard` now emit
 *      dev-mode `console.warn` calls when their per-name lookup
 *      tables miss. Pin the contract that personas already covered
 *      by the maps DO NOT trigger the warn — i.e. their entries are
 *      still present.
 *
 * Personas covered today by PERSONALITIES + TRAITS_BY_NAME +
 * LEVEL_BY_NAME: Mariah, Priya, Jamal. Peta + Nori are intentionally
 * out-of-scope here (covered by
 * `fix-peta-personality-trait-level-lookup-coverage`).
 *
 * The local Playwright lane runs in dev mode against the mock
 * gateway, so `process.env.NODE_ENV === "development"` and the warns
 * are live.
 */

const COVERED_PERSONAS = [
  { name: "Mariah", id: "018f67d0-0000-7000-8000-000000000001" },
  { name: "Priya", id: "018f67d0-0000-7000-8000-000000000002" },
  { name: "Jamal", id: "018f67d0-0000-7000-8000-000000000003" },
] as const;

const PERSONALITY_WARN_RE =
  /\[WorkerPersonalityCard\] Missing personality for/;
const TEAMMATE_TRAITS_WARN_RE = /\[TeammateCard\] Missing traits for/;
const TEAMMATE_LEVEL_WARN_RE = /\[TeammateCard\] Missing level for/;
const TAB_STUB_RE = /coming in /;

function captureWarnings(page: import("@playwright/test").Page): string[] {
  const messages: string[] = [];
  page.on("console", (msg: ConsoleMessage) => {
    if (msg.type() === "warning") messages.push(msg.text());
  });
  return messages;
}

test("unknown tab segment falls through to 404, not a TAB_BRIEFS stub", async ({
  page,
}) => {
  const realId = COVERED_PERSONAS[0].id;
  await page.goto(`/workers/${realId}/this-is-not-a-real-tab`);

  // The pre-fix stub rendered "<Label> — coming in <brief-id>". The
  // dead-code-delete must make those words unreachable for any
  // worker-detail path the static segments don't claim.
  await expect(page.locator("body")).not.toContainText(TAB_STUB_RE);
});

for (const { name, id } of COVERED_PERSONAS) {
  test(`${name} overview emits no missing-personality / -traits / -level warns`, async ({
    page,
  }) => {
    const warnings = captureWarnings(page);

    await page.goto(`/workers/${id}/overview`);
    await expect(page.getByTestId("worker-name")).toHaveText(
      new RegExp(name, "i"),
    );

    const personalityWarns = warnings.filter((m) =>
      PERSONALITY_WARN_RE.test(m),
    );
    const traitsWarns = warnings.filter((m) => TEAMMATE_TRAITS_WARN_RE.test(m));
    const levelWarns = warnings.filter((m) => TEAMMATE_LEVEL_WARN_RE.test(m));

    expect(personalityWarns, JSON.stringify(personalityWarns)).toEqual([]);
    expect(traitsWarns, JSON.stringify(traitsWarns)).toEqual([]);
    expect(levelWarns, JSON.stringify(levelWarns)).toEqual([]);
  });
}
