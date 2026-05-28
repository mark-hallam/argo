/**
 * F8 — Persisted onboarding draft must not flash empty fields on
 * hard reload.
 *
 * Before the fix: zustand-persist hydrates the draft asynchronously
 * after first paint. On hard reload of any wizard step, the input
 * mounts with the un-hydrated empty default ("") and re-renders ~1-2
 * frames later with the persisted value, producing a visible
 * empty-then-full flash on slow devices and screen recordings.
 *
 * After the fix: pages gate the form on
 * `useOnboardingDraft((s) => s.hasHydrated)`. While `false` the page
 * renders `<WizardSkeleton />`; once `onRehydrateStorage` flips the
 * flag the page swaps to the form, whose `useState` capture sees the
 * rehydrated value on first render. The wizard never paints a form
 * field with an empty value when persisted state exists.
 *
 * The spec seeds a draft into localStorage *before* navigation via
 * `page.addInitScript` so the page hard-loads with a populated
 * persisted blob. It then polls the workspace-name input every ~16ms
 * for 500ms and asserts no observed frame ever shows the input present
 * with an empty value. Skeleton-only frames (no input in the DOM) are
 * OK; populated frames are OK. The forbidden state is "input mounted,
 * value empty" — exactly the flash this brief closes.
 */

import { expect, test } from "@playwright/test";

const SPA_ORIGIN = new URL(
  process.env.ARGO_E2E_SPA_URL ?? process.env.E2E_SPA_URL ?? "http://localhost:3000",
).origin;

const STORAGE_KEY = "skycot:onboarding-draft-v1";

interface SeededDraft {
  step: number;
  workspaceName: string;
  companyName: string;
  providerKeyConfigured: boolean;
  providerSkipped: boolean;
  verticalId: string | null;
  verticalDisplayName: string | null;
  verticalSkillCount: number | null;
  firstHireName: string;
  firstHirePrompt: string;
  tenantId: string | null;
  workspaceId: string | null;
  selectedPlan: "free" | "pro" | null;
}

const SEEDED_DRAFT: SeededDraft = {
  step: 2,
  workspaceName: "Acme Growth",
  companyName: "Acme Inc.",
  providerKeyConfigured: false,
  providerSkipped: false,
  verticalId: null,
  verticalDisplayName: null,
  verticalSkillCount: null,
  firstHireName: "",
  firstHirePrompt: "",
  tenantId: null,
  workspaceId: null,
  selectedPlan: null,
};

test("workspace input never flashes empty on hard reload with a persisted draft", async ({
  page,
}) => {
  // Seed the persisted draft BEFORE the page's scripts run so the
  // zustand-persist middleware sees the same blob a returning user's
  // browser would on hard reload. The persist blob format matches the
  // `version: 1` envelope zustand writes.
  await page.addInitScript(
    ({ key, draft }) => {
      const envelope = { state: draft, version: 1 };
      window.localStorage.setItem(key, JSON.stringify(envelope));
    },
    { key: STORAGE_KEY, draft: SEEDED_DRAFT },
  );

  // Begin sampling the workspace-name input immediately so frames are
  // collected from the very first paint window. Each sample records
  // either {present: false} (input not in DOM — skeleton or nothing
  // yet) or {present: true, value}.
  await page.goto(appUrl("/onboarding/workspace"));

  type Frame = { t: number; present: boolean; value: string | null };
  const frames: Frame[] = await page.evaluate(
    async (): Promise<Frame[]> => {
      const samples: Frame[] = [];
      const start = performance.now();
      // Poll every ~16ms (~60fps) for 500ms.
      while (performance.now() - start < 500) {
        const el = document.querySelector<HTMLInputElement>(
          '[data-testid="workspace-name"]',
        );
        samples.push({
          t: performance.now() - start,
          present: el !== null,
          value: el?.value ?? null,
        });
        await new Promise((r) => setTimeout(r, 16));
      }
      return samples;
    },
  );

  // The forbidden state: the input mounted in the DOM with an empty
  // value. ANY such frame would be a visible flash to a recorded camera.
  const flashFrames = frames.filter((f) => f.present && f.value === "");
  expect(
    flashFrames,
    `Detected ${flashFrames.length} frames where the workspace input was present with an empty value. ` +
      `First few: ${JSON.stringify(flashFrames.slice(0, 3))}`,
  ).toHaveLength(0);

  // Sanity: by the end of the window the input must be present AND
  // populated with the seeded value. Without this assertion the test
  // would silently pass on a page that never rendered the input at
  // all.
  const finalInput = page.getByTestId("workspace-name");
  await expect(finalInput).toBeVisible();
  await expect(finalInput).toHaveValue("Acme Growth");
});

function appUrl(pathname: string): string {
  return new URL(pathname, SPA_ORIGIN).toString();
}
