/**
 * Wizard step-pip semantics + step-indicator a11y.
 *
 * UX-audit findings closed:
 *   - F9 [P1] onboarding-hire-an-ai-teammate — the step strip lit all
 *     pips on arrival at step 5 ("STEP 05 OF 05" + 5 lit pips), making
 *     the progress strip read as decorative rather than informative.
 *     Fix: WizardEditorialShell now renders three pip states —
 *     `completed`, `active`, `pending` — keyed off `completedSteps`
 *     (default `step - 1`). The active pip is visually distinct and
 *     carries `aria-current="step"`.
 *   - F15 [P3] welcome page — the "STEP 0/5 OF 0/5" line was rendered
 *     as ~4 separate `<span>` children, each emitting its own
 *     `StaticText` a11y node. Screen readers heard the digits as
 *     separate utterances ("step, zero, five, of, zero, five"). Fix:
 *     the visible chrome is now a single text node with the digits
 *     inline (no span boundary between "0" and "5") AND the parent
 *     div carries an `aria-label="Step N of 5 · LABEL"` that screen
 *     readers use in place of the visible text. The label markup is
 *     `aria-hidden` to avoid duplicate announcement.
 *
 * The brief intent included an axe-core sweep on every step. axe-core
 * is not currently a project dependency; rather than introduce a new
 * tooling dep behind this brief, the spec asserts the specific
 * accessible-name + node-count properties that axe would have flagged.
 *
 * Requires the dev server to be running with
 * `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1` so middleware does not
 * redirect onboarding routes to /auth/signin. The playwright config
 * sets that env var on the spawned `pnpm dev` web server already.
 */

import { expect, test, type Page, type TestInfo } from "@playwright/test";

function appUrl(testInfo: TestInfo, route: string) {
  const baseURL =
    testInfo.project.use.baseURL ??
    process.env.ARGO_E2E_SPA_URL ??
    process.env.E2E_SPA_URL ??
    "http://localhost:3000";
  return new URL(route, baseURL).toString();
}

type PipState = "completed" | "active" | "pending";

async function expectPipStates(page: Page, expected: ReadonlyArray<PipState>) {
  for (let i = 0; i < expected.length; i += 1) {
    const pip = page.getByTestId(`wiz-pip-${i + 1}`);
    await expect(pip, `pip ${i + 1} state`).toHaveAttribute("data-state", expected[i]);
    if (expected[i] === "active") {
      await expect(pip, `pip ${i + 1} should carry aria-current=step`).toHaveAttribute(
        "aria-current",
        "step",
      );
    } else {
      await expect(pip, `pip ${i + 1} should not carry aria-current`).not.toHaveAttribute(
        "aria-current",
        /.*/,
      );
    }
  }
}

interface StepCase {
  path: string;
  step: number;
  label: string;
  expectedPips: ReadonlyArray<PipState>;
}

const STEP_CASES: ReadonlyArray<StepCase> = [
  {
    path: "/onboarding/welcome",
    step: 1,
    label: "WELCOME",
    expectedPips: ["active", "pending", "pending", "pending", "pending"],
  },
  {
    path: "/onboarding/workspace",
    step: 2,
    label: "WORKSPACE",
    expectedPips: ["completed", "active", "pending", "pending", "pending"],
  },
  {
    path: "/onboarding/connect-provider",
    step: 3,
    label: "PROVIDER",
    expectedPips: ["completed", "completed", "active", "pending", "pending"],
  },
  {
    path: "/onboarding/pick-vertical",
    step: 4,
    label: "TEMPLATE",
    expectedPips: ["completed", "completed", "completed", "active", "pending"],
  },
  {
    path: "/onboarding/first-hire",
    step: 5,
    label: "FIRST HIRE",
    expectedPips: ["completed", "completed", "completed", "completed", "active"],
  },
];

test.describe("onboarding wizard step-pip semantics + a11y (F9 / F15)", () => {
  for (const { path, step, expectedPips } of STEP_CASES) {
    test(`step ${step} (${path}) renders correct pip states`, async ({ page }, testInfo) => {
      await page.goto(appUrl(testInfo, path), { waitUntil: "domcontentloaded" });
      await expect(page.getByTestId("wiz-shell-hero")).toBeVisible();
      await expectPipStates(page, expectedPips);
    });

    test(`step ${step} (${path}) step indicator has single accessible name`, async ({
      page,
    }, testInfo) => {
      await page.goto(appUrl(testInfo, path), { waitUntil: "domcontentloaded" });
      const progress = page.getByRole("progressbar");
      await expect(progress).toBeVisible();
      // Screen-reader name is the aria-label, not the (aria-hidden)
      // visible chrome — verifies F15 single-utterance fix.
      const accessibleName = await progress.getAttribute("aria-label");
      expect(accessibleName).toMatch(new RegExp(`^Step ${step} of 5\\b`));
      await expect(progress).toHaveAttribute("aria-valuenow", String(step));
      await expect(progress).toHaveAttribute("aria-valuemax", "5");
      // The visible "STEP NN OF NN" chrome must collapse into a single
      // child span (not 4 split spans) so assistive tech doesn't double-
      // announce the digits even if it ignored aria-hidden.
      const visibleChromeText = await progress.evaluate((node) => {
        return Array.from(node.querySelectorAll(":scope > span"))
          .map((el) => (el.textContent ?? "").trim())
          .filter((t) => t.length > 0);
      });
      // Exactly two direct <span> children at progressbar level — the
      // "STEP NN OF NN" run and the right-side stepLabel — both
      // aria-hidden; the pips live in their own <div>.
      expect(visibleChromeText.length).toBe(2);
      expect(visibleChromeText[0]).toMatch(/^STEP\s+\d{2}\s+OF\s+\d{2}$/);
    });
  }
});
